use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use nezha_staking::fixed_point::FPUSDC;
use solana_sdk::pubkey::Pubkey;

use crate::{
    model::{
        faucet::{FaucetError, LatestMintTransaction},
        transaction::TransactionId,
    },
    solana::Solana,
};

#[async_trait]
pub trait FaucetRepository: Sync + Send {
    async fn latest_transaction(&self, wallet: &Pubkey) -> Result<Option<LatestMintTransaction>>;
    async fn store_latest_transaction(
        &self,
        wallet: &Pubkey,
        amount: u64,
        transaction_time: &DateTime<Utc>,
        transaction_id: &TransactionId,
    ) -> Result<LatestMintTransaction>;
}

#[async_trait]
pub trait FaucetService: Sync + Send {
    async fn mint_devnet_usdc(&self, wallet: &Pubkey) -> Result<LatestMintTransaction>;
}

pub struct SolanaFaucetService {
    retry_time_limit: Duration,
    amount: FPUSDC,
    faucet_repository: Box<dyn FaucetRepository>,
    solana: Box<dyn Solana>,
    enable_faucet: bool,
}

impl SolanaFaucetService {
    pub fn new(
        retry_time_limit: Duration,
        amount: FPUSDC,
        faucet_repository: Box<dyn FaucetRepository>,
        solana: Box<dyn Solana>,
        enable_faucet: bool,
    ) -> Self {
        Self {
            retry_time_limit,
            amount,
            faucet_repository,
            solana,
            enable_faucet,
        }
    }
}

#[async_trait]
impl FaucetService for SolanaFaucetService {
    async fn mint_devnet_usdc(&self, wallet: &Pubkey) -> Result<LatestMintTransaction> {
        if !self.enable_faucet {
            return Err(FaucetError::FaucetDisabled.into());
        }
        if let Some(latest_mint_transaction) = self.faucet_repository.latest_transaction(wallet).await? {
            let time_elapsed = Utc::now() - latest_mint_transaction.transaction_time;
            if time_elapsed < self.retry_time_limit {
                let remaining_time = self.retry_time_limit - time_elapsed;
                return Err(FaucetError::LimitReached {
                    hours: remaining_time.num_hours(),
                    minutes: remaining_time.num_minutes() % 60,
                    seconds: remaining_time.num_seconds() % 60,
                }
                .into());
            }
        }
        let transaction_id = self.solana.mint_usdc(*wallet, self.amount).await?.into();
        let latest_transaction = self
            .faucet_repository
            .store_latest_transaction(wallet, self.amount.as_usdc(), &Utc::now(), &transaction_id)
            .await?;

        Ok(latest_transaction)
    }
}
