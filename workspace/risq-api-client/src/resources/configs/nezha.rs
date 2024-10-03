use std::collections::HashMap;

use crate::resources::{
    draw::objects::{DrawId, DrawInfo, DrawPrize, DrawPrizeType, DrawWinningKey},
    entry::objects::EntryVariant,
};
use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, Weekday};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub struct Nezha;
use super::DrawConfig;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NezhaSelection {
    pub main: Vec<u8>,
    pub bonus: Vec<u8>,
}

impl NezhaSelection {
    fn try_from_sequence(seq: &[u8]) -> Result<Self> {
        if seq.len() != 6 {
            anyhow::bail!("Expected sequence to have length 5; got {}", seq.len());
        }
        Ok(Self {
            main: seq[0..=4].to_owned(),
            bonus: vec![seq[5]],
        })
    }

    fn into_sequence(self) -> Vec<u8> {
        let mut res = self.main;
        res.extend(self.bonus);
        res
    }
}

impl DrawConfig for Nezha {
    fn product_id(&self) -> String {
        String::from("nezha-draw")
    }

    /// Validates that given date is on a saturday and returns DrawId newtype
    fn draw_id_try_from_draw_date(&self, draw_date: NaiveDate) -> anyhow::Result<DrawId> {
        if draw_date.weekday() == Weekday::Sat {
            Ok(DrawId::from_draw_date_unchecked(draw_date))
        } else {
            anyhow::bail!("given draw date must be on a saturday");
        }
    }

    /// Generate draw id for a draw after the given date
    /// Draw happens every saturday. So get the date for next saturday.
    fn next_draw_id(&self, after: NaiveDate) -> DrawId {
        let weekday = after.weekday();
        let num_days_from_saturday = match weekday {
            Weekday::Sat => 7,
            Weekday::Sun => 6,
            Weekday::Mon => 5,
            Weekday::Tue => 4,
            Weekday::Wed => 3,
            Weekday::Thu => 2,
            Weekday::Fri => 1,
        };
        let new_date = after + chrono::Duration::days(num_days_from_saturday as _);
        DrawId::from_draw_date_unchecked(new_date)
    }

    fn draw_info(&self, amount: Decimal) -> DrawInfo {
        let prize = DrawPrize {
            // Nezha uses RISQ for only tier-1 prizes which is of the type Fixed
            // see also the docs of DrawPrizeType
            type_: DrawPrizeType::Fixed,
            prize_id: "match-5-1".to_string(),
            estimated_amount: amount,
            actual_amount: amount,
            primary_winners: 1,
        };
        DrawInfo {
            currency: "USD".to_string(),
            prizes: HashMap::from([("default".to_string(), vec![prize])]),
        }
    }

    fn encode_entry_variant(&self, sequences: &mut dyn Iterator<Item = &[u8]>) -> Result<EntryVariant> {
        let mut selections: Vec<NezhaSelection> = Vec::with_capacity(sequences.size_hint().0);
        for seq in sequences {
            selections.push(NezhaSelection::try_from_sequence(seq)?);
        }

        Ok(EntryVariant {
            product_id: self.product_id(),
            entry_details: json!({
                "selections": selections,
                "draws": vec!["sat".to_string()],
                "weeks": 1
            }),
        })
    }

    fn decode_entry_variant(&self, entry_variant: &EntryVariant) -> anyhow::Result<Vec<Vec<u8>>> {
        if entry_variant.product_id != self.product_id() {
            anyhow::bail!(
                "Unexpected product_id {}; Expected {}",
                entry_variant.product_id,
                self.product_id()
            );
        }

        entry_variant
            .entry_details
            .as_object()
            .with_context(|| "Expected an object")
            .and_then(|obj| obj.get("selections").with_context(|| "Can't find key `selections`"))
            .and_then(|selections| {
                let selection: Vec<NezhaSelection> =
                    serde_json::from_value(selections.clone()).with_context(|| "Can't parse `selections`")?;
                Ok(selection.into_iter().map(NezhaSelection::into_sequence).collect())
            })
    }

    fn decode_winning_key(&self, winning_key: &DrawWinningKey) -> anyhow::Result<Vec<u8>> {
        let selection: NezhaSelection =
            serde_json::from_value(winning_key.winning_key.clone()).with_context(|| "Can't parse `winning_key`")?;
        Ok(selection.into_sequence())
    }
}

#[test]
fn test_nezha_selection_from_sequences() {
    let sequence = [1, 2, 3, 4, 5, 6];
    let selection_exp = NezhaSelection {
        main: vec![1, 2, 3, 4, 5],
        bonus: vec![6],
    };
    let selection_recv = NezhaSelection::try_from_sequence(&sequence).unwrap();
    assert_eq!(selection_recv, selection_exp);

    let sequence = [1, 2, 3, 4, 5, 6, 7];
    let res = NezhaSelection::try_from_sequence(&sequence);
    assert!(res.is_err());
    let sequence = [1, 2, 3, 4, 5];
    let res = NezhaSelection::try_from_sequence(&sequence);
    assert!(res.is_err());
}

#[test]
fn test_nezha_selection_to_sequences() {
    let selection = NezhaSelection {
        main: vec![1, 2, 3, 4, 5],
        bonus: vec![6],
    };
    let sequence_exp = [1, 2, 3, 4, 5, 6];
    let sequence_recv = selection.into_sequence();
    assert_eq!(sequence_recv, sequence_exp);
}
