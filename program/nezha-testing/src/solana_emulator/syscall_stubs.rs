use std::{collections::HashSet, sync::Once};

use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, instruction::Instruction, program_stubs::SyscallStubs,
    pubkey::Pubkey,
};
use std::cell::RefCell;

thread_local! {
    static RETURN_DATA: RefCell<Option<(Pubkey, Vec<u8>)>> = RefCell::new(None);
}

static INIT: Once = Once::new();

pub fn init() {
    INIT.call_once(|| {
        solana_program::program_stubs::set_syscall_stubs(Box::new(SyscallStubsImpl));
    });
}

pub struct SyscallStubsImpl;

impl SyscallStubs for SyscallStubsImpl {
    fn sol_invoke_signed(
        &self,
        ixn: &Instruction,
        account_infos: &[AccountInfo],
        signers_seeds: &[&[&[u8]]],
    ) -> ProgramResult {
        let current_program_id = super::get_current_program_id();
        let signed_accounts: HashSet<Pubkey> = signers_seeds
            .iter()
            .map(|s| {
                let (pubkey, _bump) = Pubkey::find_program_address(
                    &{
                        let mut x = s.to_vec();
                        x.pop();
                        x
                    },
                    &current_program_id,
                );
                pubkey
            })
            .collect();

        let ordered_accounts: Vec<AccountInfo> = ixn
            .accounts
            .iter()
            .map(|ac| {
                account_infos
                    .iter()
                    .find(|ac_info| *ac_info.key == ac.pubkey)
                    .expect(&format!(
                        "Can't find account: {} {}",
                        ac.pubkey,
                        super::get_account_name(&ac.pubkey).unwrap_or_default(),
                    ))
                    .clone()
            })
            .map(|ac| {
                if signed_accounts.contains(ac.key) {
                    AccountInfo { is_signer: true, ..ac }
                } else {
                    ac
                }
            })
            .collect();
        super::invoke_unchecked(ixn, &ordered_accounts)
    }

    fn sol_get_clock_sysvar(&self, _var_addr: *mut u8) -> u64 {
        let clock = super::mem::get_clock();
        let clock_bytes = bincode::serialize(&clock).unwrap();
        let target_slice = unsafe { std::slice::from_raw_parts_mut(_var_addr, clock_bytes.len()) };
        target_slice.copy_from_slice(&clock_bytes);
        0
    }

    fn sol_get_epoch_schedule_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }

    fn sol_get_fees_sysvar(&self, _var_addr: *mut u8) -> u64 {
        0
    }

    fn sol_get_rent_sysvar(&self, _var_addr: *mut u8) -> u64 {
        let rent_bytes = bincode::serialize(&super::mem::RENT).unwrap();
        let target_slice = unsafe { std::slice::from_raw_parts_mut(_var_addr, rent_bytes.len()) };
        target_slice.copy_from_slice(&rent_bytes);
        0
    }

    fn sol_get_return_data(&self) -> Option<(Pubkey, Vec<u8>)> {
        RETURN_DATA.with(|r| r.borrow().clone())
    }

    fn sol_set_return_data(&self, data: &[u8]) {
        let current_program_id = super::get_current_program_id();
        RETURN_DATA.with(|r| *r.borrow_mut() = Some((current_program_id, data.to_vec())));
    }
}
