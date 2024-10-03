use super::mem;

use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError, pubkey::Pubkey,
    system_instruction::SystemInstruction,
};

pub fn emulate(account_infos: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let ixn: SystemInstruction = bincode::deserialize(input).map_err(|_| ProgramError::InvalidInstructionData)?;
    match ixn {
        SystemInstruction::CreateAccount { lamports, space, owner } => {
            let funder = &account_infos[0];
            let new_account = &account_infos[1];

            if !funder.is_signer || !new_account.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
            if !funder.is_writable || !new_account.is_writable {
                return Err(ProgramError::InvalidArgument);
            }

            if funder.lamports() < lamports {
                return Err(ProgramError::InsufficientFunds);
            }
            **funder.lamports.borrow_mut() -= lamports;

            if let Some(account) = mem::get_account(new_account.key) {
                if unsafe { *account.lamports.get() } > 0 {
                    println!("Account already initialized {}", new_account.key);
                    return Err(ProgramError::AccountAlreadyInitialized);
                }
            }

            let ac_mem = {
                mem::new_account(*new_account.key, space as _, lamports, owner);
                mem::get_account(new_account.key).unwrap()
            };
            let lamports: &mut u64 = unsafe { &mut *ac_mem.lamports.get() };
            let data: &mut [u8] = unsafe { (&mut *ac_mem.data.get()).as_mut_slice() };
            let owner: &Pubkey = unsafe { &*ac_mem.owner.get() };
            new_account.lamports.replace(lamports);
            new_account.data.replace(data);
            new_account.assign(owner);
        }
        SystemInstruction::Assign { owner } => {
            let new_account = &account_infos[0];
            new_account.assign(&owner);
        }
        SystemInstruction::Transfer { lamports } => {
            let funder = &account_infos[0];
            let recipient = &account_infos[1];

            if funder.lamports() < lamports {
                return Err(ProgramError::InsufficientFunds);
            }
            **funder.lamports.borrow_mut() -= lamports;

            if mem::get_account(recipient.key).is_none() {
                let ac_mem = {
                    mem::new_account(*recipient.key, 0, lamports, solana_program::system_program::ID);
                    mem::get_account(recipient.key).unwrap()
                };
                let lamports: &mut u64 = unsafe { &mut *ac_mem.lamports.get() };
                let data: &mut [u8] = unsafe { (&mut *ac_mem.data.get()).as_mut_slice() };
                let owner: &Pubkey = unsafe { &*ac_mem.owner.get() };
                recipient.lamports.replace(lamports);
                recipient.data.replace(data);
                recipient.assign(owner);
            } else {
                **recipient.lamports.borrow_mut() += lamports;
            }
        }
        _ => todo!(),
    };
    Ok(())
}
