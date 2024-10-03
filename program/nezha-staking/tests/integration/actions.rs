#![allow(dead_code)]

use borsh::BorshSerialize;
use nezha_staking_lib::fixed_point::test_utils::fp;
use nezha_staking_lib::instruction::WithdrawVault;
use nezha_staking_lib::state::HasAccountType;
use nezha_staking_lib::state::InsuranceCfg;
use nezha_staking_lib::state::TicketsInfo;
use nezha_testing::solana_test_runtime::Account;
use nezha_vrf_lib::state::NezhaVrfRequest;
use nezha_vrf_lib::state::NezhaVrfRequestStatus;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::accounts::Accounts;
use anyhow::Context;
use anyhow::Result;
use borsh::BorshDeserialize;
use nezha_staking_lib::fixed_point::FPUSDC;
use nezha_staking_lib::{
    accounts as ac,
    instruction::{self, CreateEpochWinnersMetaArgs, WinnerInput},
    state::{LatestEpoch, Stake, YieldSplitCfg, MAX_NUM_WINNERS_PER_PAGE},
};
use solana_program::borsh0_10::try_from_slice_unchecked;
use solana_program::{program_pack::Pack, pubkey::Pubkey, system_instruction};
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_associated_token_account::get_associated_token_address;
use spl_associated_token_account::instruction::create_associated_token_account;

use nezha_testing::solana_test_runtime::SolanaTestRuntime;

pub async fn create_mint(
    mint: &Keypair,
    manager: &Pubkey,
    freeze_authority: Option<&Pubkey>,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let rent = processor.get_rent().await.unwrap();
    let payer = &processor.get_payer().pubkey();

    processor
        .send_ixns(
            &[
                system_instruction::create_account(
                    payer,
                    &mint.pubkey(),
                    rent.minimum_balance(spl_token::state::Mint::LEN),
                    spl_token::state::Mint::LEN as u64,
                    &spl_token::id(),
                ),
                spl_token::instruction::initialize_mint(
                    &spl_token::id(),
                    &mint.pubkey(),
                    &manager,
                    freeze_authority,
                    6,
                )?,
            ],
            &[&mint],
        )
        .await
}

pub async fn init(accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<()> {
    processor
        .send_ixns(
            &[instruction::init(
                &accounts.program_id,
                &accounts.super_admin.pubkey(),
                &accounts.admin.pubkey(),
                &accounts.investor.pubkey(),
                &accounts.usdc_mint.pubkey(),
                &accounts.nezha_vrf_program_id,
            )],
            &[&accounts.super_admin],
        )
        .await
}

pub fn random_yield_split_cfg() -> YieldSplitCfg {
    YieldSplitCfg {
        jackpot: fp("100_000.0"),
        insurance: InsuranceCfg {
            premium: fp("2.0"),
            probability: fp("0.0005"),
        },
        treasury_ratio: fp("0.4"),
        tier2_prize_share: 3,
        tier3_prize_share: 2,
    }
}

pub fn random_tickets_info(num_tickets: u64) -> TicketsInfo {
    TicketsInfo {
        num_tickets,
        tickets_url: String::from("https://nezha-tickets.com/asdfg"),
        tickets_hash: vec![0, 1, 2],
        tickets_version: 0,
    }
}

pub async fn create_epoch(
    accounts: &Accounts,
    yield_split_cfg: YieldSplitCfg,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let latest_epoch: LatestEpoch = get_latest_epoch(accounts, processor).await?;
    let index = latest_epoch.index + 1;
    let expected_end_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64
        + 60 * 60 * 24;

    processor
        .send_ixns(
            &[instruction::create_epoch(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                index,
                expected_end_at,
                yield_split_cfg,
            )],
            &[&accounts.admin],
        )
        .await
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum StakeUpdateOp {
    Deposit,
    Withdraw,
}

pub async fn request_stake_update(
    op: StakeUpdateOp,
    amount: FPUSDC,
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let owner_usdc = get_associated_token_address(&accounts.owner.pubkey(), &accounts.usdc_mint.pubkey());
    processor
        .send_ixns(
            &[instruction::request_stake_update(
                &accounts.program_id,
                &accounts.owner.pubkey(),
                &owner_usdc,
                match op {
                    StakeUpdateOp::Deposit => amount.as_usdc_i64(),
                    StakeUpdateOp::Withdraw => -amount.as_usdc_i64(),
                },
            )],
            &[&accounts.owner],
        )
        .await
}

pub async fn approve_stake_update(
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
    op: StakeUpdateOp,
    amount: FPUSDC,
) -> Result<()> {
    processor
        .send_ixns(
            &[instruction::approve_stake_update(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                &accounts.owner.pubkey(),
                match op {
                    StakeUpdateOp::Deposit => amount.as_usdc_i64(),
                    StakeUpdateOp::Withdraw => -amount.as_usdc_i64(),
                },
            )],
            &[&accounts.admin],
        )
        .await
}

pub async fn complete_stake_update(accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<()> {
    let owner_usdc = get_associated_token_address(&accounts.owner.pubkey(), &accounts.usdc_mint.pubkey());
    processor
        .send_ixns(
            &[instruction::complete_stake_update(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                &accounts.owner.pubkey(),
                &owner_usdc,
            )],
            &[&accounts.admin],
        )
        .await
}

pub async fn cancel_stake_update(
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
    op: StakeUpdateOp,
    amount: FPUSDC,
) -> Result<()> {
    let owner_usdc = get_associated_token_address(&accounts.owner.pubkey(), &accounts.usdc_mint.pubkey());
    processor
        .send_ixns(
            &[instruction::cancel_stake_update(
                &accounts.program_id,
                None,
                &accounts.owner.pubkey(),
                &owner_usdc,
                match op {
                    StakeUpdateOp::Deposit => amount.as_usdc_i64(),
                    StakeUpdateOp::Withdraw => -amount.as_usdc_i64(),
                },
            )],
            &[&accounts.owner],
        )
        .await
}

pub async fn cancel_stake_update_by_admin(
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
    op: StakeUpdateOp,
    amount: FPUSDC,
) -> Result<()> {
    let owner_usdc = get_associated_token_address(&accounts.owner.pubkey(), &accounts.usdc_mint.pubkey());
    processor
        .send_ixns(
            &[instruction::cancel_stake_update(
                &accounts.program_id,
                Some(&accounts.admin.pubkey()),
                &accounts.owner.pubkey(),
                &owner_usdc,
                match op {
                    StakeUpdateOp::Deposit => amount.as_usdc_i64(),
                    StakeUpdateOp::Withdraw => -amount.as_usdc_i64(),
                },
            )],
            &[&accounts.admin],
        )
        .await
}

pub async fn yield_withdraw_by_investor(
    num_tickets: u64,
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let investor_usdc = get_associated_token_address(&accounts.investor.pubkey(), &accounts.usdc_mint.pubkey());
    let epoch_index = get_latest_epoch(accounts, processor).await?.index;
    processor
        .send_ixns(
            &[instruction::yield_withdraw_by_investor(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                &investor_usdc,
                epoch_index,
                random_tickets_info(num_tickets),
            )],
            &[&accounts.admin],
        )
        .await
}

pub async fn yield_deposit_by_investor(
    amount: FPUSDC,
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let investor_usdc = get_associated_token_address(&accounts.investor.pubkey(), &accounts.usdc_mint.pubkey());
    let epoch_index = get_latest_epoch(accounts, processor).await?.index;

    processor
        .send_ixns(
            &[instruction::yield_deposit_by_investor(
                &accounts.program_id,
                &accounts.investor.pubkey(),
                &investor_usdc,
                epoch_index,
                amount.as_usdc(),
            )],
            &[&accounts.investor],
        )
        .await
}

pub async fn set_winning_combination(
    epoch_index: u64,
    winning_combination: [u8; 6],
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let account_pubkey = nezha_vrf_lib::accounts::nezha_vrf_request(&accounts.nezha_vrf_program_id, epoch_index).pubkey;
    let vrf_request = NezhaVrfRequest {
        account_type: nezha_vrf_lib::accounts::AccountType::NezhaVrfRequest,
        contract_version: nezha_vrf_lib::state::ContractVersion::V1,
        vrf_counter: 0,
        status: NezhaVrfRequestStatus::Success,
        winning_combination: Some(winning_combination),
        request_start: 0,
        request_end: Some(0),
    };
    let mut v = Vec::new();
    vrf_request.serialize(&mut v)?;
    processor.set_account(
        &account_pubkey,
        &Account {
            lamports: 1,
            owner: accounts.nezha_vrf_program_id,
            data: v,
        },
    );
    Ok(())
}

pub async fn create_epoch_winners_meta(
    meta_args: &CreateEpochWinnersMetaArgs,
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let epoch_index = get_latest_epoch(accounts, processor).await?.index;
    processor
        .send_ixns(
            &[instruction::create_epoch_winners_meta(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                epoch_index,
                meta_args.clone(),
                &accounts.nezha_vrf_program_id,
            )],
            &[&accounts.admin],
        )
        .await
}

pub async fn publish_epoch_winners_page(
    page_index: u32,
    winners_input: &[WinnerInput],
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let epoch_index = get_latest_epoch(accounts, processor).await?.index;
    processor
        .send_ixns(
            &[instruction::publish_winners(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                epoch_index,
                page_index,
                winners_input.to_vec(),
                &accounts.nezha_vrf_program_id,
            )],
            &[&accounts.admin],
        )
        .await
}

pub async fn publish_epoch_winners(
    meta_args: &CreateEpochWinnersMetaArgs,
    winners_input: &[WinnerInput],
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let epoch_index = get_latest_epoch(accounts, processor).await?.index;
    create_epoch_winners_meta(meta_args, accounts, processor).await?;

    for (page_index, chunk) in winners_input.chunks(MAX_NUM_WINNERS_PER_PAGE).enumerate() {
        processor
            .send_ixns(
                &[instruction::publish_winners(
                    &accounts.program_id,
                    &accounts.admin.pubkey(),
                    epoch_index,
                    page_index as u32,
                    chunk.to_vec(),
                    &accounts.nezha_vrf_program_id,
                )],
                &[&accounts.admin],
            )
            .await?;
    }
    Ok(())
}

pub async fn claim_winning(
    epoch_index: u64,
    page: u32,
    winner_index: u32,
    tier: u8,
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    processor
        .send_ixns(
            &[instruction::claim_winning(
                &accounts.program_id,
                &accounts.owner.pubkey(),
                epoch_index,
                page,
                winner_index,
                tier,
            )],
            &[],
        )
        .await
}

pub async fn fund_jackpot(accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<()> {
    let admin_usdc = get_associated_token_address(&accounts.admin.pubkey(), &accounts.usdc_mint.pubkey());
    let epoch_index = get_latest_epoch(accounts, processor).await?.index;
    processor
        .send_ixns(
            &[instruction::fund_jackpot(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                &admin_usdc,
                epoch_index,
            )],
            &[&accounts.admin],
        )
        .await
}

pub async fn withdraw_vault(
    vault: WithdrawVault,
    destination: &Pubkey,
    amount: FPUSDC,
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    processor
        .send_ixns(
            &[instruction::withdraw_vault(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                vault,
                destination,
                amount.as_usdc(),
            )],
            &[&accounts.admin],
        )
        .await
}

pub async fn mint_tokens(
    wallet: &Pubkey,
    amount: u64,
    mint: &Pubkey,
    mint_authority: &Keypair,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    let ata = get_associated_token_address(wallet, mint);
    processor
        .send_ixns(
            &[
                spl_token::instruction::mint_to(&spl_token::id(), &mint, &ata, &mint_authority.pubkey(), &[], amount)
                    .unwrap(),
            ],
            &[&mint_authority],
        )
        .await
}

pub async fn create_token_account(owner: &Pubkey, mint: &Pubkey, processor: &mut dyn SolanaTestRuntime) -> Result<()> {
    let payer = processor.get_payer().pubkey();
    processor
        .send_ixns(
            &[create_associated_token_account(&payer, owner, mint, &spl_token::id())],
            &[],
        )
        .await
}

pub async fn mint_sols(account: &Pubkey, sols: u64, processor: &mut dyn SolanaTestRuntime) -> Result<()> {
    let payer = processor.get_payer().pubkey();
    processor
        .send_ixns(
            &[system_instruction::transfer(&payer, account, sols * 1_000_000_000)],
            &[],
        )
        .await
}

// Helper fns

pub async fn get_data<T: BorshDeserialize + HasAccountType>(
    pubkey: Pubkey,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<T> {
    Ok(get_optional_data(pubkey, processor)
        .await
        .with_context(|| format!("Account not found: {pubkey}"))?
        .unwrap())
}

pub async fn get_optional_data<T: BorshDeserialize + HasAccountType>(
    pubkey: Pubkey,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<Option<T>> {
    let account = processor.get_account(pubkey).await?;
    if let Some(account) = account {
        let data = try_from_slice_unchecked(&account.data)
            .map_err(anyhow::Error::from)
            .context(format!(
                "Failed to deserialize data for account: {:?} {}",
                T::account_type(),
                pubkey
            ))?;
        Ok(Some(data))
    } else {
        Ok(None)
    }
}

pub async fn get_data_packed<T: Pack>(pubkey: Pubkey, processor: &mut dyn SolanaTestRuntime) -> Result<T> {
    let account = processor
        .get_account(pubkey)
        .await?
        .with_context(|| format!("Account not found: {pubkey}"))?;
    Pack::unpack_unchecked(&account.data).with_context(|| format!("Failed to unpack: {pubkey}"))
}

pub async fn get_latest_epoch(accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<LatestEpoch> {
    let latest_epoch = ac::latest_epoch(&accounts.program_id).pubkey;
    get_data(latest_epoch, processor).await
}

pub async fn get_epoch_pubkey(accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<Pubkey> {
    Ok(get_latest_epoch(accounts, processor).await?.epoch)
}

pub async fn get_owner_stake_balance(accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<FPUSDC> {
    let stake_pubkey = ac::stake(&accounts.program_id, &accounts.owner.pubkey()).pubkey;
    let stake: Option<Stake> = get_optional_data(stake_pubkey, processor).await?;
    if let Some(stake) = stake {
        let latest_epoch = get_latest_epoch(accounts, processor).await?;
        let balance = stake
            .balance
            .get_amount(latest_epoch.cumulative_return_rate)
            .context("Can't get balance")?;
        Ok(balance.change_precision())
    } else {
        Ok(FPUSDC::from_usdc(0))
    }
}

pub async fn get_owner_usdc_balance(accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<FPUSDC> {
    let ata = get_associated_token_address(&accounts.owner.pubkey(), &accounts.usdc_mint.pubkey());
    get_usdc_balance_by_account(&ata, processor).await
}

pub async fn get_investor_usdc_balance(accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<FPUSDC> {
    let ata = get_associated_token_address(&accounts.investor.pubkey(), &accounts.usdc_mint.pubkey());
    get_usdc_balance_by_account(&ata, processor).await
}

pub async fn get_usdc_balance_by_account(account: &Pubkey, processor: &mut dyn SolanaTestRuntime) -> Result<FPUSDC> {
    let account: spl_token::state::Account = get_data_packed(*account, processor).await?;
    Ok(FPUSDC::from_usdc(account.amount))
}
