use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the demo program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum FarmingError {
    // 0
    /// Invalid instruction data passed in.
    #[error("Failed to unpack instruction data")]
    InstructionUnpackError,
    // 1
    /// Invalid pool account
    #[error("Invalid pool account")]
    InvalidPoolAccount,
    // 2
    /// Invalid user farming account
    #[error("Invalid user farming account")]
    InvalidUserAccount,
    //3
    /// Invalid token owner
    #[error("Invalid token owner")]
    InvalidTokenAccountOwner,
    // 4
    /// Already Initialized Account
    #[error("Already Initialized Account")]
    AlreadyInitializedAccount,
    // 5
    /// Invalid pool admin
    #[error("Invalid pool admin")]
    InvalidPoolAdmin,
    // 6
    /// Invalid token account
    #[error("Invalid token account")]
    InvalidTokenAccount,
    // 7
    /// Invalid token mint
    #[error("Invalid token mint")]
    InvalidTokenMint,
    // 8
    /// Invalid pool authority
    #[error("Invalid pool authority")]
    InvalidPoolAuthority,

    // 9
    /// Invalid pool authority
    #[error("InvalidData")]
    InvalidData,
}

impl From<FarmingError> for ProgramError {
    fn from(e: FarmingError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for FarmingError {
    fn type_of() -> &'static str {
        "Lending Pool Error"
    }
}
