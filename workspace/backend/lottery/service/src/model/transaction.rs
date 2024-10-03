use chrono::{DateTime, Utc};
use nezha_staking::fixed_point::FPUSDC;
use rand::{distributions::Standard, prelude::Distribution};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TransactionId(pub String);

impl From<Signature> for TransactionId {
    fn from(signature: Signature) -> Self {
        TransactionId(signature.to_string())
    }
}

impl TryFrom<TransactionId> for Signature {
    type Error = anyhow::Error;

    fn try_from(transaction_id: TransactionId) -> Result<Self, Self::Error> {
        Signature::from_str(&transaction_id.0).map_err(|error| error.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionType {
    DepositAttempt,
    DepositCancelled,
    DepositApproved,
    DepositCompleted,
    WithdrawAttempt,
    WithdrawCancelled,
    WithdrawApproved,
    WithdrawCompleted,
    Claim,
}

impl Distribution<TransactionType> for Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> TransactionType {
        match rng.gen_range(0..=8) {
            0 => TransactionType::DepositAttempt,
            1 => TransactionType::DepositApproved,
            2 => TransactionType::DepositCompleted,
            3 => TransactionType::DepositCancelled,
            4 => TransactionType::WithdrawAttempt,
            5 => TransactionType::WithdrawApproved,
            6 => TransactionType::WithdrawCompleted,
            7 => TransactionType::WithdrawCancelled,
            8 => TransactionType::Claim,
            _ => unreachable!(),
        }
    }
}

impl FromStr for TransactionType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "deposit_attempt" => Ok(Self::DepositAttempt),
            "deposit_approved" => Ok(Self::DepositApproved),
            "deposit_completed" => Ok(Self::DepositCompleted),
            "deposit_cancelled" => Ok(Self::DepositCancelled),
            "withdraw_attempt" => Ok(Self::WithdrawAttempt),
            "withdraw_approved" => Ok(Self::WithdrawApproved),
            "withdraw_completed" => Ok(Self::WithdrawCompleted),
            "withdraw_cancelled" => Ok(Self::WithdrawCancelled),
            "claim" => Ok(Self::Claim),
            s => Err(anyhow::anyhow!("Invalid transaction type: {}", s)),
        }
    }
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DepositAttempt => write!(f, "deposit_attempt"),
            Self::DepositApproved => write!(f, "deposit_approved"),
            Self::DepositCompleted => write!(f, "deposit_completed"),
            Self::DepositCancelled => write!(f, "deposit_cancelled"),
            Self::WithdrawAttempt => write!(f, "withdraw_attempt"),
            Self::WithdrawApproved => write!(f, "withdraw_approved"),
            Self::WithdrawCompleted => write!(f, "withdraw_completed"),
            Self::WithdrawCancelled => write!(f, "withdraw_cancelled"),
            Self::Claim => write!(f, "claim"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub transaction_id: TransactionId,
    pub instruction_index: u8,
    pub wallet: Pubkey,
    pub amount: FPUSDC,
    pub mint: Pubkey,
    pub time: Option<DateTime<Utc>>,
    pub transaction_type: TransactionType,
}

impl Ord for Transaction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.transaction_id
            .cmp(&other.transaction_id)
            .then(self.instruction_index.cmp(&other.instruction_index))
    }
}

impl PartialOrd for Transaction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.transaction_id == other.transaction_id && self.instruction_index == other.instruction_index
    }
}

impl Eq for Transaction {}
