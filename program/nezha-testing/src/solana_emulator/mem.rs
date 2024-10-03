use std::{
    cell::{RefCell, UnsafeCell},
    sync::{Arc, Once},
};

use solana_program::{
    clock::Clock, program_option::COption, program_pack::Pack, pubkey::Pubkey, rent::Rent, system_program, sysvar,
};

thread_local! {
    pub static MEM: RefCell<Mem> = RefCell::new(Mem { accounts: Vec::new() });
    pub static MEM_TXN: RefCell<Mem> = RefCell::new(Mem { accounts: Vec::new() });
    static MEM_INIT: Once = Once::new();
}

pub struct Mem {
    accounts: Vec<(Pubkey, Arc<Account>)>,
}

impl Clone for Mem {
    fn clone(&self) -> Self {
        Self {
            accounts: self
                .accounts
                .iter()
                .map(|(pk, ac)| (*pk, Arc::new(Account::clone(ac))))
                .collect(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Account {
    pub lamports: UnsafeCell<u64>,
    pub data: UnsafeCell<Vec<u8>>,
    pub owner: UnsafeCell<Pubkey>,
}

impl Clone for Account {
    fn clone(&self) -> Self {
        unsafe {
            Self {
                lamports: UnsafeCell::new((*self.lamports.get()).clone()),
                data: UnsafeCell::new((*self.data.get()).clone()),
                owner: UnsafeCell::new((*self.owner.get()).clone()),
            }
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct AccountStatic {
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: Pubkey,
}

impl From<&Account> for AccountStatic {
    fn from(ac: &Account) -> Self {
        AccountStatic {
            lamports: unsafe { (*ac.lamports.get()).clone() },
            data: unsafe { (*ac.data.get()).clone() },
            owner: unsafe { (*ac.owner.get()).clone() },
        }
    }
}

fn uc<T>(x: T) -> UnsafeCell<T> {
    UnsafeCell::new(x)
}

pub fn begin_txn() {
    MEM.with(|mem| {
        MEM_TXN.with(|mem_txn| {
            let mem = mem.borrow();
            let mut mem_txn = mem_txn.borrow_mut();
            *mem_txn = mem.clone();
        })
    })
}

pub fn commit_txn() {
    MEM.with(|mem| {
        MEM_TXN.with(|mem_txn| {
            let mut mem = mem.borrow_mut();
            let mem_txn = mem_txn.borrow();
            *mem = mem_txn.clone();
        })
    })
}

pub fn init() {
    MEM_INIT.with(|once| once.call_once(|| _init()));
}

fn _init() {
    MEM.with(|s| {
        let mut s = s.borrow_mut();
        let clock = get_clock();
        let clock_bytes = bincode::serialize(&clock).unwrap();
        let account = Account {
            lamports: uc(1),
            data: uc(clock_bytes),
            owner: uc(system_program::id()),
        };
        s.accounts.push((sysvar::clock::id(), Arc::new(account)));

        let rent = RENT;
        let rent_bytes = bincode::serialize(&rent).unwrap();
        let account = Account {
            lamports: uc(1),
            data: uc(rent_bytes),
            owner: uc(system_program::id()),
        };
        s.accounts.push((sysvar::rent::id(), Arc::new(account)));

        let native_mint = spl_token::state::Mint {
            mint_authority: COption::None,
            supply: 0,
            decimals: 0,
            is_initialized: true,
            freeze_authority: COption::None,
        };
        let mut native_mint_bytes = vec![0; spl_token::state::Mint::LEN];
        Pack::pack(native_mint, &mut native_mint_bytes).unwrap();
        let account = Account {
            lamports: uc(1),
            data: uc(native_mint_bytes),
            owner: uc(spl_token::id()),
        };
        s.accounts.push((spl_token::native_mint::ID, Arc::new(account)));
    });
}

/// Create a new empty account
pub fn new_account(pubkey: Pubkey, space: usize, lamports: u64, owner: Pubkey) {
    new_account_with_data(pubkey, &vec![0; space], lamports, owner)
}

/// Create a new account with given data
pub fn new_account_with_data(pubkey: Pubkey, data: &[u8], lamports: u64, owner: Pubkey) {
    let ac = Account {
        lamports: uc(lamports),
        data: uc(data.to_vec()),
        owner: uc(owner),
    };
    MEM_TXN.with(|mem| mem.borrow_mut().accounts.push((pubkey, Arc::new(ac))));
}

pub fn get_account(pubkey: &Pubkey) -> Option<Arc<Account>> {
    MEM_TXN.with(|mem| {
        mem.borrow_mut()
            .accounts
            .iter()
            .rev()
            .find(|(pk, _)| pk == pubkey)
            .map(|(_, ac)| ac.clone())
    })
}

pub fn remove_closed_accounts() {
    MEM_TXN.with(|mem| {
        mem.borrow_mut()
            .accounts
            .retain(|(_, ac)| (unsafe { *ac.lamports.get() != 0 }))
    })
}

pub const RENT: Rent = Rent {
    lamports_per_byte_year: 1,
    exemption_threshold: 1.0,
    burn_percent: 0,
};

pub fn get_clock() -> Clock {
    Clock {
        slot: 1,
        epoch_start_timestamp: 1,
        epoch: 1,
        leader_schedule_epoch: 1,
        unix_timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as _,
    }
}
