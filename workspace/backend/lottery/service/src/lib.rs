use anyhow::Result;
use log::{error, info};
use model::error::decode_staking_error;
use nezha_staking::error::StakingError;
use solana_client::{client_error::ClientError, nonblocking::rpc_client::RpcClient};
use solana_program::instruction::Instruction;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use thiserror::Error;

pub mod epoch;
pub mod faucet;
pub mod health_check;
pub mod model;
pub mod prize;
pub mod rng;
pub mod solana;
pub mod stake;
pub mod tickets;
pub mod transaction;
pub mod util;

#[derive(Error, Debug)]
pub enum SolanaError {
    #[error("Staking program error: {0}")]
    StakingProgramError(StakingError),
    #[error("Unexpected RPC client error: {0}")]
    OtherRpcError(ClientError),
}

impl From<ClientError> for SolanaError {
    fn from(client_error: ClientError) -> Self {
        match decode_staking_error(&client_error) {
            Some(e) => SolanaError::StakingProgramError(e),
            None => SolanaError::OtherRpcError(client_error),
        }
    }
}

pub async fn send_and_confirm_transaction(
    rpc_client: &RpcClient,
    signer: &Keypair,
    instruction: Instruction,
) -> Result<Signature> {
    send_and_confirm_transaction2(rpc_client, signer, &[instruction]).await
}

/// Middleware for [`send_and_confirm_transaction`]. Logs transaction ID.
///
/// [`send_and_confirm_transaction`]: solana_client::nonblocking::rpc_client::RpcClient::send_and_confirm_transaction
pub async fn send_and_confirm_transaction2(
    rpc_client: &RpcClient,
    signer: &Keypair,
    instructions: &[Instruction],
) -> Result<Signature> {
    let mut transaction = Transaction::new_with_payer(instructions, Some(&signer.pubkey()));

    let hash = rpc_client.get_latest_blockhash().await?;

    transaction.sign(&[signer], hash);

    let res = rpc_client.simulate_transaction(&transaction).await?;
    if let Some(err) = res.value.err {
        error!("Simulate transaction error: {}", err);
        info!("Logs: {:#?}", res.value.logs.unwrap_or_default());
        anyhow::bail!("Simulate transaction error: {}", err);
    }

    // send_and_confirm_transaction() hammers the RPC every 500ms.
    // We are using get_signature_statuses() separately to customize the delay.
    // We can't use confirm_transaction() because it doesn't indicate if a transaction failed.
    let signature = rpc_client.send_transaction(&transaction).await?;
    info!("Confirming txn {}", signature);
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
        let status = &rpc_client.get_signature_statuses(&[signature]).await?.value[0];
        match status {
            Some(status) => {
                if let Some(err) = &status.err {
                    log::error!("Error confirming txn: {}", err);
                    anyhow::bail!("Error confirming txn {}: {}", signature, err);
                }

                if status.satisfies_commitment(CommitmentConfig::finalized()) {
                    info!("Finalized txn {}", signature);
                    return Ok(signature);
                } else {
                    info!("Confirmations: {}", status.confirmations.unwrap_or_default());
                }
            }
            None => {
                let blockhash_not_found = !rpc_client
                    .is_blockhash_valid(&hash, CommitmentConfig::processed())
                    .await?;
                if blockhash_not_found {
                    error!("Blockhash expired, transaction not found {}", signature);
                    anyhow::bail!("Blockhash expired, transaction not found {}", signature);
                } else {
                    info!("Txn not found yet {}", signature);
                    continue;
                }
            }
        };
    }
}
