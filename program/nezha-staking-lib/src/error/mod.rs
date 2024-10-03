//! Error types, conversions and helper functions.

use francium_lending_pool::error::LendingError;
use francium_lending_rewards_pool::error::FarmingError;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_program::program_error::ProgramError;
use solana_program::system_instruction::SystemError;
use spl_token::error::TokenError;
use thiserror::Error;

use crate::{
    accounts::AccountType,
    state::{EpochStatus, StakeUpdateState},
};

mod conversions;
#[cfg(test)]
mod tests;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum StakingError {
    // 0
    #[error("Invalid Instruction")]
    InvalidInstruction,
    #[error("Numerical Overflow")]
    NumericalOverflow,
    #[error("Not enough stake balance")]
    NotEnoughStakeBalance,
    #[error("Epoch expected end is in the past")]
    EpochExpectedEndIsInPast,
    #[error("Winning combination already published")]
    WinningCombinationAlreadyPublished,
    // 5
    #[error("Winning combination not published")]
    WinningCombinationNotPublished,
    #[error("Jackpot is not claimable yet")]
    JackpotNotClaimableYet,
    #[error("Jackpot already claimable")]
    JackpotAlreadyClaimable,
    #[error("No prize to claim")]
    NoPrizeToClaim,
    #[error("Yield not withdrawn")]
    YieldNotWithdrawn,
    // 10
    #[error("Return amount is 0")]
    ReturnAmountIsZero,
    #[error("Winners already published")]
    WinnersAlreadyPublished,
    #[error("Invalid winner tier")]
    InvalidWinnerTier,
    #[error("Processed winners meta mismatch")]
    ProcessedWinnersMetaMismatch,
    #[error("Winner index out of bounds")]
    WinnerIndexOutOfBounds,
    // 15
    #[error("Wrong number of winners in page")]
    WrongNumberOfWinnersInPage,
    #[error("Unexpected winner index")]
    UnexpectedWinnerIndex,
    #[error("Page index out of bounds")]
    PageIndexOutOfBounds,
    #[error("Removed instruction")]
    RemovedInstruction,
    #[error("Return amount is non zero but total invested is zero")]
    ReturnAmountIsNonZeroButInvestedIsZero,
    // 20
    #[error("Invalid prize claim")]
    InvalidPrizeClaim,
    #[error("Prize already claimed")]
    PrizeAlreadyClaimed,
    #[error("A pending stake update request exists. Cancel it to issue a new one")]
    StakeUpdateRequestExists,
    #[error("Stake update amount mismatch")]
    StakeUpdateAmountMismatch,
    #[error("Program already initialized")]
    ProgramAlreadyInitialized,
    // 25
    #[error("Page index not in sequence")]
    PageIndexNotInSequence,
    #[error("Insufficient Balance")]
    InsufficientBalance,

    // 100 + {0}
    #[error("Missing Signature: {0}")]
    MissingSignature(SignatureType),

    // 200 + {0}
    #[error("Invalid constant: {0}")]
    InvalidConstant(InvalidConstant),

    // 300 + {0}
    #[error("Invalid account provided for: {0:?}")]
    InvalidAccount(AccountType),

    // 400 + {0}
    #[error("Cannot proceed with this epoch status: {0}")]
    InvalidEpochStatus(EpochStatus),

    // 450 + {0}
    #[error("Cannot proceed with this stake update state: {0}")]
    InvalidStakeUpdateState(StakeUpdateState),

    // 1000 + {0}
    #[error("System Error: {0}")]
    SystemProgramError(SystemError),

    // 1100 + {0}
    #[error("Token Error: {0}")]
    TokenProgramError(TokenError),

    // 1200 + {0}
    #[error("Francium Lending Error: {0}")]
    FranciumLendingError(LendingError),

    // 1300 + {0}
    #[error("Francium Farming Error: {0}")]
    FranciumFarmingError(FarmingError),

    // 10000 + {0}
    #[error("Unknown Error: {0}")]
    UnknownError(u32),
}

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum SignatureType {
    #[error("Admin")]
    Admin = 0,
    #[error("Owner")]
    Owner,
    #[error("Investor")]
    Investor,
    #[error("Super Admin")]
    SuperAdmin,
}

#[derive(Error, Debug, Clone, Copy, PartialEq, Eq, FromPrimitive)]
pub enum InvalidConstant {
    #[error("Admin Key")]
    AdminKey = 0,
    #[error("Investor Key")]
    InvestorKey,
    #[error("Super Admin Key")]
    SuperAdminKey,
    // 10
    #[error("System Program")]
    SystemProgram = 10,
    #[error("Token Program")]
    TokenProgram,
    #[error("ATA Program")]
    ATAProgram,
    //
    #[error("Rent Sysvar")]
    RentSysvar = 20,
    #[error("Clock Sysvar")]
    ClockSysvar,
}

// The following conversion functions can be used for mapping a ProgramError to appropriate variant
// of StakingError.
// This will be used as
//  `invoke(token_instruction).map_err(StakingError::token_program_error)`
//
// It is the responsibility of the caller to make sure that the instruction they are executing
// matches the conversion function. The following will get encoded as `UnknownError`.
//  `invoke(system_instruction).map_err(StakingError::token_program_error)`

impl StakingError {
    pub fn token_program_error(error: ProgramError) -> ProgramError {
        match error {
            ProgramError::Custom(x) => match TokenError::from_u32(x) {
                Some(e) => Self::TokenProgramError(e).into(),
                None => StakingError::UnknownError(x).into(),
            },
            x => x,
        }
    }

    pub fn system_program_error(error: ProgramError) -> ProgramError {
        match error {
            ProgramError::Custom(x) => match SystemError::from_u32(x) {
                Some(e) => Self::SystemProgramError(e).into(),
                None => StakingError::UnknownError(x).into(),
            },
            x => x,
        }
    }

    pub fn francium_lending_error(error: ProgramError) -> ProgramError {
        match error {
            ProgramError::Custom(x) => match LendingError::from_u32(x) {
                Some(e) => Self::FranciumLendingError(e).into(),
                None => StakingError::UnknownError(x).into(),
            },
            x => x,
        }
    }

    pub fn francium_farming_error(error: ProgramError) -> ProgramError {
        match error {
            ProgramError::Custom(x) => match FarmingError::from_u32(x) {
                Some(e) => Self::FranciumFarmingError(e).into(),
                None => StakingError::UnknownError(x).into(),
            },
            x => x,
        }
    }
}
