use anyhow::Result;
use nezha_staking::state::EpochStatus;
use nezha_staking_lib::state::LatestEpoch;
use nezha_staking_lib::{
    accounts as ac,
    fixed_point::{
        test_utils::{fp, usdc},
        FPInternal, FPUSDC,
    },
    instruction::{self, CreateEpochWinnersMetaArgs, TierWinnersMetaInput, WinnerInput, WithdrawVault},
    state::{Epoch, EpochWinnersMeta, EpochWinnersPage, InsuranceCfg, Stake, YieldSplitCfg},
};
use pretty_assertions::assert_eq;
use solana_program::pubkey::Pubkey;
use solana_program_test::tokio;
use solana_sdk::signer::Signer;
use spl_associated_token_account::get_associated_token_address;

use crate::{accounts::Accounts, actions::*, setup::*};

use nezha_testing::solana_test_runtime::SolanaTestRuntime;

async fn setup() -> Result<(Accounts, Box<dyn SolanaTestRuntime>)> {
    let accounts = Accounts::new();
    let processor = setup_test_runtime(&accounts).await?;
    Ok((accounts, processor))
}

async fn progress_epoch(
    deposit_amount: f64,
    yield_split_cfg: YieldSplitCfg,
    num_tickets_issued: u64,
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    create_epoch(&accounts, yield_split_cfg, processor).await?;

    if deposit_amount != 0.0 {
        request_stake_update(StakeUpdateOp::Deposit, fp(deposit_amount), &accounts, processor).await?;
        approve_stake_update(&accounts, processor, StakeUpdateOp::Deposit, fp(deposit_amount)).await?;
        complete_stake_update(&accounts, processor).await?;
    }

    yield_withdraw_by_investor(num_tickets_issued, &accounts, processor).await?;

    Ok(())
}

async fn end_epoch(
    winners: [Option<Pubkey>; 3],
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let latest_epoch: LatestEpoch = get_data(ac::latest_epoch(&accounts.program_id).pubkey, processor).await?;
    set_winning_combination(latest_epoch.index, [0u8; 6], accounts, processor).await?;

    let meta = CreateEpochWinnersMetaArgs {
        tier1_meta: TierWinnersMetaInput {
            total_num_winners: if winners[0].is_some() { 1 } else { 0 },
            total_num_winning_tickets: if winners[0].is_some() { 1 } else { 0 },
        },
        tier2_meta: TierWinnersMetaInput {
            total_num_winners: if winners[1].is_some() { 1 } else { 0 },
            total_num_winning_tickets: if winners[1].is_some() { 1 } else { 0 },
        },
        tier3_meta: TierWinnersMetaInput {
            total_num_winners: if winners[2].is_some() { 1 } else { 0 },
            total_num_winning_tickets: if winners[2].is_some() { 1 } else { 0 },
        },
    };
    let mut winners_input = Vec::new();
    for (i, wallet) in winners.iter().enumerate() {
        if let Some(wallet) = wallet {
            winners_input.push(WinnerInput {
                index: winners_input.len() as _,
                address: *wallet,
                tier: (i + 1) as _,
                num_winning_tickets: 1,
            });
        }
    }
    publish_epoch_winners(&meta, &winners_input, accounts, processor).await?;

    Ok(())
}

struct Assertions {
    return_amount: f64,
    cumulative_return_rate: f64,
    user_stake: f64,
    insurance: f64,
    tier2_prize: f64,
    tier3_prize: f64,
    treasury: f64,
    draw_enabled: bool,
}

struct VaultBalances {
    insurance: FPUSDC,
    treasury: FPUSDC,
    tier2_prize: FPUSDC,
    tier3_prize: FPUSDC,
}

impl VaultBalances {
    async fn acquire(accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<Self> {
        let insurance = get_usdc_balance_by_account(&ac::insurance_vault(&accounts.program_id), processor).await?;
        let treasury = get_usdc_balance_by_account(&ac::treasury_vault(&accounts.program_id), processor).await?;
        let tier2_prize = get_usdc_balance_by_account(&ac::prize_vault(&accounts.program_id, 2), processor).await?;
        let tier3_prize = get_usdc_balance_by_account(&ac::prize_vault(&accounts.program_id, 3), processor).await?;
        Ok(Self {
            insurance,
            treasury,
            tier2_prize,
            tier3_prize,
        })
    }
}

impl Assertions {
    async fn assert(
        &self,
        prev_vault_balances: &VaultBalances,
        accounts: &Accounts,
        processor: &mut dyn SolanaTestRuntime,
    ) -> Result<()> {
        let latest_epoch = get_latest_epoch(accounts, processor).await?;
        assert_eq!(*latest_epoch.cumulative_return_rate, fp(self.cumulative_return_rate));

        let epoch: Epoch = get_data(latest_epoch.epoch, processor).await?;
        assert_eq!(epoch.returns.as_ref().unwrap().total, fp(self.return_amount));

        let user_stake: Option<Stake> =
            get_optional_data(*ac::stake(&accounts.program_id, &accounts.owner.pubkey()), processor).await?;
        let user_stake_balance: FPInternal = if let Some(user_stake) = user_stake {
            user_stake
                .balance
                .get_amount(latest_epoch.cumulative_return_rate)
                .unwrap()
        } else {
            fp(0.0)
        };
        assert_eq!(user_stake_balance, fp(self.user_stake));

        let yield_split = epoch.returns.unwrap();
        assert_eq!(yield_split.insurance, fp(self.insurance));
        assert_eq!(yield_split.tier2_prize, fp(self.tier2_prize));
        assert_eq!(yield_split.tier3_prize, fp(self.tier3_prize));
        assert_eq!(yield_split.treasury, fp(self.treasury));

        assert_eq!(epoch.draw_enabled.unwrap(), self.draw_enabled);

        let current_vault_balances = VaultBalances::acquire(accounts, processor).await?;
        assert_eq!(
            current_vault_balances
                .insurance
                .checked_sub(prev_vault_balances.insurance)
                .unwrap(),
            fp(self.insurance)
        );

        assert_eq!(
            current_vault_balances
                .treasury
                .checked_sub(prev_vault_balances.treasury)
                .unwrap(),
            fp(self.treasury)
        );

        assert_eq!(
            current_vault_balances
                .tier2_prize
                .checked_sub(prev_vault_balances.tier2_prize)
                .unwrap(),
            fp(self.tier2_prize)
        );

        assert_eq!(
            current_vault_balances
                .tier3_prize
                .checked_sub(prev_vault_balances.tier3_prize)
                .unwrap(),
            fp(self.tier3_prize)
        );

        Ok(())
    }
}

#[tokio::test]
async fn returns_2x() -> Result<()> {
    let deposit_amount = 100.0;
    let insurance_premium = 2.0;
    let insurance_probability = 0.0001;
    let jackpot = 100_000.0;
    let treasury_ratio = 0.5;
    let tier2_share = 3;
    let tier3_share = 1;
    let num_tickets_issued = 1;

    let yield_split_cfg = YieldSplitCfg {
        insurance: InsuranceCfg {
            premium: fp(insurance_premium),
            probability: fp(insurance_probability),
        },
        jackpot: fp(jackpot),
        treasury_ratio: fp(treasury_ratio),
        tier2_prize_share: tier2_share,
        tier3_prize_share: tier3_share,
    };

    let (accounts, mut processor) = setup().await?;

    let vault_balances = VaultBalances::acquire(&accounts, processor.as_mut()).await?;

    progress_epoch(
        deposit_amount,
        yield_split_cfg,
        num_tickets_issued,
        &accounts,
        processor.as_mut(),
    )
    .await?;

    yield_deposit_by_investor(fp(200.0), &accounts, processor.as_mut()).await?;

    let assertions = Assertions {
        return_amount: 200.0,
        cumulative_return_rate: 1.0,
        user_stake: 100.0,
        insurance: 20.0,
        treasury: 40.0,
        tier2_prize: 30.0,
        tier3_prize: 10.0,
        draw_enabled: true,
    };

    assertions
        .assert(&vault_balances, &accounts, processor.as_mut())
        .await?;

    Ok(())
}

#[tokio::test]
async fn just_cover_insurance() -> Result<()> {
    let deposit_amount = 100.0;
    let insurance_premium = 2.0;
    let insurance_probability = 0.0001;
    let jackpot = 100_000.0;
    let treasury_ratio = 0.5;
    let tier2_share = 3;
    let tier3_share = 1;
    let num_tickets_issued = 1;

    let yield_split_cfg = YieldSplitCfg {
        insurance: InsuranceCfg {
            premium: fp(insurance_premium),
            probability: fp(insurance_probability),
        },
        jackpot: fp(jackpot),
        treasury_ratio: fp(treasury_ratio),
        tier2_prize_share: tier2_share,
        tier3_prize_share: tier3_share,
    };

    let (accounts, mut processor) = setup().await?;

    let vault_balances = VaultBalances::acquire(&accounts, processor.as_mut()).await?;
    progress_epoch(
        deposit_amount,
        yield_split_cfg,
        num_tickets_issued,
        &accounts,
        processor.as_mut(),
    )
    .await?;

    yield_deposit_by_investor(fp(120.0), &accounts, processor.as_mut()).await?;

    let assertions = Assertions {
        return_amount: 120.0,
        cumulative_return_rate: 1.0,
        user_stake: 100.0,
        insurance: 20.0,
        treasury: 0.0,
        tier2_prize: 0.0,
        tier3_prize: 0.0,
        draw_enabled: true,
    };

    assertions
        .assert(&vault_balances, &accounts, processor.as_mut())
        .await?;

    Ok(())
}

#[tokio::test]
async fn carryover_covers_insurance() -> Result<()> {
    let deposit_amount = 100.0;
    let insurance_premium = 2.0;
    let insurance_probability = 0.0001;
    let jackpot = 100_000.0;
    let treasury_ratio = 0.5;
    let tier2_share = 3;
    let tier3_share = 1;
    let num_tickets_issued = 1;

    let yield_split_cfg = YieldSplitCfg {
        insurance: InsuranceCfg {
            premium: fp(insurance_premium),
            probability: fp(insurance_probability),
        },
        jackpot: fp(jackpot),
        treasury_ratio: fp(treasury_ratio),
        tier2_prize_share: tier2_share,
        tier3_prize_share: tier3_share,
    };

    let (accounts, mut processor) = setup().await?;

    let vault_balances = VaultBalances::acquire(&accounts, processor.as_mut()).await?;
    progress_epoch(
        deposit_amount,
        yield_split_cfg.clone(),
        num_tickets_issued,
        &accounts,
        processor.as_mut(),
    )
    .await?;

    yield_deposit_by_investor(fp(110.0), &accounts, processor.as_mut()).await?;

    let assertions = Assertions {
        return_amount: 110.0,
        cumulative_return_rate: 1.0,
        user_stake: 100.0,
        insurance: 10.0,
        treasury: 0.0,
        tier2_prize: 0.0,
        tier3_prize: 0.0,
        draw_enabled: false,
    };

    assertions
        .assert(&vault_balances, &accounts, processor.as_mut())
        .await?;

    let latest_epoch = get_latest_epoch(&accounts, processor.as_mut()).await?;
    let epoch: Epoch = get_data(latest_epoch.epoch, processor.as_mut()).await?;
    assert_eq!(latest_epoch.status, epoch.status);
    assert_eq!(latest_epoch.status, EpochStatus::Ended);

    let vault_balances = VaultBalances::acquire(&accounts, processor.as_mut()).await?;
    progress_epoch(0.0, yield_split_cfg, num_tickets_issued, &accounts, processor.as_mut()).await?;

    yield_deposit_by_investor(fp(110.0), &accounts, processor.as_mut()).await?;

    let assertions = Assertions {
        return_amount: 110.0,
        cumulative_return_rate: 1.0,
        user_stake: 100.0,
        insurance: 10.0,
        treasury: 0.0,
        tier2_prize: 0.0,
        tier3_prize: 0.0,
        draw_enabled: true,
    };

    assertions
        .assert(&vault_balances, &accounts, processor.as_mut())
        .await?;

    Ok(())
}

#[tokio::test]
async fn loss() -> Result<()> {
    let deposit_amount = 100.0;
    let insurance_premium = 2.0;
    let insurance_probability = 0.0001;
    let jackpot = 100_000.0;
    let treasury_ratio = 0.5;
    let tier2_share = 3;
    let tier3_share = 1;
    let num_tickets_issued = 1;

    let yield_split_cfg = YieldSplitCfg {
        insurance: InsuranceCfg {
            premium: fp(insurance_premium),
            probability: fp(insurance_probability),
        },
        jackpot: fp(jackpot),
        treasury_ratio: fp(treasury_ratio),
        tier2_prize_share: tier2_share,
        tier3_prize_share: tier3_share,
    };

    let (accounts, mut processor) = setup().await?;

    let vault_balances = VaultBalances::acquire(&accounts, processor.as_mut()).await?;
    progress_epoch(
        deposit_amount,
        yield_split_cfg.clone(),
        num_tickets_issued,
        &accounts,
        processor.as_mut(),
    )
    .await?;

    yield_deposit_by_investor(fp(90.0), &accounts, processor.as_mut()).await?;

    let assertions = Assertions {
        return_amount: 90.0,
        cumulative_return_rate: 0.9,
        user_stake: 90.0,
        insurance: 0.0,
        treasury: 0.0,
        tier2_prize: 0.0,
        tier3_prize: 0.0,
        draw_enabled: false,
    };

    assertions
        .assert(&vault_balances, &accounts, processor.as_mut())
        .await?;

    let latest_epoch = get_latest_epoch(&accounts, processor.as_mut()).await?;
    let epoch: Epoch = get_data(latest_epoch.epoch, processor.as_mut()).await?;
    assert_eq!(latest_epoch.status, epoch.status);
    assert_eq!(latest_epoch.status, EpochStatus::Ended);

    let vault_balances = VaultBalances::acquire(&accounts, processor.as_mut()).await?;
    progress_epoch(0.0, yield_split_cfg, num_tickets_issued, &accounts, processor.as_mut()).await?;

    yield_deposit_by_investor(fp(45.0), &accounts, processor.as_mut()).await?;

    let assertions = Assertions {
        return_amount: 45.0,
        cumulative_return_rate: 0.45,
        user_stake: 45.0,
        insurance: 0.0,
        treasury: 0.0,
        tier2_prize: 0.0,
        tier3_prize: 0.0,
        draw_enabled: false,
    };

    assertions
        .assert(&vault_balances, &accounts, processor.as_mut())
        .await?;

    Ok(())
}

#[tokio::test]
async fn no_deposit() -> Result<()> {
    let deposit_amount = 0.0;
    let insurance_premium = 2.0;
    let insurance_probability = 0.0001;
    let jackpot = 100_000.0;
    let treasury_ratio = 0.5;
    let tier2_share = 3;
    let tier3_share = 1;
    let num_tickets_issued = 0;

    let yield_split_cfg = YieldSplitCfg {
        insurance: InsuranceCfg {
            premium: fp(insurance_premium),
            probability: fp(insurance_probability),
        },
        jackpot: fp(jackpot),
        treasury_ratio: fp(treasury_ratio),
        tier2_prize_share: tier2_share,
        tier3_prize_share: tier3_share,
    };

    let (accounts, mut processor) = setup().await?;

    let vault_balances = VaultBalances::acquire(&accounts, processor.as_mut()).await?;
    progress_epoch(
        deposit_amount,
        yield_split_cfg,
        num_tickets_issued,
        &accounts,
        processor.as_mut(),
    )
    .await?;

    let res = yield_deposit_by_investor(fp(1.0), &accounts, processor.as_mut()).await;
    assert!(res.is_err(), "Shouldn't allow returning yield without any deposits.");

    yield_deposit_by_investor(fp(0.0), &accounts, processor.as_mut()).await?;

    let assertions = Assertions {
        return_amount: 0.0,
        cumulative_return_rate: 1.0,
        user_stake: 0.0,
        insurance: 0.0,
        treasury: 0.0,
        tier2_prize: 0.0,
        tier3_prize: 0.0,
        draw_enabled: true,
    };

    assertions
        .assert(&vault_balances, &accounts, processor.as_mut())
        .await?;

    Ok(())
}

#[tokio::test]
async fn prize_carry_forward() -> Result<()> {
    let num_tickets_issued = 0;

    let yield_split_cfg = YieldSplitCfg {
        insurance: InsuranceCfg {
            premium: fp(2.0),
            probability: fp(0.0001),
        },
        jackpot: fp(1.0),
        treasury_ratio: fp(0.0),
        tier2_prize_share: 3,
        tier3_prize_share: 1,
    };

    let (accounts, mut processor) = setup().await?;

    progress_epoch(
        100.0,
        yield_split_cfg.clone(),
        num_tickets_issued,
        &accounts,
        processor.as_mut(),
    )
    .await?;

    yield_deposit_by_investor(fp(200.0), &accounts, processor.as_mut()).await?;

    // Pending prizes calculated correctly
    let latest_epoch = get_latest_epoch(&accounts, processor.as_mut()).await?;
    assert_eq!(latest_epoch.pending_funds.tier2_prize, fp(75.0));
    assert_eq!(latest_epoch.pending_funds.tier3_prize, fp(25.0));

    let winners = [None, None, None];
    end_epoch(winners, &accounts, processor.as_mut()).await?;

    progress_epoch(
        0.0,
        yield_split_cfg.clone(),
        num_tickets_issued,
        &accounts,
        processor.as_mut(),
    )
    .await?;

    yield_deposit_by_investor(fp(200.0), &accounts, processor.as_mut()).await?;

    // Added to pending prize
    let latest_epoch = get_latest_epoch(&accounts, processor.as_mut()).await?;
    assert_eq!(latest_epoch.pending_funds.tier2_prize, fp(150.0));
    assert_eq!(latest_epoch.pending_funds.tier3_prize, fp(50.0));

    let winners = [None, Some(Pubkey::new_unique()), Some(Pubkey::new_unique())];
    end_epoch(winners, &accounts, processor.as_mut()).await?;

    // Pending prizes get emptied
    let latest_epoch = get_latest_epoch(&accounts, processor.as_mut()).await?;
    assert_eq!(latest_epoch.pending_funds.tier2_prize, fp(0.0));
    assert_eq!(latest_epoch.pending_funds.tier3_prize, fp(0.0));

    let epoch_winners_meta: EpochWinnersMeta = get_data(
        *ac::epoch_winners_meta(&accounts.program_id, latest_epoch.index),
        processor.as_mut(),
    )
    .await?;

    assert_eq!(epoch_winners_meta.tier2_meta.total_prize, fp(150.0));
    assert_eq!(epoch_winners_meta.tier3_meta.total_prize, fp(50.0));

    progress_epoch(
        0.0,
        yield_split_cfg.clone(),
        num_tickets_issued,
        &accounts,
        processor.as_mut(),
    )
    .await?;
    yield_deposit_by_investor(fp(110.0), &accounts, processor.as_mut()).await?;

    let winners = [None, Some(Pubkey::new_unique()), Some(Pubkey::new_unique())];
    end_epoch(winners, &accounts, processor.as_mut()).await?;

    let latest_epoch = get_latest_epoch(&accounts, processor.as_mut()).await?;
    let epoch_winners_meta: EpochWinnersMeta = get_data(
        *ac::epoch_winners_meta(&accounts.program_id, latest_epoch.index),
        processor.as_mut(),
    )
    .await?;

    // Pending prizes did get emptied
    assert_eq!(epoch_winners_meta.tier2_meta.total_prize, fp(7.5));
    assert_eq!(epoch_winners_meta.tier3_meta.total_prize, fp(2.5));

    Ok(())
}

#[tokio::test]
async fn fund_jackpot_claim_winning() -> Result<()> {
    let num_tickets_issued = 0;

    let yield_split_cfg = YieldSplitCfg {
        insurance: InsuranceCfg {
            premium: fp(2.0),
            probability: fp(0.0005),
        },
        jackpot: fp(1000.0),
        treasury_ratio: fp(0.0),
        tier2_prize_share: 3,
        tier3_prize_share: 1,
    };

    let (accounts, mut processor) = setup().await?;

    progress_epoch(
        100.0,
        yield_split_cfg.clone(),
        num_tickets_issued,
        &accounts,
        processor.as_mut(),
    )
    .await?;

    yield_deposit_by_investor(fp(100.0), &accounts, processor.as_mut()).await?;

    let winners = [Some(accounts.owner.pubkey()), None, None];
    end_epoch(winners, &accounts, processor.as_mut()).await?;

    assert_eq!(fp(100.0), get_owner_stake_balance(&accounts, processor.as_mut()).await?);

    let epoch_index = 1;
    let page = 0;
    let winner_index = 0;
    let tier = 1;

    // Claim doesn't work before funding
    let res = claim_winning(epoch_index, page, winner_index, tier, &accounts, processor.as_mut()).await;
    assert!(res.is_err());

    assert_eq!(fp(100.0), get_owner_stake_balance(&accounts, processor.as_mut()).await?);

    fund_jackpot(&accounts, processor.as_mut()).await?;

    // Can't fund twice
    let res = fund_jackpot(&accounts, processor.as_mut()).await;
    assert!(res.is_err());

    // Claim works after funding
    claim_winning(epoch_index, page, winner_index, tier, &accounts, processor.as_mut()).await?;

    // Claim moved funds from prize vault into deposit vault
    assert_eq!(
        fp(1100.0),
        get_owner_stake_balance(&accounts, processor.as_mut()).await?
    );
    assert_eq!(
        fp(1100.0),
        get_usdc_balance_by_account(&ac::deposit_vault(&accounts.program_id), processor.as_mut()).await?
    );

    Ok(())
}

#[tokio::test]
async fn claim_winning_tier2_tier3() -> Result<()> {
    let num_tickets_issued = 0;

    let yield_split_cfg = YieldSplitCfg {
        insurance: InsuranceCfg {
            premium: fp(2.0),
            probability: fp(0.0001),
        },
        jackpot: fp(1.0),
        treasury_ratio: fp(0.0),
        tier2_prize_share: 3,
        tier3_prize_share: 1,
    };

    let (accounts, mut processor) = setup().await?;

    progress_epoch(
        100.0,
        yield_split_cfg.clone(),
        num_tickets_issued,
        &accounts,
        processor.as_mut(),
    )
    .await?;
    yield_deposit_by_investor(fp(200.0), &accounts, processor.as_mut()).await?;

    let winners = [Some(accounts.owner.pubkey()); 3];
    end_epoch(winners, &accounts, processor.as_mut()).await?;

    assert_eq!(fp(100.0), get_owner_stake_balance(&accounts, processor.as_mut()).await?);

    let epoch_index = 1;
    let page = 0;

    let epoch_winners_page: EpochWinnersPage = get_data(
        *ac::epoch_winners_page(&accounts.program_id, epoch_index, page),
        processor.as_mut(),
    )
    .await?;

    for winner in &epoch_winners_page.winners {
        if winner.tier == 2 {
            claim_winning(
                epoch_index,
                page,
                winner.index,
                winner.tier,
                &accounts,
                processor.as_mut(),
            )
            .await?;
        }
    }

    // Claim moved funds from prize vault into deposit vault
    assert_eq!(fp(175.0), get_owner_stake_balance(&accounts, processor.as_mut()).await?);
    assert_eq!(
        fp(175.0),
        get_usdc_balance_by_account(&ac::deposit_vault(&accounts.program_id), processor.as_mut()).await?
    );

    for winner in &epoch_winners_page.winners {
        if winner.tier == 3 {
            claim_winning(
                epoch_index,
                page,
                winner.index,
                winner.tier,
                &accounts,
                processor.as_mut(),
            )
            .await?;
        }
    }

    // Two claims in a single epoch works
    assert_eq!(fp(200.0), get_owner_stake_balance(&accounts, processor.as_mut()).await?);
    assert_eq!(
        fp(200.0),
        get_usdc_balance_by_account(&ac::deposit_vault(&accounts.program_id), processor.as_mut()).await?
    );

    Ok(())
}

#[tokio::test]
async fn test_withdraw_vault() -> Result<()> {
    let num_tickets_issued = 1;
    let yield_split_cfg = YieldSplitCfg {
        insurance: InsuranceCfg {
            premium: fp(2.0),
            probability: fp(0.0005),
        },
        jackpot: fp(100000.0),
        treasury_ratio: fp(1.0),
        tier2_prize_share: 1,
        tier3_prize_share: 1,
    };

    let (accounts, mut processor) = setup().await?;

    progress_epoch(
        100.0,
        yield_split_cfg.clone(),
        num_tickets_issued,
        &accounts,
        processor.as_mut(),
    )
    .await?;
    yield_deposit_by_investor(fp(250.0), &accounts, processor.as_mut()).await?;

    let winners = [None; 3];
    end_epoch(winners, &accounts, processor.as_mut()).await?;

    let destination_owner = Pubkey::new_unique();
    let destination_ata = get_associated_token_address(&destination_owner, &accounts.usdc_mint.pubkey());
    create_token_account(&destination_owner, &accounts.usdc_mint.pubkey(), processor.as_mut()).await?;

    assert_eq!(
        fp(100.0),
        get_usdc_balance_by_account(&ac::insurance_vault(&accounts.program_id), processor.as_mut()).await?
    );

    withdraw_vault(
        WithdrawVault::Insurance,
        &destination_ata,
        fp(10.0),
        &accounts,
        processor.as_mut(),
    )
    .await?;

    assert_eq!(
        fp(90.0),
        get_usdc_balance_by_account(&ac::insurance_vault(&accounts.program_id), processor.as_mut()).await?
    );

    assert_eq!(
        fp(10.0),
        get_usdc_balance_by_account(&destination_ata, processor.as_mut()).await?
    );

    assert_eq!(
        fp(50.0),
        get_usdc_balance_by_account(&ac::treasury_vault(&accounts.program_id), processor.as_mut()).await?
    );

    withdraw_vault(
        WithdrawVault::Treasury,
        &destination_ata,
        fp(20.0),
        &accounts,
        processor.as_mut(),
    )
    .await?;

    assert_eq!(
        fp(30.0),
        get_usdc_balance_by_account(&ac::treasury_vault(&accounts.program_id), processor.as_mut()).await?
    );

    assert_eq!(
        fp(30.0),
        get_usdc_balance_by_account(&destination_ata, processor.as_mut()).await?
    );

    Ok(())
}

#[tokio::test]
async fn incorrect_epoch_index() -> Result<()> {
    let num_tickets_issued = 1;
    let yield_split_cfg = YieldSplitCfg {
        insurance: InsuranceCfg {
            premium: fp(2.0),
            probability: fp(0.0005),
        },
        jackpot: fp(100000.0),
        treasury_ratio: fp(1.0),
        tier2_prize_share: 1,
        tier3_prize_share: 1,
    };

    let (accounts, mut processor) = setup().await?;

    let deposit_amount = 100.0;
    create_epoch(&accounts, yield_split_cfg, processor.as_mut()).await?;

    if deposit_amount != 0.0 {
        request_stake_update(
            StakeUpdateOp::Deposit,
            fp(deposit_amount),
            &accounts,
            processor.as_mut(),
        )
        .await?;
        approve_stake_update(
            &accounts,
            processor.as_mut(),
            StakeUpdateOp::Deposit,
            fp(deposit_amount),
        )
        .await?;
        complete_stake_update(&accounts, processor.as_mut()).await?;
    }

    let investor_usdc = get_associated_token_address(&accounts.investor.pubkey(), &accounts.usdc_mint.pubkey());

    let res = processor
        .send_ixns(
            &[instruction::yield_withdraw_by_investor(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                &investor_usdc,
                2,
                random_tickets_info(num_tickets_issued),
            )],
            &[&accounts.admin],
        )
        .await;
    assert!(res.is_err());

    processor
        .send_ixns(
            &[instruction::yield_withdraw_by_investor(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                &investor_usdc,
                1,
                random_tickets_info(num_tickets_issued),
            )],
            &[&accounts.admin],
        )
        .await?;

    let res = processor
        .send_ixns(
            &[instruction::yield_deposit_by_investor(
                &accounts.program_id,
                &accounts.investor.pubkey(),
                &investor_usdc,
                2,
                usdc("100.0").as_usdc(),
            )],
            &[&accounts.investor],
        )
        .await;
    assert!(res.is_err());

    processor
        .send_ixns(
            &[instruction::yield_deposit_by_investor(
                &accounts.program_id,
                &accounts.investor.pubkey(),
                &investor_usdc,
                1,
                usdc("100.0").as_usdc(),
            )],
            &[&accounts.investor],
        )
        .await?;

    Ok(())
}
