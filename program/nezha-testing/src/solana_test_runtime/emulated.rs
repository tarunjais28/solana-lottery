use anyhow::Result;
use async_trait::async_trait;
use solana_program::{bpf_loader_upgradeable, instruction::Instruction, pubkey::Pubkey, rent::Rent, system_program};
use solana_sdk::{signature::Keypair, signer::Signer};

use crate::solana_emulator;

use super::{Account, SolanaTestRuntime};

pub struct EmulatedTestRuntime {
    payer: Keypair,
}

impl EmulatedTestRuntime {
    pub fn new(program_ids: &[Pubkey]) -> Self {
        let payer = Keypair::new();
        let payer_pubkey = payer.pubkey();
        solana_emulator::begin_txn();
        solana_emulator::new_account(payer_pubkey, 0, u64::MAX / 2, system_program::id());
        for program_id in program_ids {
            solana_emulator::new_account(*program_id, 0, u64::MAX / 2, bpf_loader_upgradeable::id());
        }
        solana_emulator::commit_txn();
        Self { payer }
    }
}

#[async_trait]
impl SolanaTestRuntime for EmulatedTestRuntime {
    async fn send_ixns(&mut self, ixns: &[Instruction], signers: &[&Keypair]) -> Result<()> {
        solana_emulator::invoke(ixns, signers, &self.payer)
    }
    async fn get_rent(&mut self) -> Result<Rent> {
        Ok(solana_emulator::RENT)
    }
    fn get_payer(&mut self) -> &Keypair {
        &self.payer
    }
    async fn get_account(&mut self, account: Pubkey) -> Result<Option<Account>> {
        let ac_mem = solana_emulator::get_account(&account);

        Ok(ac_mem.map(|ac| {
            let mut ac_ = Account::default();
            unsafe {
                std::ptr::copy(ac.owner.get(), &mut ac_.owner as *mut _, 1);
                std::ptr::copy(ac.lamports.get(), &mut ac_.lamports as *mut _, 1);
                ac_.data.extend_from_slice(&*ac.data.get());
            };
            ac_
        }))
    }

    fn set_account(&mut self, address: &Pubkey, account: &Account) {
        solana_emulator::begin_txn();
        solana_emulator::new_account_with_data(*address, &account.data, account.lamports, account.owner);
        solana_emulator::commit_txn();
    }
}
