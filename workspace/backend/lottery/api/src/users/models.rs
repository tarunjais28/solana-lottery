use crate::{TransactionId, WalletAddr};
use async_graphql::{Enum, SimpleObject};
use chrono::{DateTime, Utc};
use service::model;

/// Macro to generate From impls in both direction for GraphQL and Service enums
macro_rules! same_enum {
    (
        #[$($meta:meta),*]
        pub enum $enum:ident: $from:ty {
            $($variant:ident),*
        }
    ) => {
        #[$($meta),*]
        pub enum $enum {
            $($variant),*
        }

        impl From<$from> for $enum {
            fn from(s: $from) -> $enum {
                match s {
                    $(<$from>::$variant => $enum::$variant),*
                }
            }
        }

        impl From<$enum> for $from {
            fn from(s: $enum) -> $from {
                match s {
                    $($enum::$variant => <$from>::$variant),*
                }
            }
        }
    }
}

// Balance

#[derive(SimpleObject, Debug)]
pub struct Balance {
    pub(crate) amount: String,
    pub(crate) currency: String,
}

impl From<service::solana::Stake> for Balance {
    fn from(stake: service::solana::Stake) -> Self {
        Self {
            amount: stake.amount.to_string(),
            currency: "USDC".to_owned(),
        }
    }
}

// StakeUpdate

#[derive(SimpleObject, Debug)]
pub struct StakeUpdate {
    pub amount: String,
    pub transaction_id: Option<TransactionId>,
    pub state: StakeUpdateState,
    pub type_: StakeUpdateType,
    pub currency: String,
    pub mint: String,
}

same_enum! {
    #[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
    pub enum StakeUpdateState: model::stake_update::StakeUpdateState {
        Pending,
        Failed,
        Completed
    }
}

same_enum! {
    #[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
    pub enum StakeUpdateType: model::stake_update::StakeUpdateType {
        Deposit,
        Withdraw
    }
}

impl From<model::stake_update::StakeUpdate> for StakeUpdate {
    fn from(stake_update: model::stake_update::StakeUpdate) -> Self {
        Self {
            amount: stake_update.amount.to_string(),
            state: stake_update.state.into(),
            type_: stake_update.type_.into(),
            transaction_id: stake_update.transaction_id.map(Into::into),
            currency: stake_update.currency,
            mint: stake_update.mint.to_string(),
        }
    }
}

// DepositAttempt - This one should not be used by the UI, only by the indexers

#[derive(SimpleObject, Debug)]
pub struct StakeUpdateRequest {
    pub owner: WalletAddr,
    pub amount: i64,
    pub state: StakeUpdateRequestState,
}

same_enum! {
    #[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
    pub enum StakeUpdateRequestState: service::solana::StakeUpdateState {
        PendingApproval,
        Queued
    }
}

impl From<service::solana::StakeUpdateRequest> for StakeUpdateRequest {
    fn from(d: service::solana::StakeUpdateRequest) -> Self {
        Self {
            owner: WalletAddr(d.owner.to_string()),
            amount: d.amount,
            state: d.state.into(),
        }
    }
}

//

/// Represents the latest USDC faucet transaction for a user.
/// Amount is USDC value * 10^pow(decimals).
#[derive(SimpleObject, Debug)]
pub struct LatestMintTransaction {
    pub wallet: WalletAddr,
    pub amount: String,
    pub transaction_time: String,
    pub transaction_id: TransactionId,
}

impl From<service::model::faucet::LatestMintTransaction> for LatestMintTransaction {
    fn from(latest_mint_transaction: service::model::faucet::LatestMintTransaction) -> Self {
        LatestMintTransaction {
            wallet: WalletAddr(latest_mint_transaction.wallet.to_string()),
            amount: latest_mint_transaction.amount.to_string(),
            transaction_time: latest_mint_transaction.transaction_time.to_string(),
            transaction_id: latest_mint_transaction.transaction_id.into(),
        }
    }
}

same_enum! {
    #[derive(Enum, Copy, Clone, Eq, PartialEq, Debug)]
    pub enum TransactionType: service::model::transaction::TransactionType {
        DepositAttempt,
        DepositCancelled,
        DepositApproved,
        DepositCompleted,
        WithdrawAttempt,
        WithdrawCancelled,
        WithdrawApproved,
        WithdrawCompleted,
        Claim
    }
}

#[derive(SimpleObject, Debug)]
pub struct Transaction {
    pub transaction_id: TransactionId,
    pub wallet: WalletAddr,
    pub amount: String,
    pub mint: WalletAddr,
    pub time: Option<DateTime<Utc>>,
    pub transaction_type: TransactionType,
}

impl From<service::model::transaction::Transaction> for Transaction {
    fn from(transaction: service::model::transaction::Transaction) -> Self {
        Self {
            transaction_id: transaction.transaction_id.into(),
            wallet: WalletAddr::from(transaction.wallet),
            amount: transaction.amount.to_string(),
            mint: WalletAddr::from(transaction.mint),
            time: transaction.time,
            transaction_type: transaction.transaction_type.into(),
        }
    }
}

//

#[derive(SimpleObject, Debug)]
pub struct Prize {
    pub wallet: WalletAddr,
    pub epoch_index: u64,
    pub page: u32,
    pub winner_index: u32,
    pub tier: u8,
    pub amount: String,
    pub claimable: bool,
    pub claimed: bool,
}

impl From<service::model::prize::Prize> for Prize {
    fn from(prize: service::model::prize::Prize) -> Self {
        Self {
            wallet: prize.wallet.into(),
            epoch_index: prize.epoch_index,
            page: prize.page,
            winner_index: prize.winner_index,
            tier: prize.tier,
            amount: prize.amount.to_string(),
            claimable: prize.claimable,
            claimed: prize.claimed,
        }
    }
}
