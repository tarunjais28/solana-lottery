use num_traits::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};

use super::{InvalidConstant, NezhaVrfError};

// Not sure how this works, but error types in SPL program seems to have this

impl<T> DecodeError<T> for NezhaVrfError {
    fn type_of() -> &'static str {
        "NezhaVrfError"
    }
}

// To & From u32

impl From<NezhaVrfError> for u32 {
    fn from(error: NezhaVrfError) -> Self {
        match error {
            NezhaVrfError::InvalidInstruction => 0,
            NezhaVrfError::ProgramAlreadyInitialized => 1,
            NezhaVrfError::WinningCombinationAlreadySet => 2,
            NezhaVrfError::EpochNotInFinalising => 3,
            // 5
            //
            NezhaVrfError::MissingSignature(s) => 100 + s as u32,
            NezhaVrfError::InvalidConstant(c) => 200 + c as u32,
            NezhaVrfError::InvalidAccount(a) => 300 + a as u32,
            //
            NezhaVrfError::SystemProgramError(e) => 1000 + e as u32,
            NezhaVrfError::TokenProgramError(e) => 1100 + e as u32,
            NezhaVrfError::SwitchboardError(e) => 2000 + e,
            //
            NezhaVrfError::UnknownError(e) => 10_000 + e,
        }
    }
}

impl From<u32> for NezhaVrfError {
    fn from(e: u32) -> Self {
        match e {
            0 => NezhaVrfError::InvalidInstruction,
            1 => NezhaVrfError::ProgramAlreadyInitialized,
            2 => NezhaVrfError::WinningCombinationAlreadySet,
            3 => NezhaVrfError::EpochNotInFinalising,
            //
            e => if e >= 100 && e < 200 {
                FromPrimitive::from_u32(e - 100).map(NezhaVrfError::MissingSignature)
            } else if e >= 200 && e < 300 {
                FromPrimitive::from_u32(e - 200).map(NezhaVrfError::InvalidConstant)
            } else if e >= 300 && e < 400 {
                FromPrimitive::from_u32(e - 300).map(NezhaVrfError::InvalidAccount)
            } else if e >= 1000 && e < 1100 {
                FromPrimitive::from_u32(e - 1000).map(NezhaVrfError::SystemProgramError)
            } else if e >= 1100 && e < 1200 {
                FromPrimitive::from_u32(e - 1100).map(NezhaVrfError::TokenProgramError)
            } else if e >= 2000 && e < 9000 {
                Some(NezhaVrfError::SwitchboardError(e - 2000))
            } else if e >= 10000 {
                FromPrimitive::from_u32(e - 10000).map(NezhaVrfError::UnknownError)
            } else {
                None
            }
            .unwrap_or(NezhaVrfError::UnknownError(e)),
        }
    }
}

// To ProgramError

impl From<NezhaVrfError> for ProgramError {
    fn from(e: NezhaVrfError) -> Self {
        ProgramError::Custom(e.into())
    }
}

impl From<InvalidConstant> for ProgramError {
    fn from(e: InvalidConstant) -> Self {
        NezhaVrfError::InvalidConstant(e).into()
    }
}
