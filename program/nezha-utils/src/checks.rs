use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError, pubkey::Pubkey,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::GenericTokenAccount;

pub fn check_pubkey(name: &str, account_info: &AccountInfo, expected_key: &Pubkey) -> ProgramResult {
    if account_info.key != expected_key {
        msg!(
            "Error: Unexpected value for {}. Expected: {}. Got: {}",
            name,
            expected_key,
            account_info.key
        );
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

pub fn check_ata_account(name: &str, ata: &Pubkey, wallet: &Pubkey, mint: &Pubkey) -> ProgramResult {
    let expected_ata_pubkey = get_associated_token_address(wallet, mint);
    if &expected_ata_pubkey != ata {
        msg!(
            "Unexpected value for ATA {}. Expected: {}. Got: {}",
            name,
            expected_ata_pubkey,
            ata
        );
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

pub fn check_owned_by(account: &AccountInfo, program_id: &Pubkey) -> ProgramResult {
    if account.owner != program_id {
        msg!(
            "Error: {} is not owned by {}. It is owned by {}",
            account.key,
            program_id,
            account.owner
        );
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

pub fn check_token_account_owner(token_account: &AccountInfo, owner: &AccountInfo) -> ProgramResult {
    check_owned_by(token_account, &spl_token::id())?;
    let token_account_data = token_account.try_borrow_data()?;
    let owner_ = spl_token::state::Account::unpack_account_owner(&token_account_data);
    match owner_ {
        None => {
            msg!("Error: Unable extract token account owner from {}", token_account.key);
            return Err(ProgramError::InvalidArgument);
        }
        Some(owner_) => {
            if owner_ != owner.key {
                msg!(
                    "Error: Token account {} is not owned by {}. Owned by {}",
                    token_account.key,
                    owner.key,
                    owner_
                );
                return Err(ProgramError::InvalidArgument);
            }
        }
    }
    Ok(())
}

pub struct SignerWritable {
    pub is_signer: bool,
    pub is_writable: bool,
}

pub fn check_is_signer_writable(
    name: &str,
    account: &AccountInfo<'_>,
    signer_writable: &SignerWritable,
) -> ProgramResult {
    if signer_writable.is_signer && !account.is_signer {
        msg!("{} must be signer", name);
        return Err(ProgramError::MissingRequiredSignature);
    }
    if signer_writable.is_writable && !account.is_writable {
        msg!("{} must be writable", name);
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}
