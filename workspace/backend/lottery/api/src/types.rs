use async_graphql::scalar;
use serde::{Deserialize, Serialize};
use service::tickets::Sequence;
use solana_sdk::pubkey::{ParsePubkeyError, Pubkey};
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletAddr(pub String);

scalar!(WalletAddr);

impl From<Pubkey> for WalletAddr {
    fn from(pubkey: Pubkey) -> Self {
        WalletAddr(pubkey.to_string())
    }
}

impl TryFrom<WalletAddr> for Pubkey {
    type Error = ParsePubkeyError;

    fn try_from(wallet: WalletAddr) -> Result<Self, Self::Error> {
        Pubkey::from_str(&wallet.0)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Sequences(pub Vec<Sequence>);

scalar!(Sequences);

#[derive(Debug, Deserialize, Serialize)]
pub struct TransactionId(pub String);

scalar!(TransactionId);

impl From<service::model::transaction::TransactionId> for TransactionId {
    fn from(transaction_id: service::model::transaction::TransactionId) -> Self {
        TransactionId(transaction_id.0)
    }
}

impl From<TransactionId> for service::model::transaction::TransactionId {
    fn from(transaction_id: TransactionId) -> Self {
        service::model::transaction::TransactionId(transaction_id.0)
    }
}
