//! This module contains processed versions of on-chain structs to hide implementation details
//! On-chain structs those don't need any further processing are returned as-is by the Solana trait

use nezha_staking::{
    fixed_point::FPUSDC,
    state::{LatestEpoch, Stake as SolanaStake, Winner},
};
use solana_program::pubkey::Pubkey;

use super::error::{SolanaError, ToSolanaError};

/// Extracts common bits and winner specific bits from on-chain EpochTierWinners struct
pub struct WalletPrize {
    pub epoch_index: u64,
    pub page: u32,
    pub winner: Winner,
}

/// Applies cumulative returns to on-chain Stake struct
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Stake {
    pub owner: Pubkey,
    pub amount: FPUSDC,
    pub updated_epoch_index: u64,
}

impl Stake {
    pub fn try_from(solana_stake: SolanaStake, latest_epoch: &LatestEpoch) -> Result<Self, SolanaError> {
        let amount = solana_stake
            .balance
            .get_amount(latest_epoch.cumulative_return_rate)
            .context("Failed to calculate balance")?;
        Ok(Self {
            owner: solana_stake.owner,
            amount: amount.change_precision(),
            updated_epoch_index: solana_stake.updated_epoch_index,
        })
    }
}
