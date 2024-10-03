use std::{collections::HashMap, fmt::Display, str::FromStr};

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{de::Error, Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawId(NaiveDate);

impl DrawId {
    pub fn get_date(&self) -> NaiveDate {
        self.0
    }

    pub fn from_draw_date_unchecked(draw_date: NaiveDate) -> DrawId {
        DrawId(draw_date)
    }
}

impl Serialize for DrawId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // to_string() gives ISO8601 string (yyyy-mm-dd) which is exactly what we want
        self.0.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DrawId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <&str>::deserialize(deserializer)?;
        let date = NaiveDate::from_str(s).map_err(|_err| {
            D::Error::invalid_type(
                serde::de::Unexpected::Str(s),
                &"a yyyy-mm-dd date string. Example: 2022-04-05",
            )
        })?;
        Ok(Self(date))
    }
}

impl Display for DrawId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // to_string() gives ISO8601 string (yyyy-mm-dd) which is exactly what we want
        self.0.to_string().fmt(f)
    }
}

/// All the information about a Draw returned by RISQ
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Draw {
    pub draw_id: DrawId,
    pub product_id: String,
    pub info: DrawInfo,
    // Available once the draw is settled
    pub stats: Option<DrawStats>,
}

/// The part of a Draw which we can update
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DrawInfo {
    pub currency: String,
    pub prizes: HashMap<String, Vec<DrawPrize>>,
}

/// Part of DrawInfo
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DrawPrize {
    #[serde(rename = "type")]
    pub type_: DrawPrizeType,
    pub prize_id: String,
    pub estimated_amount: Decimal,
    pub actual_amount: Decimal,
    pub primary_winners: u64,
}

/// Part of DrawPrize
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum DrawPrizeType {
    #[serde(rename = "pari")]
    /// All the amount is pooled and is distributed among the winners
    PariMutuel,
    #[serde(rename = "fixed")]
    /// Each winner gets a fixed pre-defined amount
    Fixed,
}

/// Partner and licensee costs info about a Draw.
/// Will be returned after a Draw has finished.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DrawStats {
    pub currency: String,
    pub cost: DrawStatsPartnerCost,
}

/// Part of DrawStats
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DrawStatsPartnerCost {
    pub entries: u64,
    pub lines: u64,
    pub licensee_costs: Vec<DrawStatsLicenseeCost>,
}

/// Part of DrawStats
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DrawStatsLicenseeCost {
    pub licensee_id: String,
    pub entries: u64,
    pub lines: u64,
    pub prize_costs: DrawStatsPrizeCost,
}

/// Part of DrawStats
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DrawStatsPrizeCost {
    pub prize_id: String,
    pub cost_per_line: Decimal,
    pub cost: Decimal,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DrawWinningKey {
    pub draw_id: Option<DrawId>,
    pub winning_key: serde_json::Value,
}
