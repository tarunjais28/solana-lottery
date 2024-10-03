use std::sync::RwLock;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use service::{epoch::FPUSDC, model::prize::Prize, prize::PrizeRepository};
use solana_sdk::pubkey::Pubkey;

pub struct InMemoryPrizeRepository {
    mem: RwLock<Vec<Prize>>,
}

#[async_trait]
impl PrizeRepository for InMemoryPrizeRepository {
    async fn by_wallet(&self, wallet: &Pubkey, limit: usize, offset: usize) -> Result<Vec<Prize>> {
        let mem = self.mem.read().unwrap();
        Ok(mem
            .iter()
            .filter(|prize| prize.wallet == *wallet)
            .skip(offset * limit)
            .take(limit)
            .cloned()
            .collect())
    }

    async fn by_wallet_epoch_and_tier(&self, wallet: &Pubkey, epoch_index: u64, tier: u8) -> Result<Option<Prize>> {
        let mem = self.mem.read().unwrap();
        Ok(mem
            .iter()
            .find(|prize| prize.wallet == *wallet && prize.epoch_index == epoch_index && prize.tier == tier)
            .cloned())
    }

    async fn total_prize_by_wallet(&self, wallet: &Pubkey) -> Result<FPUSDC> {
        let mem = self.mem.read().unwrap();
        Ok(mem
            .iter()
            .filter(|prize| prize.wallet == *wallet)
            .try_fold(FPUSDC::zero(), |acc, prize| {
                acc.checked_add(prize.amount)
                    .ok_or_else(|| anyhow!("Error while calculating total prize amount"))
            })?)
    }

    async fn upsert_prizes(&self, prizes: &[Prize]) -> Result<()> {
        let mut mem = self.mem.write().unwrap();
        mem.extend_from_slice(prizes);
        Ok(())
    }
}
