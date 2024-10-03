pub mod bpf;
pub mod emulated;

use std::collections::HashMap;
use std::fmt::Display;

use anyhow::Result;
use async_trait::async_trait;
use solana_program::{instruction::Instruction, pubkey::Pubkey, rent::Rent};
use solana_sdk::signature::Keypair;

use crate::solana_emulator::{self, ProcessorFn};

/// Function that converts error code to corresponding concrete error
pub type ErrorFn = fn(u32) -> Option<Box<dyn Display + Send + Sync + 'static>>;

#[derive(Clone, Default, Debug)]
pub struct Account {
    pub lamports: u64,
    pub owner: Pubkey,
    pub data: Vec<u8>,
}

#[async_trait]
pub trait SolanaTestRuntime {
    async fn send_ixns(&mut self, ixns: &[Instruction], signers: &[&Keypair]) -> Result<()>;
    async fn get_rent(&mut self) -> Result<Rent>;
    fn get_payer(&mut self) -> &Keypair;
    async fn get_account(&mut self, account: Pubkey) -> Result<Option<Account>>;
    fn set_account(&mut self, address: &Pubkey, account: &Account);
}

pub enum TestRuntimeType {
    Emulated {
        processors: HashMap<Pubkey, ProcessorFn>,
        account_names: HashMap<Pubkey, String>,
    },
    BPF {
        program_name: String,
        program_id: Pubkey,
    },
}

/// # Arguments:
/// - `extra_program_ids`: Will create dummy program accounts with BPFUpgradableLoader as the owner.
///   Some programs need this because they check the owner before CPI
pub async fn new_test_runtime(
    test_runtime_type: TestRuntimeType,
    errors: HashMap<Pubkey, ErrorFn>,
    extra_program_ids: &[Pubkey],
) -> Result<Box<dyn SolanaTestRuntime + Send + Sync + 'static>> {
    match test_runtime_type {
        TestRuntimeType::Emulated {
            processors,
            account_names,
        } => {
            solana_emulator::init();
            solana_emulator::set_account_names(account_names);
            solana_emulator::set_errors(errors.clone());
            solana_emulator::set_processors(processors);
            let runtime = emulated::EmulatedTestRuntime::new(&extra_program_ids);
            Ok(Box::new(runtime))
        }
        TestRuntimeType::BPF {
            program_name,
            program_id,
        } => {
            let runtime = bpf::BPFTestRuntime::new(&program_name, &program_id, errors, &extra_program_ids).await?;
            Ok(Box::new(runtime))
        }
    }
}
