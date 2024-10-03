use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use arrayref::array_ref;
use crate::error::FarmingError;
use solana_program::program::invoke_signed;
use spl_token::state::{Mint, Account};
use solana_program::program_pack::Pack;

/// get token balance
#[inline(always)]
pub fn token_balance_of(token_account: &AccountInfo) -> Result<u64, ProgramError> {
    let src = &token_account.data.borrow();
    let amount_bytes = array_ref![src, 64, 8];

    return Ok(u64::from_le_bytes(*amount_bytes));
}

// check token owner
#[inline(always)]
pub fn check_token_owner(token_account: &AccountInfo, owner: &Pubkey) -> ProgramResult {
    let src = &token_account.data.borrow();
    let owner_bytes = array_ref![src, 32, 32];

    if !owner_bytes.eq(owner.as_ref()) {
        return Err(FarmingError::InvalidTokenAccountOwner.into());
    }

    Ok(())
}

/// Issue a spl_token `Transfer` instruction.
pub fn token_transfer<'a>(
    token_program: AccountInfo<'a>,
    source: AccountInfo<'a>,
    destination: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    signers_seeds: &[&[&[u8]]],
    amount: u64,
) -> Result<(), ProgramError> {
    let ix = spl_token::instruction::transfer(
        token_program.key,
        source.key,
        destination.key,
        authority.key,
        &[authority.key],
        amount,
    )?;
    invoke_signed(
        &ix,
        &[source, destination, authority, token_program],
        signers_seeds,
    )
}

/// Verify Token Account and Mint
pub fn chekc_token_account(
    token_program_id: &AccountInfo,
    token_mint: &AccountInfo,
    token_account: &AccountInfo
) -> Result<(), ProgramError> {
    let account = Account::unpack(&token_account.data.borrow())?;

    if token_mint.owner.ne(token_program_id.key ) {
        return Err(FarmingError::InvalidTokenMint.into());
    }

    if token_account.owner.ne(token_program_id.key) {
        return Err(FarmingError::InvalidTokenAccount.into());
    }

    if account.mint.ne(token_mint.key) {
        return Err(FarmingError::InvalidTokenAccount.into());
    }

    Ok(())
}


