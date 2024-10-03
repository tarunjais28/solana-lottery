use crate::{error::InvalidConstant, state::NezhaVrfProgramState};
use solana_program::msg;
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

pub fn check_super_admin(
    account: &AccountInfo,
    nezha_vrf_program_state: &NezhaVrfProgramState,
) -> Result<(), InvalidConstant> {
    if account.key != &nezha_vrf_program_state.pubkeys.super_admin {
        return Err(InvalidConstant::SuperAdminKey);
    }

    Ok(())
}

pub fn check_admin(
    account: &AccountInfo,
    nezha_vrf_program_state: &NezhaVrfProgramState,
) -> Result<(), InvalidConstant> {
    if account.key != &nezha_vrf_program_state.pubkeys.admin {
        msg!("Error: Admin key mismatch");
        return Err(InvalidConstant::AdminKey);
    }

    Ok(())
}

pub fn check_switchboard_program(
    account: &AccountInfo,
    nezha_vrf_program_state: &NezhaVrfProgramState,
) -> Result<(), InvalidConstant> {
    if account.key != &nezha_vrf_program_state.pubkeys.switchboard_program_id {
        return Err(InvalidConstant::SwitchboardProgram);
    }

    Ok(())
}
