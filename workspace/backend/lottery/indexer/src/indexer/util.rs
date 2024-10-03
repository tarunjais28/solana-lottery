use std::sync::Arc;

use anyhow::Result;
use log::{error, info};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig, instruction::Instruction, pubkey::Pubkey, signature::Signature,
    signer::keypair::Keypair, transaction::Transaction,
};

#[derive(Clone)]
pub struct SolanaProgramContext {
    pub rpc_client: Arc<RpcClient>,
    pub staking_program_id: Pubkey,
    pub usdc_mint_pubkey: Pubkey,
    pub admin_keypair: Arc<Keypair>,
    pub investor_keypair: Arc<Keypair>,
}

impl SolanaProgramContext {
    pub fn new(
        rpc_client: Arc<RpcClient>,
        staking_program_id: Pubkey,
        usdc_mint_pubkey: Pubkey,
        admin_keypair: Arc<Keypair>,
        investor_keypair: Arc<Keypair>,
    ) -> Self {
        Self {
            rpc_client,
            staking_program_id,
            usdc_mint_pubkey,
            admin_keypair,
            investor_keypair,
        }
    }
}

pub async fn send_and_confirm_transaction(
    rpc_client: &RpcClient,
    instruction: Instruction,
    signer: &Keypair,
    payer: &Pubkey,
) -> Result<Signature> {
    let mut transaction = Transaction::new_with_payer(&[instruction.clone()], Some(&payer));

    let hash = rpc_client.get_latest_blockhash().await?;

    transaction.sign(&[signer], hash);

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
