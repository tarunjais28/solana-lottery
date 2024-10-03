use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::{clock, Sysvar},
};

use crate::error::StakingError;

pub type Seeds<'a> = &'a [&'a [u8]];

pub fn system_create_account<'a>(
    system_program: &AccountInfo<'a>,
    account: &AccountInfo<'a>,
    payer: &AccountInfo<'a>,
    rent: &AccountInfo<'a>,
    account_seeds: Seeds,
    program_id: &Pubkey,
    account_length: usize,
) -> ProgramResult {
    let rent = &Rent::from_account_info(rent)?;
    let required_lamports = rent.minimum_balance(account_length);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            account.key,
            required_lamports,
            account_length as u64,
            program_id,
        ),
        &[system_program.clone(), payer.clone(), account.clone()],
        &[account_seeds],
    )
    .map_err(StakingError::system_program_error)
}

pub fn token_init_account<'a>(
    token_program: &AccountInfo<'a>,
    account: &AccountInfo<'a>,
    mint: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    rent: &AccountInfo<'a>,
) -> ProgramResult {
    invoke(
        &spl_token::instruction::initialize_account(token_program.key, account.key, mint.key, authority.key)?,
        &[
            token_program.clone(),
            account.clone(),
            mint.clone(),
            authority.clone(),
            rent.clone(),
        ],
    )
    .map_err(StakingError::token_program_error)
}

pub fn token_transfer<'a>(
    token_program: &AccountInfo<'a>,
    from: &AccountInfo<'a>,
    to: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    authority_seeds: Option<Seeds>,
    amount: u64,
) -> ProgramResult {
    let ixn = spl_token::instruction::transfer(token_program.key, from.key, to.key, authority.key, &[], amount)?;
    let accounts = [token_program.clone(), from.clone(), to.clone(), authority.clone()];
    if let Some(authority_seeds) = authority_seeds {
        invoke_signed(&ixn, &accounts, &[authority_seeds])
    } else {
        invoke(&ixn, &accounts)
    }
    .map_err(StakingError::token_program_error)
}

pub fn sysvar_clock() -> Result<Clock, ProgramError> {
    clock::Clock::get()
}
