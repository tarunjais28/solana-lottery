//! Error types, conversions and helper functions.

use anchor_lang::prelude::Error as AnchorError;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_program::program_error::ProgramError;
use solana_program::system_instruction::SystemError;
use spl_token::error::TokenError;
use thiserror::Error;

use crate::accounts::AccountType;

mod conversions;

#[cfg(test)]
mod tests;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum NezhaVrfError {
    // 0
    #[error("Invalid Instruction")]
    InvalidInstruction,
    #[error("Program already initialized")]
    ProgramAlreadyInitialized,
    #[error("Winning combination is already set for given epoch")]
    WinningCombinationAlreadySet,
    #[error("Epoch state is not FINALISING")]
    EpochNotInFinalising,

    // 100 + {0}
    #[error("Missing Signature: {0}")]
    MissingSignature(SignatureType),

    // 200 + {0}
    #[error("Invalid constant: {0}")]
    InvalidConstant(InvalidConstant),

    // 300 + {0}
    #[error("Invalid account provided for: {0:?}")]
    InvalidAccount(AccountType),

    // 1000 + {0}
    #[error("System Error: {0}")]
    SystemProgramError(SystemError),

    // 1100 + {0}
    #[error("Token Error: {0}")]
    TokenProgramError(TokenError),

    // 2000 + {0}
    // Anchor doesn't provide u32 -> SwitchboardError conversion.
    // They only provide ErrorCode -> u32 and SwitchboardError -> u32 conversions.
    #[error("Switchboard Error: {0}")]
    SwitchboardError(u32),

    // 10000 + {0}
    #[error("Unknown Error: {0}")]
    UnknownError(u32),
}

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum SignatureType {
    #[error("Admin")]
    Admin = 0,
    #[error("Super Admin")]
    SuperAdmin,
}

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum InvalidConstant {
    #[error("Admin Key")]
    AdminKey = 0,
    #[error("Super Admin Key")]
    SuperAdminKey,
    // 10
    #[error("System Program")]
    SystemProgram = 10,
    #[error("Token Program")]
    TokenProgram,
    #[error("ATA Program")]
    ATAProgram,
    #[error("Switchboard Program")]
    SwitchboardProgram,
    //
    #[error("Rent Sysvar")]
    RentSysvar = 20,
    #[error("Clock Sysvar")]
    ClockSysvar,
}

// The following conversion functions can be used for mapping a ProgramError to appropriate variant
// of NezhaVrfError.
// This will be used as
//  `invoke(token_instruction).map_err(NezhaVrfError::token_program_error)`
//
// It is the responsibility of the caller to make sure that the instruction they are executing
// matches the conversion function. The following will get encoded as `UnknownError`.
//  `invoke(system_instruction).map_err(NezhaVrfError::token_program_error)`

impl NezhaVrfError {
    pub fn token_program_error(error: ProgramError) -> ProgramError {
        match error {
            ProgramError::Custom(x) => match TokenError::from_u32(x) {
                Some(e) => Self::TokenProgramError(e).into(),
                None => NezhaVrfError::UnknownError(x).into(),
            },
            x => x,
        }
    }

    pub fn system_program_error(error: ProgramError) -> ProgramError {
        match error {
            ProgramError::Custom(x) => match SystemError::from_u32(x) {
                Some(e) => Self::SystemProgramError(e).into(),
                None => NezhaVrfError::UnknownError(x).into(),
            },
            x => x,
        }
    }

    pub fn switchboard_error_anchor(error: AnchorError) -> ProgramError {
        match error {
            AnchorError::AnchorError(e) => Self::SwitchboardError(e.error_code_number).into(),
            AnchorError::ProgramError(p) => p.program_error,
        }
    }

    pub fn switchboard_error(error: ProgramError) -> ProgramError {
        match error {
            ProgramError::Custom(x) => Self::SwitchboardError(x).into(),
            x => x,
        }
    }
}
