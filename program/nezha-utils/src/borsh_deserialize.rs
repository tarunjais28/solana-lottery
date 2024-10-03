use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, borsh0_10::try_from_slice_unchecked, program_error::ProgramError};

/// Helper function to deserialze Borsh encoded accounts correctly.
/// `T::deserialize()` and `T::try_from_slice()` don't account for variable length structs.
/// ie, when you have an Option<T> or Vec<T> inside a struct.
/// We must always use `solana_program::borsh::try_from_slice_unchecked`.
pub fn borsh_deserialize<T>(account: &AccountInfo) -> Result<T, ProgramError>
where
    T: BorshDeserialize,
{
    let account: T = try_from_slice_unchecked(&account.try_borrow_data()?)?;
    Ok(account)
}
