use std::collections::HashMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Information about API usage quota
/// I kept the Bean in the name because that's how it is in the API Doc shared by RISQ.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceDetailsBean {
    partner_id: String,
    policy_id: String,
    currency: String,
    total_remaining_insurance_balance: Decimal,
    total_insurance_amount_spent: Decimal,
    total_remaining_cash_balance: Decimal,
    total_cash_amount_spent: Decimal,
    balance_by_licensee: HashMap<String, LicenseeDetailBean>,
}

/// Part of BalanceDetailsBean
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseeDetailBean {
    remaining_insurance_balance: Decimal,
    remaining_cash_balance: Decimal,
    insurance_amount_spent: Decimal,
    cash_amount_spent: Decimal,
}

/// Response of get_draw_prizes
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawPrizes {
    pub draw_id: String,
    pub product_id: String,
    pub info: DrawPrizesInfo,
}

/// Part of DrawPrizes
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawPrizesInfo {
    pub currency: String,
    pub prizes: Vec<DrawPrize>,
}

/// Part of DrawPrize
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawPrize {
    pub prize_id: String,
    pub amount: Decimal,
    pub winners: u64,
    pub prize_number: u64,
}

/// Response of get_draw_prizes_estimate
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawPrizesEstimate {
    pub currency: String,
    pub estimate_prizes: Vec<DrawPrizesEstimateItem>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DrawPrizesEstimateItem {
    pub tier_number: u64,
    pub estimated_amount: Decimal,
}
