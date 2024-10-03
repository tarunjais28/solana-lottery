use solana_program::rent::Rent;
use solana_program::system_instruction;
use solana_sdk::signer::Signer;
use std::collections::BTreeSet;
use tokio::sync::Mutex;

use async_trait::async_trait;
use borsh::BorshSerialize;
use nezha_staking::state::AccountType;
use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_program_test::{ProgramTest, ProgramTestBanksClientExt, ProgramTestContext};
use solana_sdk::account::Account as SolanaAccount;
use solana_sdk::{
    signature::{Keypair, Signature},
    transaction::Transaction,
};

use crate::solana::TransactionErrorParsed;
use crate::solana::{with_pubkey::WithPubkey, SolanaError, ToSolanaError};

use super::SolanaRpc;

pub struct SolanaRpcTest {
    staking_program_id: Pubkey,
    vrf_program_id: Pubkey,
    ptc: Mutex<ProgramTestContext>,
    pubkeys: Mutex<BTreeSet<Pubkey>>,
}

impl SolanaRpcTest {
    pub async fn new(staking_program_id: Pubkey, vrf_program_id: Pubkey) -> SolanaRpcTest {
        let mut program_test = ProgramTest::new("nezha_staking", staking_program_id, None);
        program_test.add_program("nezha_vrf_mock", vrf_program_id, None);
        program_test.set_compute_max_units(400_000);

        let ptc = program_test.start_with_context().await;

        Self {
            staking_program_id,
            ptc: Mutex::new(ptc),
            pubkeys: Default::default(),
            vrf_program_id,
        }
    }

    pub async fn mint_sols(&self, account: Pubkey, lamports: u64) {
        let payer = self.ptc.lock().await.payer.pubkey();
        let instructions = [system_instruction::transfer(&payer, &account, lamports)];
        self._send_and_confirm_transaction(&[], None, &instructions)
            .await
            .unwrap();
    }

    pub async fn get_rent(&self) -> Rent {
        let mut ptc = self.ptc.lock().await;
        let rent = ptc.banks_client.get_rent().await.unwrap();
        rent
    }
}

#[async_trait]
impl SolanaRpc for SolanaRpcTest {
    async fn _send_and_confirm_transaction(
        &self,
        signers: &[&Keypair],
        _payer: Option<&Pubkey>,
        instructions: &[Instruction],
    ) -> Result<Signature, SolanaError> {
        let mut ptc_lock = self.ptc.lock().await;

        let ptc: &mut ProgramTestContext = &mut ptc_lock;

        let mut signers: Vec<_> = signers.into();
        signers.push(&ptc.payer);

        let last_blockhash = ptc
            .banks_client
            .get_latest_blockhash()
            .await
            .with_context(|| "Failed to get latest blockhash")?;

        let blockhash = ptc
            .banks_client
            .get_new_latest_blockhash(&last_blockhash)
            .await
            .with_context(|| "Failed to advance blockhash")?;

        let transaction =
            Transaction::new_signed_with_payer(instructions, Some(&ptc.payer.pubkey()), &signers, blockhash);

        let signature = transaction.signatures.first().unwrap().clone();

        match ptc.banks_client.process_transaction(transaction).await {
            Ok(()) => {
                for ix in instructions {
                    self.pubkeys.lock().await.extend(ix.accounts.iter().map(|ac| ac.pubkey));
                }
                Ok(signature)
            }
            Err(e) => match e {
                solana_program_test::BanksClientError::TransactionError(err) => {
                    let err_parsed =
                        TransactionErrorParsed::from_instructions(&instructions, self.staking_program_id, &err);
                    Err(SolanaError::TransactionFailedToConfirm {
                        signature,
                        error_parsed: err_parsed,
                        error: err,
                    })
                }
                solana_program_test::BanksClientError::SimulationError { err, logs, .. } => {
                    let err_parsed =
                        TransactionErrorParsed::from_instructions(&instructions, self.staking_program_id, &err);
                    Err(SolanaError::TransactionSimulationFailed {
                        error: err,
                        error_parsed: err_parsed,
                        logs,
                    })
                }
                err => Err(err).context("Failed to process transaction"),
            },
        }
    }

    async fn get_program_accounts_by_type(
        &self,
        program_id: &Pubkey,
        account_type: AccountType,
    ) -> Result<Vec<WithPubkey<SolanaAccount>>, SolanaError> {
        let mut res = Vec::new();

        let account_type_bytes = account_type
            .try_to_vec()
            .with_context(|| format!("Failed to encode {:#?} into bytes", account_type))?;

        let mut ptc = self.ptc.lock().await;
        for pk in self.pubkeys.lock().await.iter() {
            let ac = ptc
                .banks_client
                .get_account(*pk)
                .await
                .context("Failed to call get_account()")?;
            let ac = match ac {
                None => continue,
                Some(ac) => ac,
            };
            if ac.owner != *program_id {
                continue;
            }
            if !ac.data.starts_with(&account_type_bytes) {
                continue;
            }

            res.push(WithPubkey { pubkey: *pk, inner: ac });
        }
        Ok(res)
    }

    async fn get_multiple_accounts(
        &self,
        pks: &[Pubkey],
    ) -> Result<Vec<Option<WithPubkey<SolanaAccount>>>, SolanaError> {
        let mut res = Vec::new();
        let mut ptc = self.ptc.lock().await;
        for pk in pks {
            let ac = ptc
                .banks_client
                .get_account(*pk)
                .await
                .with_context(|| format!("Error getting account {}", pk))?;
            res.push(ac.map(|ac| WithPubkey { inner: ac, pubkey: *pk }));
        }
        Ok(res)
    }

    async fn request_airdrop(&self, pubkey: Pubkey, lamports: u64) -> Result<(), SolanaError> {
        let payer = Keypair::from_bytes(&self.ptc.lock().await.payer.to_bytes()).unwrap();
        let instructions = [system_instruction::transfer(&payer.pubkey(), &pubkey, lamports)];
        // payer signature is implicitly added by _send_and_confirm_transaction()
        self._send_and_confirm_transaction(&[], None, &instructions).await?;
        Ok(())
    }
}
