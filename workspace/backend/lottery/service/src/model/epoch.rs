use std::time::{Duration, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use nezha_staking::fixed_point::FPUSDC;
use solana_program::pubkey::Pubkey;
use thiserror::Error;

pub use nezha_staking::state::EpochStatus;

use crate::solana::WithPubkey;
pub use nezha_staking::state::Returns;
pub use nezha_staking::state::YieldSplitCfg;

#[derive(Debug, Clone, PartialEq)]
pub struct Epoch {
    pub pubkey: Pubkey,
    pub index: u64,
    pub status: EpochStatus,
    pub yield_split_cfg: YieldSplitCfg,
    pub winning_combination: Option<[u8; 6]>,
    pub total_invested: Option<FPUSDC>,
    pub returns: Option<Returns>,
    pub started_at: DateTime<Utc>,
    pub expected_end_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub draw_enabled: Option<bool>,
}

impl Epoch {
    pub fn from_solana(epoch: &WithPubkey<nezha_staking::state::Epoch>, winning_combination: Option<[u8; 6]>) -> Epoch {
        Epoch {
            pubkey: epoch.pubkey,
            index: epoch.index,
            status: epoch.status.into(),
            yield_split_cfg: epoch.yield_split_cfg.clone(),
            winning_combination,
            total_invested: epoch.total_invested,
            returns: epoch.returns.clone(),

            // unwrap is not really great but this is created on-chain by the admin so it *should* be fine.
            ended_at: epoch
                .end_at
                .map(|seconds| DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_secs(seconds.try_into().unwrap()))),
            started_at: DateTime::<Utc>::from(UNIX_EPOCH + Duration::from_secs(epoch.start_at.try_into().unwrap())),
            expected_end_at: DateTime::<Utc>::from(
                UNIX_EPOCH + Duration::from_secs(epoch.expected_end_at.try_into().unwrap()),
            ),
            draw_enabled: epoch.draw_enabled,
        }
    }
}

#[derive(Error, Debug)]
pub enum EpochError {
    #[error("Last epoch not finalized. You need to publish winners first.")]
    LastEpochNotFinished,

    #[error("Could not read latest epoch")]
    CouldNotReadLatestEpoch,

    #[error("Total invested not set. Yield has to be withdrawn first.")]
    TotalInvestedNotSet,

    #[error("Total returned not set. Yield has to be returned first.")]
    TotalReturnedNotSet,

    #[error("Winning combination is not set")]
    WinningCombinationNotSet,

    #[error("End timestamp not set")]
    EndTimestampNotSet,

    #[error("Wrong number of tiers. Expected 3 but found {0}")]
    WrongNumberOfTiers(u8),

    #[error("Could not read winners in tier {tier} for epoch {epoch_index}")]
    CouldNotReadWinners { epoch_index: u64, tier: u8 },
}

pub enum UseCache {
    Yes,
    No,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Investor {
    Francium,
    Fake,
}
