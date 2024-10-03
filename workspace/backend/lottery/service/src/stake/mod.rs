use anyhow::Result;
use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;

mod service;
pub use self::service::*;
mod service_impl;
pub use self::service_impl::*;

use crate::model::{
    stake_update::StakeUpdate,
    transaction::{Transaction, TransactionId},
};

#[async_trait]
pub trait StakeUpdateRepository: Sync + Send {
    async fn by_transaction_id(&self, transaction_id: &TransactionId) -> Result<Option<StakeUpdate>>;
    async fn by_wallet(&self, wallet: &Pubkey) -> Result<Vec<StakeUpdate>>;
    async fn store(&self, stake_update: &StakeUpdate) -> Result<StakeUpdate>;
}

#[async_trait]
pub trait TransactionDecoder: Sync + Send {
    async fn by_wallet(&self, wallet: &Pubkey, before: Option<TransactionId>) -> Result<Vec<Transaction>>;
    async fn deposits(&self, before: Option<TransactionId>) -> Result<Vec<Transaction>>;
}
