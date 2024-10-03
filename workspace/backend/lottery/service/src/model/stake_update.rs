use anyhow::{bail, Result};
use nezha_staking::fixed_point::FPUSDC;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

use super::transaction::TransactionId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StakeUpdateState {
    Pending,
    Failed,
    Completed,
}

impl FromStr for StakeUpdateState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(StakeUpdateState::Pending),
            "failed" => Ok(StakeUpdateState::Failed),
            "completed" => Ok(StakeUpdateState::Completed),
            _ => bail!("Invalid stake update state: {}", s),
        }
    }
}

impl ToString for StakeUpdateState {
    fn to_string(&self) -> String {
        match self {
            StakeUpdateState::Pending => "pending".to_string(),
            StakeUpdateState::Failed => "failed".to_string(),
            StakeUpdateState::Completed => "completed".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StakeUpdateType {
    Deposit,
    Withdraw,
}

impl FromStr for StakeUpdateType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "deposit" => Ok(StakeUpdateType::Deposit),
            "withdraw" => Ok(StakeUpdateType::Withdraw),
            _ => bail!("Invalid stake update type: {}", s),
        }
    }
}

impl ToString for StakeUpdateType {
    fn to_string(&self) -> String {
        match self {
            StakeUpdateType::Deposit => "deposit".to_string(),
            StakeUpdateType::Withdraw => "withdraw".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakeUpdate {
    pub owner: Pubkey,
    pub amount: FPUSDC,
    pub type_: StakeUpdateType,
    pub state: StakeUpdateState,
    pub currency: String,
    pub mint: Pubkey,
    pub transaction_id: Option<TransactionId>,
}
