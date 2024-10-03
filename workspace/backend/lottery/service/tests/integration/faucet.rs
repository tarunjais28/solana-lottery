use std::sync::RwLock;

use crate::common;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use service::{
    faucet::{FaucetRepository, FaucetService, SolanaFaucetService},
    model::{faucet::LatestMintTransaction, transaction::TransactionId},
    solana::Solana,
};
use solana_sdk::pubkey::Pubkey;
use tokio::time::sleep;

#[tokio::test]
async fn test_mint_usdc() -> Result<()> {
    let ctx = common::setup_solana().await;
    let solana = ctx.solana;

    let faucet_repository = InMemoryFaucetRepository::default();
    let expected_amount = "1.0".parse().unwrap();
    let retry_time_limit = Duration::seconds(1);
    let faucet_service = SolanaFaucetService::new(
        retry_time_limit.clone(),
        expected_amount,
        Box::new(faucet_repository),
        Box::new(solana.clone()),
        true,
    );
    let wallet = Pubkey::new_unique();
    let old_balance = solana.get_usdc_balance_by_wallet(wallet).await?;
    let tx = faucet_service.mint_devnet_usdc(&wallet).await?;
    assert_eq!(expected_amount.as_usdc(), tx.amount);
    let new_balance = solana.get_usdc_balance_by_wallet(wallet).await?;
    assert_eq!(new_balance.checked_sub(old_balance).unwrap(), expected_amount);

    sleep(retry_time_limit.to_std()?).await;
    let res = faucet_service.mint_devnet_usdc(&wallet).await;
    assert!(res.is_ok());
    Ok(())
}

#[tokio::test]
async fn test_mint_usdc_limit_exceeded() -> Result<()> {
    let ctx = common::setup_solana().await;
    let solana = ctx.solana;

    let faucet_repository = InMemoryFaucetRepository::default();
    let amount = "1.0".parse().unwrap();
    let faucet_service = SolanaFaucetService::new(
        Duration::days(1),
        amount,
        Box::new(faucet_repository),
        Box::new(solana),
        true,
    );
    let wallet = Pubkey::new_unique();
    faucet_service.mint_devnet_usdc(&wallet).await?;
    let res = faucet_service.mint_devnet_usdc(&wallet).await;
    assert!(res.is_err(), "{res:?}");
    Ok(())
}

#[derive(Default)]
pub struct InMemoryFaucetRepository {
    mem: RwLock<Vec<LatestMintTransaction>>,
}

#[async_trait]
impl FaucetRepository for InMemoryFaucetRepository {
    async fn latest_transaction(&self, wallet: &Pubkey) -> Result<Option<LatestMintTransaction>> {
        Ok(self
            .mem
            .read()
            .unwrap()
            .iter()
            .find(|v| &v.wallet == wallet)
            .map(|v| v.clone()))
    }

    async fn store_latest_transaction(
        &self,
        wallet: &Pubkey,
        amount: u64,
        transaction_time: &DateTime<Utc>,
        transaction_id: &TransactionId,
    ) -> Result<LatestMintTransaction> {
        self.mem.write().unwrap().push(LatestMintTransaction {
            wallet: wallet.clone(),
            amount,
            transaction_time: transaction_time.clone(),
            transaction_id: transaction_id.clone(),
        });
        Ok(LatestMintTransaction {
            wallet: wallet.clone(),
            amount,
            transaction_time: transaction_time.clone(),
            transaction_id: transaction_id.clone(),
        })
    }
}
