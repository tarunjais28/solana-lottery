use crate::{error::InvalidConstant, state::LatestEpoch};
use nezha_staking_lib::error::{SignatureType, StakingError};
use solana_program::{account_info::AccountInfo, system_program};

pub fn check_system_program(account: &AccountInfo) -> Result<(), InvalidConstant> {
    if *account.key != system_program::id() {
        return Err(InvalidConstant::SystemProgram);
    }
    Ok(())
}

pub fn check_token_program(account: &AccountInfo) -> Result<(), InvalidConstant> {
    if *account.key != spl_token::id() {
        return Err(InvalidConstant::TokenProgram);
    }
    Ok(())
}

pub fn check_ata_program(account: &AccountInfo) -> Result<(), InvalidConstant> {
    if *account.key != spl_associated_token_account::id() {
        return Err(InvalidConstant::ATAProgram);
    }
    Ok(())
}

pub fn check_rent_sysvar(account: &AccountInfo) -> Result<(), InvalidConstant> {
    if *account.key != solana_program::sysvar::rent::id() {
        return Err(InvalidConstant::RentSysvar);
    }
    Ok(())
}

pub fn check_clock_sysvar(account: &AccountInfo) -> Result<(), InvalidConstant> {
    if *account.key != solana_program::sysvar::clock::id() {
        return Err(InvalidConstant::ClockSysvar);
    }
    Ok(())
}

pub fn check_super_admin(account: &AccountInfo, latest_epoch: &LatestEpoch) -> Result<(), StakingError> {
    if !account.is_signer {
        return Err(StakingError::MissingSignature(SignatureType::SuperAdmin));
    }

    if account.key != &latest_epoch.pubkeys.super_admin {
        return Err(StakingError::InvalidConstant(InvalidConstant::SuperAdminKey));
    }

    Ok(())
}

pub fn check_admin(account: &AccountInfo, latest_epoch: &LatestEpoch) -> Result<(), StakingError> {
    if !account.is_signer {
        return Err(StakingError::MissingSignature(SignatureType::Admin));
    }

    if account.key != &latest_epoch.pubkeys.admin {
        return Err(StakingError::InvalidConstant(InvalidConstant::AdminKey));
    }

    Ok(())
}

pub fn check_investor(account: &AccountInfo, latest_epoch: &LatestEpoch) -> Result<(), StakingError> {
    if !account.is_signer {
        return Err(StakingError::MissingSignature(SignatureType::Investor));
    }

    if account.key != &latest_epoch.pubkeys.investor {
        return Err(StakingError::InvalidConstant(InvalidConstant::InvestorKey));
    }

    Ok(())
}
