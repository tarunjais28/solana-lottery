//! Deposit/Withdraw related processor functions.
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError, pubkey::Pubkey,
};

use nezha_utils::load_accounts;
use std::ops::DerefMut;

use crate::{accounts as ac, accounts::VerifyPDA, error::*, fixed_point::*, solana, state::*, utils::*};

use nezha_utils::checks::*;

pub fn process_request_stake_update<'a>(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'a>],
    amount: i64,
) -> ProgramResult {
    msg!("Ixn: Request Stake Update");

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        owner_info,
        owner_usdc_info,
        stake_info,
        latest_epoch_info,
        stake_update_request_info,
        pending_deposit_vault_info,
        //
        system_program_info,
        token_program_info,
        rent_info,
    );

    if !owner_info.is_signer {
        return Err(StakingError::MissingSignature(SignatureType::Owner).into());
    }

    check_rent_sysvar(rent_info)?;
    check_system_program(system_program_info)?;
    check_token_program(token_program_info)?;

    let stake_pda = ac::stake(program_id, owner_info.key);
    stake_pda.verify(stake_info)?;

    let latest_epoch_pda = ac::latest_epoch(program_id);
    latest_epoch_pda.verify(latest_epoch_info)?;

    let stake_update_request_pda = ac::stake_update_request(program_id, owner_info.key);
    stake_update_request_pda.verify(stake_update_request_info)?;

    let pending_deposit_vault_pda = ac::pending_deposit_vault(program_id);
    pending_deposit_vault_pda.verify(pending_deposit_vault_info)?;

    check_token_account_owner(owner_usdc_info, owner_info)?;

    if stake_update_request_info.lamports() != 0 {
        msg!("Multiple stake updates are not allowed. Cancel the existing one to issue a new one");
        return Err(StakingError::StakeUpdateRequestExists.into());
    }

    if amount == 0 {
        msg!("Amount can't be zero");
        return Err(ProgramError::InvalidArgument)?;
    }

    let state;
    if amount > 0 {
        msg!("Transfer deposit amount into pending vault");

        solana::token_transfer(
            token_program_info,
            owner_usdc_info,
            pending_deposit_vault_info,
            owner_info,
            None,
            amount.try_into().map_err(|_| StakingError::NumericalOverflow)?,
        )?;

        // Deposits require an approval step
        state = StakeUpdateState::PendingApproval;
    } else {
        // No stake exists
        if stake_info.lamports() == 0 {
            return Err(StakingError::InsufficientBalance.into());
        }
        let stake = Stake::try_from_slice(&stake_info.try_borrow_data()?)?;
        let latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.try_borrow_data()?)?;

        let balance = stake
            .balance
            .get_amount(latest_epoch.cumulative_return_rate)
            .ok_or(StakingError::NumericalOverflow)?;
        if balance < FPInternal::from_usdc(amount.abs() as u64) {
            return Err(StakingError::InsufficientBalance.into());
        }
        // Withdrawals don't have an approval step. They are immediately queued.
        state = StakeUpdateState::Queued;
    }

    msg!("Creating stake update request account");
    solana::system_create_account(
        system_program_info,
        stake_update_request_info,
        owner_info,
        rent_info,
        &stake_update_request_pda.seeds(),
        program_id,
        StakeUpdateRequest::max_len(),
    )?;

    let mut data = stake_update_request_info.try_borrow_mut_data()?;

    BorshSerialize::serialize(
        &StakeUpdateRequest {
            account_type: AccountType::StakeUpdateRequest,
            contract_version: ContractVersion::V1,
            is_initialized: true,
            owner: owner_info.key.clone(),
            amount,
            state,
        },
        data.deref_mut(),
    )?;

    Ok(())
}

pub fn process_approve_stake_update(program_id: &Pubkey, accounts: &[AccountInfo], amount: i64) -> ProgramResult {
    msg!("Ixn: Approve stake update");

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        signer_info,
        owner_info,
        stake_update_request_info,
        latest_epoch_info,
    );

    ac::latest_epoch(program_id).verify(latest_epoch_info)?;
    let latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.data.borrow())?;

    let stake_update_request_pda = ac::stake_update_request(program_id, owner_info.key);
    stake_update_request_pda.verify(stake_update_request_info)?;

    let mut data = stake_update_request_info.try_borrow_mut_data()?;

    let mut stake_update_request = StakeUpdateRequest::try_from_slice(&data)?;

    if stake_update_request.state != StakeUpdateState::PendingApproval {
        msg!("Stake update can't be approved when not in PendingApproval state");
        return Err(StakingError::InvalidStakeUpdateState(stake_update_request.state).into());
    }

    // Require Admin signature for approving Deposits as only the admin is authorized to approve a
    // deposit after AML checks and other scrutiny
    if stake_update_request.amount > 0 {
        check_admin(signer_info, &latest_epoch)?;
    }

    if amount != stake_update_request.amount {
        msg!(
            "Stake update amount mismatch. Expected: {}. Got: {}",
            stake_update_request.amount,
            amount
        );
        return Err(StakingError::StakeUpdateAmountMismatch.into());
    }

    stake_update_request.state = StakeUpdateState::Queued;

    BorshSerialize::serialize(&stake_update_request, data.deref_mut())?;

    Ok(())
}

pub fn process_complete_stake_update<'a>(program_id: &Pubkey, accounts: &[AccountInfo<'a>]) -> ProgramResult {
    msg!("Ixn: Complete stake update");

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        payer_info,
        owner_info,
        vault_authority_info,
        //
        stake_update_request_info,
        stake_info,
        latest_epoch_info,
        pending_deposit_vault_info,
        deposit_vault_info,
        owner_usdc_ata_info,
        //
        token_program_info,
        system_program_info,
        rent_info
    );

    check_rent_sysvar(rent_info)?;
    check_system_program(system_program_info)?;
    check_token_program(token_program_info)?;

    ac::latest_epoch(program_id).verify(latest_epoch_info)?;
    let latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.data.borrow())?;

    let vault_authority_pda = ac::vault_authority(program_id);
    vault_authority_pda.verify(vault_authority_info)?;

    let stake_pda = ac::stake(program_id, owner_info.key);
    stake_pda.verify(stake_info)?;

    ac::deposit_vault(program_id).verify(deposit_vault_info)?;

    ac::pending_deposit_vault(program_id).verify(pending_deposit_vault_info)?;

    let stake_update_request_pda = ac::stake_update_request(program_id, owner_info.key);
    stake_update_request_pda.verify(stake_update_request_info)?;

    check_token_account_owner(owner_usdc_ata_info, owner_info)?;

    let stake_update_request = StakeUpdateRequest::try_from_slice(&stake_update_request_info.data.borrow())?;

    if stake_update_request.state != StakeUpdateState::Queued {
        msg!("Stake update can't be completed when not in Queued state");
        return Err(StakingError::InvalidStakeUpdateState(stake_update_request.state).into());
    }

    if latest_epoch.status != EpochStatus::Running {
        msg!("Stake update can't be completed when epoch is not in Running state");
        return Err(StakingError::InvalidEpochStatus(latest_epoch.status).into());
    }

    let mut stake = if stake_info.lamports() == 0 {
        Stake {
            account_type: AccountType::Stake,
            contract_version: ContractVersion::V1,
            is_initialized: true,
            owner: *owner_info.key,
            balance: FloatingBalance::new(0u8.into(), latest_epoch.cumulative_return_rate),
            created_epoch_index: latest_epoch.index,
            updated_epoch_index: latest_epoch.index,
        }
    } else {
        Stake::try_from_slice(&stake_info.data.borrow())?
    };

    let balance = stake
        .balance
        .get_amount(latest_epoch.cumulative_return_rate)
        .ok_or(StakingError::NumericalOverflow)?
        .as_usdc();

    if stake_update_request.amount > 0 {
        let deposit_amount: u64 = stake_update_request
            .amount
            .abs()
            .try_into()
            .map_err(|_| StakingError::NumericalOverflow)?;

        msg!("Transfer deposit amount");

        solana::token_transfer(
            token_program_info,
            pending_deposit_vault_info,
            deposit_vault_info,
            vault_authority_info,
            Some(&vault_authority_pda.seeds()),
            deposit_amount,
        )?;

        let new_balance = balance
            .checked_add(deposit_amount)
            .ok_or(StakingError::NumericalOverflow)?;

        stake.balance = FloatingBalance::new(FixedPoint::from_usdc(new_balance), latest_epoch.cumulative_return_rate);
        stake.updated_epoch_index = latest_epoch.index;
    } else {
        let withdraw_amount: u64 = stake_update_request
            .amount
            .abs()
            .try_into()
            .map_err(|_| StakingError::NumericalOverflow)?;

        let withdraw_amount = withdraw_amount.min(balance);
        msg!("Transfer withdraw amount");
        solana::token_transfer(
            token_program_info,
            deposit_vault_info,
            owner_usdc_ata_info,
            vault_authority_info,
            Some(&vault_authority_pda.seeds()),
            withdraw_amount,
        )?;

        let new_balance = balance
            .checked_sub(withdraw_amount)
            .expect("We checked that withdraw_amount <= balance");

        stake.balance = FloatingBalance::new(FixedPoint::from_usdc(new_balance), latest_epoch.cumulative_return_rate);
        stake.updated_epoch_index = latest_epoch.index;
    };

    if stake_info.lamports() == 0 {
        msg!("Create stake account");

        solana::system_create_account(
            system_program_info,
            stake_info,
            payer_info,
            rent_info,
            &stake_pda.seeds(),
            program_id,
            Stake::max_len(),
        )?;
    }

    msg!("Update stake account");
    BorshSerialize::serialize(&stake, &mut *stake_info.try_borrow_mut_data()?)?;

    msg!("Close stake update request");
    close_account_and_recoup_sols(stake_update_request_info, payer_info)?;

    Ok(())
}

pub fn process_cancel_stake_update<'a>(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'a>],
    amount: i64,
) -> ProgramResult {
    msg!("Ixn: Cancel stake update");

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        signer_info,
        owner_info,
        owner_usdc_info,
        stake_update_request_info,
        pending_deposit_vault_info,
        vault_authority_info,
        latest_epoch_info,
        token_program_info
    );

    ac::latest_epoch(program_id).verify(latest_epoch_info)?;
    let latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.data.borrow())?;

    if signer_info.key == &latest_epoch.pubkeys.admin {
        if !signer_info.is_signer {
            return Err(StakingError::MissingSignature(SignatureType::Admin).into());
        }
    } else if signer_info.key == owner_info.key {
        if !signer_info.is_signer {
            return Err(StakingError::MissingSignature(SignatureType::Owner).into());
        }
    } else {
        return Err(ProgramError::InvalidArgument);
    }

    check_token_account_owner(owner_usdc_info, owner_info)?;

    check_token_program(token_program_info)?;
    ac::stake_update_request(program_id, owner_info.key).verify(stake_update_request_info)?;
    ac::pending_deposit_vault(program_id).verify(pending_deposit_vault_info)?;

    let vault_authority_pda = ac::vault_authority(program_id);
    vault_authority_pda.verify(vault_authority_info)?;

    if stake_update_request_info.lamports() == 0 {
        msg!("Stake update request doesn't exist.");
        return Ok(());
    }

    let stake_update_request = StakeUpdateRequest::try_from_slice(&stake_update_request_info.data.borrow())?;

    if amount != stake_update_request.amount {
        msg!(
            "Stake update amount mismatch. Expected: {}. Got: {}",
            stake_update_request.amount,
            amount
        );
        return Err(StakingError::StakeUpdateAmountMismatch.into());
    }

    if stake_update_request.amount > 0 {
        msg!("Transfer back deposited amount");
        solana::token_transfer(
            token_program_info,
            pending_deposit_vault_info,
            owner_usdc_info,
            vault_authority_info,
            Some(&vault_authority_pda.seeds()),
            stake_update_request
                .amount
                .try_into()
                .expect("We checked that this is > 0 "),
        )?;
    }

    msg!("Close stake update request account");
    close_account_and_recoup_sols(stake_update_request_info, signer_info)?;
    Ok(())
}

fn close_account_and_recoup_sols(account: &AccountInfo, transfer_sols_to: &AccountInfo) -> ProgramResult {
    **transfer_sols_to.lamports.borrow_mut() = transfer_sols_to
        .lamports()
        .checked_add(account.lamports())
        .ok_or(StakingError::NumericalOverflow)?;
    **account.lamports.borrow_mut() = 0;
    *account.try_borrow_mut_data()? = &mut [];
    Ok(())
}
