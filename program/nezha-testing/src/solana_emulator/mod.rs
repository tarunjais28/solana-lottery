mod mem;
mod syscall_stubs;
mod system_program;

use anyhow::{anyhow, bail, Result};
use solana_sdk::{signature::Keypair, signer::Signer};

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    rc::Rc,
};

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};
use std::cell::RefCell;

use self::mem::AccountStatic;

pub type ProcessorFn = for<'a> fn(&Pubkey, &[AccountInfo<'a>], &[u8]) -> ProgramResult;

/// Function that converts error code to corresponding concrete error
pub type ErrorFn = fn(u32) -> Option<Box<dyn Display + Send + Sync + 'static>>;

thread_local! {
    static RETURN_DATA: RefCell<Option<(Pubkey, Vec<u8>)>> = RefCell::new(None);
    static CURRENT_PROGRAM_ID: RefCell<Option<Pubkey>> = RefCell::new(None);
    static ACCOUNT_NAMES: RefCell<HashMap<Pubkey, String>> = RefCell::new(HashMap::new());
    static PROCESSORS: RefCell<HashMap<Pubkey, ProcessorFn>> = RefCell::new(HashMap::new());
    static ERRORS: RefCell<HashMap<Pubkey, ErrorFn>> = RefCell::new(HashMap::new());
}

pub fn init() {
    mem::init();
    syscall_stubs::init();
}

pub use mem::{begin_txn, commit_txn, get_account, new_account, new_account_with_data, remove_closed_accounts, RENT};

pub fn invoke(ixns: &[Instruction], signers: &[&Keypair], payer: &Keypair) -> Result<()> {
    mem::begin_txn();
    for (i, ixn) in ixns.iter().enumerate() {
        check_signers(i, &ixn.accounts, signers, payer)?;

        let mut account_infos = Vec::new();
        let mut accounts_before: Vec<AccountStatic> = Vec::new();
        for ac in &ixn.accounts {
            let ac_mem = {
                let ac_mem = mem::get_account(&ac.pubkey);
                match ac_mem {
                    None => {
                        mem::new_account(ac.pubkey, 0, 0, Pubkey::default());
                        mem::get_account(&ac.pubkey).unwrap()
                    }
                    Some(ac_mem) => ac_mem,
                }
            };
            accounts_before.push((&*ac_mem).into());

            let lamports: &mut u64 = unsafe { &mut *ac_mem.lamports.get() };
            let data: &mut [u8] = unsafe { (&mut *ac_mem.data.get()).as_mut_slice() };
            let owner: &Pubkey = unsafe { &*ac_mem.owner.get() };
            let ac_info = AccountInfo {
                key: &ac.pubkey,
                executable: false,
                is_signer: ac.is_signer,
                is_writable: ac.is_writable,
                rent_epoch: 0,
                lamports: Rc::new(RefCell::new(lamports)),
                data: Rc::new(RefCell::new(data)),
                owner,
            };
            account_infos.push(ac_info);
        }

        invoke_unchecked(ixn, &account_infos).map_err(|e| program_error_to_anyhow(i, &ixn.program_id, e))?;

        let writable_accounts: HashSet<Pubkey> = ixn
            .accounts
            .iter()
            .filter(|ac| ac.is_writable)
            .map(|ac| ac.pubkey)
            .collect();

        for (ac, ac_before) in ixn.accounts.iter().zip(accounts_before) {
            let ac_mem = mem::get_account(&ac.pubkey).unwrap();
            let ac_after: AccountStatic = (&*ac_mem).into();

            // Checking !ac.is_writable is not sufficient here.
            // An instruction can have the same account listed twice with different is_writable
            // values.
            // The instruction is allowed to modify the account if the account has been listed
            // as writable atleast once.
            if !writable_accounts.contains(&ac.pubkey) && ac_before != ac_after {
                let ac_name = get_account_name(&ac.pubkey).unwrap_or_default();
                bail!("Read only account modified: {} {}", ac_name, ac.pubkey);
            }
        }
    }
    mem::remove_closed_accounts();
    mem::commit_txn();
    Ok(())
}

pub fn set_account_names(account_names: HashMap<Pubkey, String>) {
    ACCOUNT_NAMES.with(|x| {
        *x.borrow_mut() = account_names;
    })
}

pub fn set_errors(errors: HashMap<Pubkey, ErrorFn>) {
    ERRORS.with(|e| {
        *e.borrow_mut() = errors;
    })
}

pub fn set_processors(processors: HashMap<Pubkey, ProcessorFn>) {
    PROCESSORS.with(|p| {
        *p.borrow_mut() = processors;
    })
}

// Helpers

fn invoke_unchecked(ixn: &Instruction, account_infos: &[AccountInfo]) -> ProgramResult {
    let last_program_id = CURRENT_PROGRAM_ID.with(|current_program_id| {
        let mut c = current_program_id.borrow_mut();
        let last_program_id = *c;
        *c = Some(ixn.program_id);
        last_program_id
    });
    let res = if ixn.program_id == solana_program::system_program::ID {
        system_program::emulate(account_infos, &ixn.data)
    } else if ixn.program_id == spl_token::ID {
        spl_token::processor::Processor::process(&ixn.program_id, account_infos, &ixn.data)
    } else if ixn.program_id == spl_associated_token_account::ID {
        spl_associated_token_account::processor::process_instruction(&ixn.program_id, account_infos, &ixn.data)
    } else {
        PROCESSORS.with(|processors| {
            let processors_ref = processors.borrow();
            let processor = processors_ref.get(&ixn.program_id);
            match processor {
                Some(processor) => processor(&ixn.program_id, &account_infos, &ixn.data),
                None => {
                    println!("Invalid program called: {}", ixn.program_id);
                    Err(ProgramError::Custom(999999))
                }
            }
        })
    };
    CURRENT_PROGRAM_ID.with(|current_program_id| {
        let mut c = current_program_id.borrow_mut();
        *c = last_program_id;
    });
    res
}

fn get_account_name(pubkey: &Pubkey) -> Option<String> {
    ACCOUNT_NAMES.with(|account_names| account_names.borrow().get(pubkey).cloned())
}

fn get_current_program_id() -> Pubkey {
    CURRENT_PROGRAM_ID.with(|c| c.borrow().clone().unwrap())
}

fn get_error(program_id: &Pubkey, error_code: u32) -> String {
    ERRORS.with(|errors| {
        let errors_ref = errors.borrow();
        let error_fn = errors_ref.get(program_id);
        match error_fn {
            None => format!("Unknown program ({}) error: {}", program_id, error_code),
            Some(error_fn) => error_fn(error_code)
                .map(|e| e.to_string())
                .unwrap_or_else(|| String::new()),
        }
    })
}

fn program_error_to_anyhow(ixn_idx: usize, program_id: &Pubkey, e: ProgramError) -> anyhow::Error {
    match e {
        ProgramError::Custom(err_code) => anyhow!(
            "Instruction [{}] Custom Program Error: {}",
            ixn_idx,
            get_error(program_id, err_code)
        ),
        program_error => anyhow!("Instruction [{}] Program Error: {}", ixn_idx, program_error),
    }
}

fn check_signers(
    instruction_idx: usize,
    accounts: &[AccountMeta],
    signers: &[&Keypair],
    payer: &Keypair,
) -> Result<()> {
    for ac in accounts {
        if ac.is_signer && ac.pubkey != payer.pubkey() && signers.iter().find(|kp| kp.pubkey() == ac.pubkey).is_none() {
            return Err(anyhow!(
                "Instruction [{}]: {} is declared to be a signer, but keypair not provided",
                instruction_idx,
                ac.pubkey
            ));
        }
    }
    Ok(())
}
