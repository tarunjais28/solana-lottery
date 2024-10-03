use anyhow::Result;
use async_trait::async_trait;
use nezha_staking::fixed_point::FPUSDC;
use solana_sdk::pubkey::Pubkey;

use crate::model::prize::Prize;

#[async_trait]
pub trait PrizeRepository: Send + Sync {
    async fn by_wallet(&self, wallet: &Pubkey, limit: usize, offset: usize) -> Result<Vec<Prize>>;
    async fn by_wallet_epoch_and_tier(&self, wallet: &Pubkey, epoch_index: u64, tier: u8) -> Result<Option<Prize>>;
    async fn total_prize_by_wallet(&self, wallet: &Pubkey) -> Result<FPUSDC>;
    async fn upsert_prizes(&self, prizes: &[Prize]) -> Result<()>;
}

#[async_trait]
pub trait PrizeService: Send + Sync {
    async fn by_wallet(&self, wallet: &Pubkey, limit: usize, offset: usize) -> Result<Vec<Prize>>;
    async fn by_wallet_epoch_and_tier(&self, wallet: &Pubkey, epoch_index: u64, tier: u8) -> Result<Option<Prize>>;
    async fn total_prize_by_wallet(&self, wallet: &Pubkey) -> Result<FPUSDC>;
}

pub struct PrizeServiceImpl {
    repository: Box<dyn PrizeRepository>,
}

impl PrizeServiceImpl {
    pub fn new(repository: Box<dyn PrizeRepository>) -> Self {
        Self { repository }
    }
}

#[async_trait]
impl PrizeService for PrizeServiceImpl {
    async fn by_wallet(&self, wallet: &Pubkey, limit: usize, offset: usize) -> Result<Vec<Prize>> {
        self.repository.by_wallet(wallet, limit, offset).await
    }

    async fn by_wallet_epoch_and_tier(&self, wallet: &Pubkey, epoch_index: u64, tier: u8) -> Result<Option<Prize>> {
        self.repository
            .by_wallet_epoch_and_tier(wallet, epoch_index, tier)
            .await
    }

    async fn total_prize_by_wallet(&self, wallet: &Pubkey) -> Result<FPUSDC> {
        self.repository.total_prize_by_wallet(wallet).await
    }
}
