use num_traits::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};

use super::{InvalidConstant, StakingError};

// Not sure how this works, but error types in SPL program seems to have this

impl<T> DecodeError<T> for StakingError {
    fn type_of() -> &'static str {
        "StakingError"
    }
}

// To & From u32

impl From<StakingError> for u32 {
    fn from(error: StakingError) -> Self {
        match error {
            StakingError::InvalidInstruction => 0,
            StakingError::NumericalOverflow => 1,
            StakingError::NotEnoughStakeBalance => 2,
            StakingError::EpochExpectedEndIsInPast => 3,
            StakingError::WinningCombinationAlreadyPublished => 4,
            // 5
            StakingError::WinningCombinationNotPublished => 5,
            StakingError::JackpotNotClaimableYet => 6,
            StakingError::JackpotAlreadyClaimable => 7,
            StakingError::NoPrizeToClaim => 8,
            StakingError::YieldNotWithdrawn => 9,
            // 10
            StakingError::ReturnAmountIsZero => 10,
            StakingError::WinnersAlreadyPublished => 11,
            StakingError::InvalidWinnerTier => 12,
            StakingError::ProcessedWinnersMetaMismatch => 13,
            StakingError::WinnerIndexOutOfBounds => 14,
            // 15
            StakingError::WrongNumberOfWinnersInPage => 15,
            StakingError::UnexpectedWinnerIndex => 16,
            StakingError::PageIndexOutOfBounds => 17,
            StakingError::RemovedInstruction => 18,
            StakingError::ReturnAmountIsNonZeroButInvestedIsZero => 19,
            // 20
            StakingError::InvalidPrizeClaim => 20,
            StakingError::PrizeAlreadyClaimed => 21,
            StakingError::StakeUpdateRequestExists => 22,
            StakingError::StakeUpdateAmountMismatch => 23,
            StakingError::ProgramAlreadyInitialized => 24,
            // 25
            StakingError::PageIndexNotInSequence => 25,
            StakingError::InsufficientBalance => 26,
            //
            StakingError::MissingSignature(s) => 100 + s as u32,
            StakingError::InvalidConstant(c) => 200 + c as u32,
            StakingError::InvalidAccount(a) => 300 + a as u32,
            StakingError::InvalidEpochStatus(s) => 400 + s as u32,
            StakingError::InvalidStakeUpdateState(s) => 450 + s as u32,
            //
            StakingError::SystemProgramError(e) => 1000 + e as u32,
            StakingError::TokenProgramError(e) => 1100 + e as u32,
            StakingError::FranciumLendingError(e) => 1200 + e as u32,
            StakingError::FranciumFarmingError(e) => 1300 + e as u32,
            //
            StakingError::UnknownError(e) => 10_000 + e,
        }
    }
}

impl From<u32> for StakingError {
    fn from(e: u32) -> Self {
        match e {
            0 => StakingError::InvalidInstruction,
            1 => StakingError::NumericalOverflow,
            2 => StakingError::NotEnoughStakeBalance,
            3 => StakingError::EpochExpectedEndIsInPast,
            4 => StakingError::WinningCombinationAlreadyPublished,
            //
            5 => StakingError::WinningCombinationNotPublished,
            6 => StakingError::JackpotNotClaimableYet,
            7 => StakingError::JackpotAlreadyClaimable,
            8 => StakingError::NoPrizeToClaim,
            9 => StakingError::YieldNotWithdrawn,
            //
            10 => StakingError::ReturnAmountIsZero,
            11 => StakingError::WinnersAlreadyPublished,
            12 => StakingError::InvalidWinnerTier,
            13 => StakingError::ProcessedWinnersMetaMismatch,
            14 => StakingError::WinnerIndexOutOfBounds,
            //
            15 => StakingError::WrongNumberOfWinnersInPage,
            16 => StakingError::UnexpectedWinnerIndex,
            17 => StakingError::PageIndexOutOfBounds,
            18 => StakingError::RemovedInstruction,
            19 => StakingError::ReturnAmountIsNonZeroButInvestedIsZero,
            //
            20 => StakingError::InvalidPrizeClaim,
            21 => StakingError::PrizeAlreadyClaimed,
            22 => StakingError::StakeUpdateRequestExists,
            23 => StakingError::StakeUpdateAmountMismatch,
            24 => StakingError::ProgramAlreadyInitialized,
            //
            25 => StakingError::PageIndexNotInSequence,
            26 => StakingError::InsufficientBalance,
            //
            e => if e >= 100 && e < 200 {
                FromPrimitive::from_u32(e - 100).map(StakingError::MissingSignature)
            } else if e >= 200 && e < 300 {
                FromPrimitive::from_u32(e - 200).map(StakingError::InvalidConstant)
            } else if e >= 300 && e < 400 {
                FromPrimitive::from_u32(e - 300).map(StakingError::InvalidAccount)
            } else if e >= 400 && e < 450 {
                FromPrimitive::from_u32(e - 400).map(StakingError::InvalidEpochStatus)
            } else if e >= 450 && e < 500 {
                FromPrimitive::from_u32(e - 450).map(StakingError::InvalidStakeUpdateState)
            } else if e >= 1000 && e < 1100 {
                FromPrimitive::from_u32(e - 1000).map(StakingError::SystemProgramError)
            } else if e >= 1100 && e < 1200 {
                FromPrimitive::from_u32(e - 1100).map(StakingError::TokenProgramError)
            } else if e >= 1200 && e < 1300 {
                FromPrimitive::from_u32(e - 1200).map(StakingError::FranciumLendingError)
            } else if e >= 1300 && e < 1400 {
                FromPrimitive::from_u32(e - 1300).map(StakingError::FranciumFarmingError)
            } else if e >= 10000 {
                FromPrimitive::from_u32(e - 10000).map(StakingError::UnknownError)
            } else {
                None
            }
            .unwrap_or(StakingError::UnknownError(e)),
        }
    }
}

// To ProgramError

impl From<StakingError> for ProgramError {
    fn from(e: StakingError) -> Self {
        ProgramError::Custom(e.into())
    }
}

impl From<InvalidConstant> for ProgramError {
    fn from(e: InvalidConstant) -> Self {
        StakingError::InvalidConstant(e).into()
    }
}
