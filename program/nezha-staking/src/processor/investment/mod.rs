//! Investment related processor functions.
pub mod francium;
pub mod manual;
pub mod returns;
#[cfg(test)]
mod tests;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, borsh0_10::try_from_slice_unchecked, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

use std::ops::DerefMut;

use crate::{accounts as ac, accounts::VerifyPDA, error::*, fixed_point::*, solana, state::*};

/// Move funds from deposit_vault into a destination account and update investment details in Epoch
/// account
pub fn invest<'a>(
    program_id: &Pubkey,
    deposit_vault_info: &AccountInfo<'a>,
    vault_authority_info: &AccountInfo<'a>,
    destination_account_info: &AccountInfo<'a>,
    latest_epoch_info: &AccountInfo<'a>,
    epoch_info: &AccountInfo<'a>,
    token_program_info: &AccountInfo<'a>,
    tickets_info: TicketsInfo,
    amount: FPUSDC,
) -> ProgramResult {
    msg!("Invest");

    let vault_authority_pda = ac::vault_authority(program_id);
    vault_authority_pda.verify(vault_authority_info)?;
    ac::deposit_vault(program_id).verify(deposit_vault_info)?;

    ac::latest_epoch(program_id).verify(latest_epoch_info)?;
    let latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.data.borrow())?;

    ac::epoch(program_id, latest_epoch.index).verify(epoch_info)?;

    if latest_epoch.status != EpochStatus::Running {
        return Err(StakingError::InvalidEpochStatus(latest_epoch.status).into());
    }

    msg!("Transferring {}", amount);
    solana::token_transfer(
        token_program_info,
        deposit_vault_info,
        destination_account_info,
        vault_authority_info,
        Some(&vault_authority_pda.seeds()),
        amount.as_usdc(),
    )?;

    msg!("Update Latest Epoch");
    let mut latest_epoch_mut = latest_epoch_info.try_borrow_mut_data()?;
    BorshSerialize::serialize(
        &LatestEpoch {
            status: EpochStatus::Yielding,
            ..latest_epoch
        },
        latest_epoch_mut.deref_mut(),
    )?;

    msg!("Update Epoch");
    let mut epoch_data: Epoch = try_from_slice_unchecked(&epoch_info.data.borrow())?;
    let mut epoch_data_mut = epoch_info.try_borrow_mut_data()?;
    epoch_data.status = EpochStatus::Yielding;
    epoch_data.total_invested = Some(amount);

    if tickets_info.tickets_url.len() > TICKETS_URL_MAX_LEN {
        msg!(
            "Tickets URL length exceeded. Max {}. Got {}",
            tickets_info.tickets_url.len(),
            TICKETS_URL_MAX_LEN
        );
        return Err(ProgramError::InvalidArgument);
    }

    if tickets_info.tickets_hash.len() > TICKETS_HASH_MAX_LEN {
        msg!(
            "Tickets Hash length exceeded. Max {}. Got {}",
            tickets_info.tickets_hash.len(),
            TICKETS_HASH_MAX_LEN
        );
        return Err(ProgramError::InvalidArgument);
    }

    epoch_data.tickets_info = Some(tickets_info);
    BorshSerialize::serialize(&epoch_data, epoch_data_mut.deref_mut())?;

    Ok(())
}

/// Move funds from a source account into the deposit vault, insurance vault, treasury vault and
/// the prize vaults.
/// Update Epoch account with returned amount and yield split details
pub fn withdraw<'a>(
    program_id: &Pubkey,
    latest_epoch_info: &AccountInfo<'a>,
    epoch_info: &AccountInfo<'a>,
    //
    source_account_info: &AccountInfo<'a>,
    source_authority_info: &AccountInfo<'a>,
    signer_seeds: Option<&[&[u8]]>,
    //
    deposit_vault_info: &AccountInfo<'a>,
    treasury_vault_info: &AccountInfo<'a>,
    insurance_vault_info: &AccountInfo<'a>,
    tier2_prize_vault_info: &AccountInfo<'a>,
    tier3_prize_vault_info: &AccountInfo<'a>,
    //
    token_program_info: &AccountInfo<'a>,
    //
    return_amount: u64,
) -> ProgramResult {
    msg!("Withdraw Investment");

    ac::deposit_vault(program_id).verify(deposit_vault_info)?;
    ac::treasury_vault(program_id).verify(treasury_vault_info)?;
    ac::insurance_vault(program_id).verify(insurance_vault_info)?;
    ac::prize_vault(program_id, 2).verify(tier2_prize_vault_info)?;
    ac::prize_vault(program_id, 3).verify(tier3_prize_vault_info)?;

    ac::latest_epoch(program_id).verify(latest_epoch_info)?;

    let mut latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.data.borrow())?;

    ac::epoch(program_id, latest_epoch.index).verify(epoch_info)?;

    if latest_epoch.status != EpochStatus::Yielding {
        return Err(StakingError::InvalidEpochStatus(latest_epoch.status).into());
    }

    let mut epoch: Epoch = try_from_slice_unchecked(&epoch_info.data.borrow())?;

    let return_amount = FPUSDC::from_usdc(return_amount);

    msg!("Distributing returns");
    let returns_info = returns::distribute_returns(
        return_amount,
        epoch.total_invested.unwrap(),
        latest_epoch.cumulative_return_rate,
        latest_epoch.pending_funds,
        returns::YieldSplitCfgInternal {
            insurance_amount: epoch
                .yield_split_cfg
                .insurance
                .calculate_amount(
                    epoch.tickets_info.as_ref().unwrap().num_tickets,
                    epoch.yield_split_cfg.jackpot,
                )
                .ok_or(StakingError::NumericalOverflow)?,
            treasury_ratio: epoch.yield_split_cfg.treasury_ratio,
            tier2_prize_share: epoch.yield_split_cfg.tier2_prize_share,
            tier3_prize_share: epoch.yield_split_cfg.tier3_prize_share,
        },
    )?;

    let transfer = |amount: FPUSDC, vault_info: &AccountInfo<'a>| -> Result<(), ProgramError> {
        solana::token_transfer(
            token_program_info,
            source_account_info,
            vault_info,
            source_authority_info,
            signer_seeds,
            amount.as_usdc(),
        )
    };

    // transfter tvl back to vault
    // Ok to unwrap() because guaranteed to be set by investor_return()
    msg!("Transfer TVL");
    transfer(returns_info.returns.deposit_back, deposit_vault_info)?;

    if returns_info.returns.insurance > 0u8.into() {
        msg!("Transfer Insurance");

        transfer(returns_info.returns.insurance, insurance_vault_info)?;
    }

    if returns_info.returns.treasury > 0u8.into() {
        msg!("Transfer Treasury");

        transfer(returns_info.returns.treasury, treasury_vault_info)?;
    }

    if returns_info.returns.tier2_prize > 0u8.into() {
        msg!("Transfer Tier2 Prize");

        transfer(returns_info.returns.tier2_prize, tier2_prize_vault_info)?;
    }

    if returns_info.returns.tier3_prize > 0u8.into() {
        msg!("Transfer Tier3 Prize");

        transfer(returns_info.returns.tier3_prize, tier3_prize_vault_info)?;
    }

    let new_status = if returns_info.draw_enabled {
        EpochStatus::Finalising
    } else {
        EpochStatus::Ended
    };

    msg!("Update Latest Epoch");
    latest_epoch.status = new_status;
    latest_epoch.cumulative_return_rate = returns_info.cumulative_return_rate;
    latest_epoch.pending_funds = returns_info.pending_funds;

    let mut latest_epoch_mut = latest_epoch_info.try_borrow_mut_data()?;
    BorshSerialize::serialize(&latest_epoch, latest_epoch_mut.deref_mut())?;

    msg!("Update Epoch");
    let current_ts = solana::sysvar_clock().unwrap().unix_timestamp;
    epoch.end_at = Some(current_ts);
    epoch.returns = Some(returns_info.returns);
    epoch.draw_enabled = Some(returns_info.draw_enabled);
    epoch.status = new_status;

    let mut epoch_data_mut = epoch_info.try_borrow_mut_data()?;
    BorshSerialize::serialize(&epoch, epoch_data_mut.deref_mut())?;

    Ok(())
}
