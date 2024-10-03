use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use solana_program::bpf_loader_upgradeable;
use solana_program::instruction::InstructionError;
use solana_program::{instruction::Instruction, pubkey::Pubkey, rent::Rent};
use solana_program_test::{BanksClientError, ProgramTest, ProgramTestBanksClientExt, ProgramTestContext};
use solana_sdk::account::AccountSharedData;
use solana_sdk::transaction::TransactionError;
use solana_sdk::{signature::Keypair, signer::Signer, transaction::Transaction};

use super::{Account, ErrorFn, SolanaTestRuntime};

pub struct BPFTestRuntime {
    ptc: ProgramTestContext,
    /// Program ID -> ErrorFn
    errors: HashMap<Pubkey, ErrorFn>,
}

impl BPFTestRuntime {
    pub async fn new(
        program_name: &str,
        program_id: &Pubkey,
        errors: HashMap<Pubkey, ErrorFn>,
        program_ids: &[Pubkey],
    ) -> Result<Self> {
        let mut program_test = ProgramTest::new(program_name, *program_id, None);
        program_test.set_compute_max_units(250_000);
        let ptc = program_test.start_with_context().await;
        let mut self_ = Self { ptc, errors };
        for program_id_ in program_ids {
            if program_id_ == program_id {
                continue;
            }
            self_.set_account(
                program_id_,
                &Account {
                    lamports: 1,
                    owner: bpf_loader_upgradeable::id(),
                    data: vec![],
                },
            )
        }
        Ok(self_)
    }
}

#[async_trait]
impl SolanaTestRuntime for BPFTestRuntime {
    async fn send_ixns(&mut self, ixns: &[Instruction], signers: &[&Keypair]) -> Result<()> {
        let mut signers: Vec<_> = signers.into();
        signers.push(&self.ptc.payer);
        let last_blockhash = self.ptc.banks_client.get_latest_blockhash().await?;
        let blockhash = self.ptc.banks_client.get_new_latest_blockhash(&last_blockhash).await?;
        let tx = Transaction::new_signed_with_payer(ixns, Some(&self.ptc.payer.pubkey()), &signers, blockhash);

        let res = self.ptc.banks_client.process_transaction(tx).await;

        if let Err(e) = res {
            let err = match e {
                BanksClientError::TransactionError(txn_error)
                | BanksClientError::SimulationError { err: txn_error, .. } => {
                    transaction_error_to_anyhow(ixns, &self.errors, txn_error)
                }
                _ => anyhow::Error::from(e),
            };
            return Err(err);
        }

        Ok(())
    }

    async fn get_rent(&mut self) -> Result<Rent> {
        self.ptc
            .banks_client
            .get_rent()
            .await
            .with_context(|| "Couldn't get rent sysvar from ptc")
    }

    fn get_payer(&mut self) -> &Keypair {
        &self.ptc.payer
    }

    async fn get_account(&mut self, account: Pubkey) -> Result<Option<Account>> {
        self.ptc
            .banks_client
            .get_account(account)
            .await
            .map(|ac| {
                ac.map(|ac| Account {
                    owner: ac.owner,
                    data: ac.data,
                    lamports: ac.lamports,
                })
            })
            .with_context(|| format!("Failed to look up account: {account}"))
    }

    fn set_account(&mut self, address: &Pubkey, account: &Account) {
        let mut account_shared_data = AccountSharedData::new(account.lamports, account.data.len(), &account.owner);
        account_shared_data.set_data(account.data.clone());
        self.ptc.set_account(address, &account_shared_data)
    }
}

fn get_error(errors: &HashMap<Pubkey, ErrorFn>, program_id: &Pubkey, error_code: u32) -> String {
    let error_fn = errors.get(program_id);
    match error_fn {
        None => format!("Unknown program ({}) error: {}", program_id, error_code),
        Some(error_fn) => error_fn(error_code)
            .map(|e| e.to_string())
            .unwrap_or_else(|| String::new()),
    }
}

fn instruction_error_to_anyhow(
    ixn_idx: usize,
    program_id: &Pubkey,
    e: InstructionError,
    errors: &HashMap<Pubkey, ErrorFn>,
) -> anyhow::Error {
    match e {
        InstructionError::Custom(err_code) => anyhow!(
            "Instruction [{}] Custom Program Error: {}",
            ixn_idx,
            get_error(errors, program_id, err_code)
        ),
        program_error => anyhow!("Instruction [{}] Program Error: {}", ixn_idx, program_error),
    }
}

fn transaction_error_to_anyhow(
    ixns: &[Instruction],
    errors: &HashMap<Pubkey, ErrorFn>,
    txn_error: TransactionError,
) -> anyhow::Error {
    if let TransactionError::InstructionError(idx, ix_error) = txn_error {
        instruction_error_to_anyhow(idx as usize, &ixns[idx as usize].program_id, ix_error, errors)
    } else {
        anyhow!("TransactionError: {}", txn_error)
    }
}
