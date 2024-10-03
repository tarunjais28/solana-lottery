use std::collections::HashMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::resources::draw::objects::DrawId;

/// Input to send_entry
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub draw_id: DrawId,
    /// Our reference
    pub entry_ref: String,
    pub licensee_id: String,
    pub player_id: String,
    pub timestamp: u64,
    #[serde(flatten)]
    pub variant: EntryVariant,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EntryVariant {
    pub product_id: String,
    #[serde(flatten)]
    pub entry_details: serde_json::Value,
}

/// Response of `send_entry`
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryReceipt {
    /// Our referencce
    pub entry_ref: String,
    #[serde(flatten)]
    pub status: EntryReceiptStatus,
}

/// Part of EntryReceipt
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum EntryReceiptStatus {
    /// Entry was accepted by RISQ
    #[serde(rename = "ok")]
    #[serde(rename_all = "camelCase")]
    Ok {
        /// Their reference, proof that it's stored in their system
        entry_id: String,
        timestamp: u64,
    },
    /// Entry was rejected by RISQ
    #[serde(rename = "failed")]
    #[serde(rename_all = "camelCase")]
    Failed { error: String },
    #[serde(rename = "unknown")]
    #[serde(rename_all = "camelCase")]
    Unknown {},
}

/// Response of retrieve_entry
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryDetail {
    /// Their reference
    pub entry_id: String,
    pub entry: Entry,
    pub status: String,
    // When the entry was accepted by RISQ
    pub timestamp: Option<u64>,
    /// The signature sent by us when sending the entry in X-Risq-Signature header for single entry
    /// of signatures map for batch of entries.
    /// Signature sending for single entries is currently not implemented as it's not mandatory.
    pub signature: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryBatch {
    pub batch_id: String,
    pub entries: Vec<Entry>,
    /// Signatures for each entry. Key is entry_ref, value is signature.
    /// Optional.
    pub signatures: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryBatchReceipt {
    pub batch_id: String,
    pub receipts: Vec<EntryReceipt>,
}

#[derive(Debug, Serialize, Deserialize)]
/// NOTE: We don't do #[serde(rename_all = "camelCase")]
/// because the response uses inconsistent casing.
pub struct RetrieveWinnersOutput {
    pub draw_id: DrawId,
    pub product_id: String,
    #[serde(rename = "partner_Id")]
    pub partner_id: String,
    #[serde(rename = "drawWinRisqList")]
    pub draw_win_risq_list: Vec<DrawWinRisq>,
}

/// Part of RetrieveWinnersOutput
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawWinRisq {
    win_id: String,
    licensee_id: String,
    /// Their reference
    entry_id: String,
    /// Our reference
    entry_ref: String,
    prize_id: String,
    prize_number: u64,
    amount: Decimal,
    currency: String,
}
