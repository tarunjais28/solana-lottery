//! Processor functions.

pub mod investment;
pub mod stake_update;
pub mod winners;

use std::ops::DerefMut;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, borsh0_10::try_from_slice_unchecked, entrypoint::ProgramResult, msg,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey,
};

use crate::{
    accounts as ac,
    accounts::VerifyPDA,
    error::*,
    fixed_point::{FPInternal, FixedPoint, FPUSDC},
    instruction::*,
    solana,
    state::*,
    utils::*,
};

use nezha_utils::load_accounts;

pub fn process_instruction<'a>(program_id: &Pubkey, accounts: &[AccountInfo<'a>], input: &[u8]) -> ProgramResult {
    let instruction = StakingInstruction::try_from_slice(input)?;
    match instruction {
        // Init
        StakingInstruction::Init {} => process_init(program_id, accounts),
        // Deposit / Withdraw
        StakingInstruction::RequestStakeUpdate { amount } => {
            stake_update::process_request_stake_update(program_id, accounts, amount)
        }
        StakingInstruction::ApproveStakeUpdate { amount } => {
            stake_update::process_approve_stake_update(program_id, accounts, amount)
        }
        StakingInstruction::CancelStakeUpdate { amount } => {
            stake_update::process_cancel_stake_update(program_id, accounts, amount)
        }
        StakingInstruction::CompleteStakeUpdate => stake_update::process_complete_stake_update(program_id, accounts),
        //
        StakingInstruction::CreateEpoch {
            expected_end_at,
            yield_split_cfg,
        } => process_create_epoch(program_id, accounts, expected_end_at, yield_split_cfg),
        StakingInstruction::ClaimWinning {
            epoch_index,
            page,
            winner_index,
            tier,
        } => process_claim_winning(program_id, accounts, epoch_index, page, winner_index, tier),
        StakingInstruction::YieldWithdrawByInvestor { tickets_info } => {
            investment::manual::process_yield_withdraw_by_investor(program_id, accounts, tickets_info)
        }
        StakingInstruction::YieldDepositByInvestor { return_amount } => {
            investment::manual::process_yield_deposit_by_investor(program_id, accounts, return_amount)
        }
        StakingInstruction::FundJackpot { epoch_index } => {
            winners::process_fund_jackpot(program_id, accounts, epoch_index)
        }
        // Francium
        StakingInstruction::FranciumInit => investment::francium::process_francium_init(program_id, accounts),
        StakingInstruction::FranciumInvest { tickets_info } => {
            investment::francium::process_francium_invest(program_id, accounts, tickets_info)
        }
        StakingInstruction::FranciumWithdraw => investment::francium::process_francium_withdraw(program_id, accounts),
        StakingInstruction::WithdrawVault { vault, amount } => {
            process_withdraw_vault(program_id, accounts, vault, amount)
        }
        StakingInstruction::CreateEpochWinnersMeta { meta_args } => {
            winners::process_create_epoch_winners_meta(program_id, accounts, meta_args)
        }
        StakingInstruction::PublishWinners {
            page_index,
            winners_input,
        } => winners::process_publish_winners(program_id, accounts, page_index, winners_input),
        StakingInstruction::RotateKey { key_type } => process_rotate_key(program_id, accounts, key_type),
        StakingInstruction::Removed1
        | StakingInstruction::Removed2
        | StakingInstruction::Removed3
        | StakingInstruction::Removed4
        | StakingInstruction::Removed5 => process_removed(program_id, accounts),
    }
}

#[inline(never)]
pub fn process_init<'a>(program_id: &Pubkey, accounts: &[AccountInfo<'a>]) -> ProgramResult {
    msg!("Ixn: Init");
    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        super_admin_info,
        admin_info,
        investor_info,
        usdc_mint_info,
        //
        vault_authority_info,
        deposit_vault_info,
        treasury_vault_info,
        insurance_vault_info,
        tier1_prize_vault_info,
        tier2_prize_vault_info,
        tier3_prize_vault_info,
        pending_deposit_vault_info,
        latest_epoch_info,
        //
        nezha_vrf_program_info,
        system_program_info,
        token_program_info,
        rent_info,
    );

    if !super_admin_info.is_signer {
        return Err(StakingError::MissingSignature(SignatureType::SuperAdmin).into());
    }

    if latest_epoch_info.lamports() != 0 {
        return Err(StakingError::ProgramAlreadyInitialized.into());
    }

    check_rent_sysvar(rent_info)?;
    check_system_program(system_program_info)?;
    check_token_program(token_program_info)?;

    // Init Vaults

    ac::vault_authority(program_id).verify(vault_authority_info)?;

    let vaults_info = vec![
        (ac::deposit_vault(program_id), deposit_vault_info),
        (ac::treasury_vault(program_id), treasury_vault_info),
        (ac::insurance_vault(program_id), insurance_vault_info),
        (ac::prize_vault(program_id, 1), tier1_prize_vault_info),
        (ac::prize_vault(program_id, 2), tier2_prize_vault_info),
        (ac::prize_vault(program_id, 3), tier3_prize_vault_info),
        (ac::pending_deposit_vault(program_id), pending_deposit_vault_info),
    ];

    for (vault_pda, vault_info) in &vaults_info {
        vault_pda.verify(vault_info)?;
    }

    for (vault_pda, vault_info) in &vaults_info {
        msg!("Creating account {:?}", vault_pda.account_type);
        solana::system_create_account(
            system_program_info,
            vault_info,
            super_admin_info,
            rent_info,
            &vault_pda.seeds(),
            token_program_info.key,
            spl_token::state::Account::LEN,
        )?;
        msg!("Initializing account");
        solana::token_init_account(
            token_program_info,
            vault_info,
            usdc_mint_info,
            vault_authority_info,
            rent_info,
        )?;
        msg!("Done");
    }

    // Init Latest Epoch

    let latest_epoch_pda = ac::latest_epoch(program_id);
    latest_epoch_pda.verify(latest_epoch_info)?;

    msg!("Create latest epoch account");
    solana::system_create_account(
        system_program_info,
        latest_epoch_info,
        super_admin_info,
        rent_info,
        &latest_epoch_pda.seeds(),
        program_id,
        LatestEpoch::max_len(),
    )?;

    // index is set to 0 initially, so the first epoch is index = 1
    BorshSerialize::serialize(
        &LatestEpoch {
            account_type: AccountType::LatestEpoch,
            contract_version: ContractVersion::V1,
            is_initialized: true,
            index: 0,
            status: EpochStatus::Ended,
            epoch: *latest_epoch_info.key,
            cumulative_return_rate: CumulativeReturnRate::unity(),
            pending_funds: PendingFunds {
                insurance: 0u8.into(),
                tier2_prize: 0u8.into(),
                tier3_prize: 0u8.into(),
            },
            pubkeys: Pubkeys {
                super_admin: super_admin_info.key.clone(),
                admin: admin_info.key.clone(),
                investor: investor_info.key.clone(),
                nezha_vrf_program_id: *nezha_vrf_program_info.key,
            },
        },
        &mut *latest_epoch_info.try_borrow_mut_data()?,
    )?;

    Ok(())
}

pub fn process_create_epoch<'a>(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'a>],
    expected_end_at: i64,
    yield_split_cfg: YieldSplitCfg,
) -> ProgramResult {
    msg!("Ixn: Create epoch");

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        admin_info,
        epoch_info,
        latest_epoch_info,
        system_program_info,
        rent_info
    );

    check_rent_sysvar(rent_info)?;
    check_system_program(system_program_info)?;

    ac::latest_epoch(program_id).verify(latest_epoch_info)?;
    let latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.data.borrow())?;

    check_admin(admin_info, &latest_epoch)?;

    if latest_epoch.status != EpochStatus::Ended {
        return Err(StakingError::InvalidEpochStatus(latest_epoch.status).into());
    }

    let current_ts = solana::sysvar_clock().unwrap().unix_timestamp;
    if current_ts >= expected_end_at {
        return Err(StakingError::EpochExpectedEndIsInPast.into());
    }

    if yield_split_cfg.jackpot == FPUSDC::zero() {
        msg!("Jackpot can't be zero");
        return Err(ProgramError::InvalidArgument);
    }

    if yield_split_cfg.jackpot > FPUSDC::from_whole_number(1_000_000_000) {
        msg!("Jackpot can't be more than a billion");
        return Err(ProgramError::InvalidArgument);
    }

    if yield_split_cfg.insurance.premium < FPUSDC::from_fixed_point_u64(1_1, 1) {
        msg!("Premium is too low. Should be >= 1.1");
        return Err(ProgramError::InvalidArgument);
    }

    if yield_split_cfg.insurance.premium > FPUSDC::from_whole_number(5) {
        msg!("Premium is too high. Should be <= 5.0");
        return Err(ProgramError::InvalidArgument);
    }

    if yield_split_cfg.insurance.probability == FPInternal::zero() {
        msg!("Probability of jackpot can't be zero");
        return Err(ProgramError::InvalidArgument);
    }

    if yield_split_cfg.insurance.probability >= FPInternal::from_fixed_point_u64(1, 3) {
        msg!("Probability of jackpot is too high. Must be less than 0.001");
        return Err(ProgramError::InvalidArgument);
    }

    if yield_split_cfg.treasury_ratio > FixedPoint::from_whole_number(1) {
        msg!("Treasury ratio can't be > 1.0");
        return Err(ProgramError::InvalidArgument);
    }

    if yield_split_cfg.tier2_prize_share == 0 {
        msg!("Tier 2 prize share can't be zero");
        return Err(ProgramError::InvalidArgument);
    }

    if yield_split_cfg.tier3_prize_share == 0 {
        msg!("Tier 3 prize share can't be zero");
        return Err(ProgramError::InvalidArgument);
    }

    if yield_split_cfg
        .tier2_prize_share
        .checked_add(yield_split_cfg.tier3_prize_share)
        .is_none()
    {
        msg!("Tier 2 prize share + Tier 3 prize share can't be > 255");
        return Err(ProgramError::InvalidArgument);
    }

    let index = latest_epoch.index + 1;
    msg!("Next epoch {}", index);

    let epoch_pda = ac::epoch(program_id, index);
    epoch_pda.verify(epoch_info)?;

    msg!("Create account");
    solana::system_create_account(
        system_program_info,
        epoch_info,
        admin_info,
        rent_info,
        &epoch_pda.seeds(),
        program_id,
        Epoch::max_len(),
    )?;

    let epoch = Epoch {
        account_type: AccountType::Epoch,
        contract_version: ContractVersion::V1,
        is_initialized: true,
        index,
        status: EpochStatus::Running,
        yield_split_cfg,
        start_at: current_ts,
        expected_end_at,
        //
        tickets_info: None,
        total_invested: None,
        //
        returns: None,
        draw_enabled: None,
        //
        end_at: None,
    };

    let mut epoch_data = epoch_info.try_borrow_mut_data()?;
    BorshSerialize::serialize(&epoch, epoch_data.deref_mut())?;

    let mut latest_epoch_mut = latest_epoch_info.try_borrow_mut_data()?;

    BorshSerialize::serialize(
        &LatestEpoch {
            is_initialized: true,
            index,
            status: EpochStatus::Running,
            epoch: *epoch_info.key,
            ..latest_epoch
        },
        latest_epoch_mut.deref_mut(),
    )?;

    Ok(())
}

pub fn process_claim_winning<'a>(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'a>],
    epoch_index: u64,
    page: u32,
    winner_index: u32,
    tier: u8,
) -> ProgramResult {
    msg!(
        "Ixn: Claim winning (Epoch {}, Page {}, Winner Index {})",
        epoch_index,
        page,
        winner_index
    );

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        owner_info,
        epoch_winners_meta_info,
        epoch_winners_page_info,
        stake_info,
        epoch_info,
        latest_epoch_info,
        vault_authority_info,
        prize_vault_info,
        deposit_vault_info,
        token_program_info,
    );

    check_token_program(token_program_info)?;

    ac::latest_epoch(program_id).verify(latest_epoch_info)?;
    let latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.data.borrow())?;

    ac::epoch(program_id, epoch_index).verify(epoch_info)?;
    ac::stake(program_id, owner_info.key).verify(stake_info)?;
    ac::epoch_winners_meta(program_id, epoch_index).verify(epoch_winners_meta_info)?;
    ac::epoch_winners_page(program_id, epoch_index, page).verify(epoch_winners_page_info)?;

    let vault_authority_pda = ac::vault_authority(program_id);
    vault_authority_pda.verify(vault_authority_info)?;
    ac::deposit_vault(program_id).verify(deposit_vault_info)?;
    ac::prize_vault(program_id, tier).verify(prize_vault_info)?;

    let winner_index_in_page = winner_index - page * MAX_NUM_WINNERS_PER_PAGE as u32;
    let mut epoch_winners_data: EpochWinnersPage = try_from_slice_unchecked(&epoch_winners_page_info.data.borrow())?;
    let winner: &mut Winner = epoch_winners_data
        .winners
        .get_mut(usize::try_from(winner_index_in_page).map_err(|_| StakingError::NumericalOverflow)?)
        .ok_or(StakingError::InvalidPrizeClaim)?;

    if tier != winner.tier || winner.address != *owner_info.key {
        return Err(StakingError::InvalidPrizeClaim.into());
    }

    let epoch_winners_meta: EpochWinnersMeta = try_from_slice_unchecked(&epoch_winners_meta_info.data.borrow())?;

    if tier == 1 && !epoch_winners_meta.jackpot_claimable {
        return Err(StakingError::JackpotNotClaimableYet.into());
    }

    if winner.claimed {
        return Err(StakingError::PrizeAlreadyClaimed.into());
    }

    msg!("Transfer prize");
    solana::token_transfer(
        token_program_info,
        prize_vault_info,
        deposit_vault_info,
        vault_authority_info,
        Some(&vault_authority_pda.seeds()),
        winner.prize.as_usdc(),
    )?;

    msg!("Update stake");
    let mut stake = Stake::try_from_slice(&stake_info.data.borrow())?;
    stake.balance = stake
        .balance
        .checked_add(winner.prize.change_precision(), latest_epoch.cumulative_return_rate)
        .ok_or(StakingError::NumericalOverflow)?;
    BorshSerialize::serialize(&stake, &mut *stake_info.try_borrow_mut_data()?)?;

    msg!("Update prize claim status");
    winner.claimed = true;
    BorshSerialize::serialize(
        &epoch_winners_data,
        &mut *epoch_winners_page_info.try_borrow_mut_data()?,
    )?;

    Ok(())
}

fn process_withdraw_vault<'a>(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'a>],
    withdraw_vault: WithdrawVault,
    amount: u64,
) -> ProgramResult {
    msg!("Ixn: Withdraw Vault: {:?}", withdraw_vault);

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        admin,
        vault_authority,
        vault,
        destination,
        latest_epoch,
        //
        token_program,
    );

    check_token_program(token_program)?;

    ac::latest_epoch(program_id).verify(latest_epoch)?;
    let latest_epoch = LatestEpoch::try_from_slice(&latest_epoch.data.borrow())?;

    check_admin(admin, &latest_epoch)?;

    let vault_authority_pda = ac::vault_authority(program_id);
    vault_authority_pda.verify(vault_authority)?;

    let vault_pda = withdraw_vault.get_pda(program_id);
    vault_pda.verify(vault)?;

    msg!("Transferring {:?}", amount);
    solana::token_transfer(
        token_program,
        vault,
        destination,
        vault_authority,
        Some(&vault_authority_pda.seeds()),
        amount,
    )?;
    Ok(())
}

fn process_rotate_key(program_id: &Pubkey, accounts: &[AccountInfo], key_type: RotateKeyType) -> ProgramResult {
    msg!("Ixn: Rotate Key: {:?}", key_type);

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        super_admin_info,
        latest_epoch_info,
        new_key_info,
    );

    ac::latest_epoch(program_id).verify(latest_epoch_info)?;
    let mut latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.data.borrow())?;

    check_super_admin(super_admin_info, &latest_epoch)?;

    let new_key = new_key_info.key.clone();

    match key_type {
        RotateKeyType::SuperAdmin => latest_epoch.pubkeys.super_admin = new_key,
        RotateKeyType::Admin => latest_epoch.pubkeys.admin = new_key,
        RotateKeyType::Investor => latest_epoch.pubkeys.investor = new_key,
    };

    BorshSerialize::serialize(&latest_epoch, &mut *latest_epoch_info.try_borrow_mut_data()?)?;

    Ok(())
}

/// Raise `StakingError::RemovedInstruction`.
pub fn process_removed(_program_id: &Pubkey, _accounts: &[AccountInfo]) -> ProgramResult {
    Err(StakingError::RemovedInstruction.into())
}
