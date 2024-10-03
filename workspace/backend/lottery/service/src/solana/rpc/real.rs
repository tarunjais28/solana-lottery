use async_trait::async_trait;
use borsh::BorshSerialize;
use nezha_staking::state::AccountType;
use solana_account_decoder::UiAccountEncoding;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType};
use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_sdk::account::Account as SolanaAccount;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Keypair;
use solana_sdk::{signature::Signature, transaction::Transaction};

use crate::solana::TransactionErrorParsed;
use crate::solana::{with_pubkey::WithPubkey, SolanaError, ToSolanaError};

pub struct SolanaRpcReal {
    staking_program_id: Pubkey,
    client: RpcClient,
}

impl SolanaRpcReal {
    pub fn new(client: RpcClient, staking_program_id: Pubkey) -> Self {
        Self {
            staking_program_id,
            client,
        }
    }
}

#[async_trait]
impl super::SolanaRpc for SolanaRpcReal {
    async fn _send_and_confirm_transaction(
        &self,
        signers: &[&Keypair],
        payer: Option<&Pubkey>,
        instructions: &[Instruction],
    ) -> Result<Signature, SolanaError> {
        let mut transaction = Transaction::new_with_payer(instructions, payer);

        let hash = self
            .client
            .get_latest_blockhash()
            .await
            .context("Failed to get latest blockhash")
            .map_err(|e| {
                log::warn!("{:?}", e);
                e
            })?;

        transaction
            .try_sign(&signers.to_owned(), hash)
            .context("Failed to sign transaction")
            .map_err(|e| {
                log::warn!("{:?}", e);
                e
            })?;

        let sim_resp = self
            .client
            .simulate_transaction(&transaction)
            .await
            .context("Failed to simulate transaction")
            .map_err(|e| {
                log::warn!("{:?}", e);
                e
            })?;

        if let Some(err) = sim_resp.value.err {
            let err_parsed = TransactionErrorParsed::from_instructions(&instructions, self.staking_program_id, &err);
            return Err(SolanaError::TransactionSimulationFailed {
                error: err,
                error_parsed: err_parsed,
                logs: sim_resp.value.logs.unwrap_or_default(),
            });
        }

        // send_and_confirm_transaction() hammers the RPC every 500ms.
        // We are using get_signature_statuses() separately to customize the delay.
        // We can't use confirm_transaction() because it doesn't indicate if a transaction failed.
        let signature = self
            .client
            .send_transaction(&transaction)
            .await
            .context("Failed to send transaction")?;
        log::info!("Confirming txn {}", signature);
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
            let status = &self
                .client
                .get_signature_statuses(&[signature])
                .await
                .context("Failed to get signature status")?
                .value[0];
            match status {
                Some(status) => {
                    if let Some(err) = &status.err {
                        let err_parsed =
                            TransactionErrorParsed::from_instructions(&instructions, self.staking_program_id, err);
                        log::error!("Error confirming txn: {}", err);
                        return Err(SolanaError::TransactionFailedToConfirm {
                            signature,
                            error_parsed: err_parsed,
                            error: err.clone(),
                        });
                    }

                    if status.satisfies_commitment(CommitmentConfig::finalized()) {
                        log::info!("Finalized txn {}", signature);
                        return Ok(signature);
                    } else {
                        log::info!("Confirmations: {}", status.confirmations.unwrap_or_default());
                    }
                }
                None => {
                    let blockhash_valid = self
                        .client
                        .is_blockhash_valid(&hash, CommitmentConfig::processed())
                        .await
                        .context("Failed to check if blockhash is valid")?;
                    if !blockhash_valid {
                        log::error!("Blockhash expired, transaction not found {}", signature);
                        return Err(SolanaError::TransactionBlockhashExpired { signature });
                    } else {
                        log::info!("Txn not found yet {}", signature);
                        continue;
                    }
                }
            };
        }
    }

    async fn get_program_accounts_by_type(
        &self,
        program_id: &Pubkey,
        account_type: AccountType,
    ) -> Result<Vec<WithPubkey<SolanaAccount>>, SolanaError> {
        let account_type_bytes = account_type
            .try_to_vec()
            .with_context(|| format!("Failed to encode {:#?} into bytes", account_type))?;
        let accounts = self
            .client
            .get_program_accounts_with_config(
                &program_id,
                RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::Memcmp(Memcmp {
                        offset: 0,
                        bytes: MemcmpEncodedBytes::Bytes(account_type_bytes),
                        encoding: None,
                    })]),
                    account_config: RpcAccountInfoConfig {
                        encoding: Some(UiAccountEncoding::Base64),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await
            .with_context(|| format!("Failed to get program accounts of type {:#?}", account_type))?;

        let iter = accounts
            .into_iter()
            .map(|(pk, acc)| WithPubkey { pubkey: pk, inner: acc });

        Ok(iter.collect())
    }

    async fn get_multiple_accounts(
        &self,
        pks: &[Pubkey],
    ) -> Result<Vec<Option<WithPubkey<SolanaAccount>>>, SolanaError> {
        let acs = self
            .client
            .get_multiple_accounts(pks)
            .await
            .context("Failed to get multiple accounts")?;

        let acs_with_pk = acs
            .into_iter()
            .zip(pks.iter())
            .map(|(ac, pk)| ac.map(|ac| WithPubkey { inner: ac, pubkey: *pk }))
            .collect();

        Ok(acs_with_pk)
    }

    async fn request_airdrop(&self, pubkey: Pubkey, lamports: u64) -> Result<(), SolanaError> {
        let sig = self
            .client
            .request_airdrop(&pubkey, lamports)
            .await
            .context("Failed to request airdrop")?;
        self.client
            .poll_for_signature(&sig)
            .await
            .context("Failed to confirm airdrop")?;
        Ok(())
    }
}
