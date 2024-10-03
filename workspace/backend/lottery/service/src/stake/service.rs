use anyhow::Result;
use async_trait::async_trait;
use nezha_staking::{fixed_point::FPUSDC, state::StakeUpdateRequest};

use crate::{
    model::{stake_update::StakeUpdate, transaction::TransactionId},
    solana::Stake,
};

#[async_trait]
pub trait StakeService: Sync + Send {
    async fn by_wallet(&self, user_wallet: &str) -> Result<Option<Stake>>;
    async fn usdc_balance(&self, user_wallet: &str) -> Result<FPUSDC>;
    async fn nez_balance(&self, user_wallet: &str) -> Result<FPUSDC>;
    async fn stake_update_by_transaction_id(&self, transaction_id: &TransactionId) -> Result<Option<StakeUpdate>>;
    async fn stake_updates_by_wallet(&self, user_wallet: &str) -> Result<Vec<StakeUpdate>>;
    async fn approve_stake_update(&self, user_wallet: &str) -> Result<StakeUpdate>;
    async fn complete_stake_update(&self, user_wallet: &str) -> Result<StakeUpdate>;
    async fn all(&self) -> Result<Vec<Stake>>;
    async fn all_stake_update_requests(&self) -> Result<Vec<StakeUpdateRequest>>;
}
