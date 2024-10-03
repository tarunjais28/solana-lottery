use solana_program::{
    msg,system_instruction,
    program_error::{
    ProgramError::MissingRequiredSignature,
    ProgramError::InvalidInstructionData
    },
    account_info::{
        next_account_info,
        AccountInfo
    },
    entrypoint::{
        ProgramResult
    },
    pubkey::{
        Pubkey
    },

};

use crate::{
    instruction::{FarmingInstructions, FarmingPoolConfig},
    state::farming_pool::FarmingPool,
    state::farming_user::FarmingUser,
};
use solana_program::program_pack::{Pack, IsInitialized};
use crate::error::FarmingError::InvalidPoolAccount;
use solana_program::program_error::ProgramError;
use crate::error::FarmingError;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::program::{invoke, invoke_signed};
use crate::utils::{token_transfer, chekc_token_account, token_balance_of, check_token_owner};
use spl_token::state::Account;
use solana_program::sysvar::clock::Clock;

/// Program state handler.
pub struct Processor {}

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        Ok(())
    }
}