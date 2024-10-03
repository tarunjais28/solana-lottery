pub mod service;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
pub use nezha_staking::fixed_point::FPUSDC;
use nezha_staking::state::YieldSplitCfg;
use solana_program::pubkey::Pubkey;

use crate::{
    model::{
        epoch::{Epoch, Investor, UseCache},
        winner::EpochWinners,
    },
    solana::WalletPrize,
    tickets::Winners,
};

#[async_trait]
pub trait EpochRepository: Sync + Send {
    /// Retrieves epoch from DB by index, returns None if no epoch exists for the given index
    async fn by_index(&self, index: u64) -> Result<Option<Epoch>>;

    /// Retrieves epoch from DB by pubkey, returns None if no epoch exists for the given index
    async fn by_pubkey(&self, epoch: &Pubkey) -> Result<Option<Epoch>>;

    /// Retrieves all epochs from DB
    async fn all(&self) -> Result<Vec<Epoch>>;

    /// Retrieves the latest epoch from DB, if there is one.
    async fn latest_epoch(&self) -> Result<Option<Epoch>>;

    /// Creates a new epoch or updates an existing epoch in the DB.
    async fn create_or_update_epoch(&self, epoch: &Epoch) -> Result<Epoch>;
}

#[async_trait]
pub trait EpochManager: Sync + Send {
    async fn epochs(&self) -> Result<Vec<Epoch>>;

    /// Reads the latest epoch from cache, if it exists, otherwise reads it from chain
    async fn latest_epoch(&self, use_cache: UseCache) -> Result<Option<Epoch>>;

    /// Reads epoch by index from cache, if it exists, otherwise reads it from chain
    async fn epoch_by_index(&self, index: u64, use_cache: UseCache) -> Result<Option<Epoch>>;

    /// Read epoch, if an index is provided, reads the specific index, if None is provided, fetches the latest index.
    async fn read_epoch_prizes(&self, index: u64) -> Result<Option<EpochWinners>>;

    /// Fetch and decode epoch by pubkey. While this is not user friendly, it can be useful when, in some cases,
    /// the pubkey for the epoch is known but not the index.
    async fn epoch_by_pubkey(&self, pubkey: &Pubkey) -> Result<Option<Epoch>>;

    /// Creates a new epoch, provided the previous one has ended.
    /// The expected end date is for reference only and will be used as an identifier for the RISQ API Draw.
    async fn create_epoch(&self, expected_end_date: DateTime<Utc>, yield_split_cfg: YieldSplitCfg) -> Result<Epoch>;

    /// Withdraw yield to fake investor or Francium for the current epoch
    async fn enter_investment(&self, investor: Investor) -> Result<Epoch>;

    /// Return yield from fake investor or Francium back to vault for the current epoch
    async fn exit_investment(&self, investor: Investor, return_amount: Option<FPUSDC>) -> Result<Epoch>;

    /// Used for testing, can't be called in release mode.
    async fn publish_winning_combination(&self, combination: &[u8; 6]) -> Result<Epoch>;

    /// Perform vrf request to generate the winning combination.
    async fn request_random_winning_combination(&self) -> Result<Epoch>;

    /// Publishes all the winners of the current epoch. This is called only after calculating winners
    async fn publish_winners(&self, winners: Winners) -> Result<Epoch>;

    /// Fetch entire history of prizes for this wallet.
    async fn wallet_prizes(&self, wallet: &Pubkey) -> Result<Vec<WalletPrize>>;

    /// Fund the jackpot winner
    async fn fund_jackpot(&self) -> Result<()>;
}
