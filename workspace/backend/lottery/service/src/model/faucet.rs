use chrono::{DateTime, Utc};
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

use super::transaction::TransactionId;

#[derive(Clone, Debug, PartialEq)]
pub struct LatestMintTransaction {
    pub wallet: Pubkey,
    pub amount: u64,
    pub transaction_time: DateTime<Utc>,
    pub transaction_id: TransactionId,
}

#[derive(Debug, Error)]
pub enum FaucetError {
    #[error("Limit reached. Please try again in {hours} hours, {minutes} minutes, {seconds} seconds")]
    LimitReached { hours: i64, minutes: i64, seconds: i64 },

    #[error("Faucet disabled")]
    FaucetDisabled,
}
