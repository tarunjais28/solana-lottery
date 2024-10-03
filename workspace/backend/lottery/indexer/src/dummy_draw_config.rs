use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;

use risq_api_client::resources::{
    configs::DrawConfig,
    draw::objects::{DrawId, DrawInfo, DrawPrize, DrawPrizeType, DrawWinningKey},
    entry::objects::EntryVariant,
};

#[derive(Clone)]
pub struct DummyDrawConfig;

impl DrawConfig for DummyDrawConfig {
    fn product_id(&self) -> String {
        String::from("dummy-draw")
    }

    fn draw_id_try_from_draw_date(
        &self,
        draw_date: chrono::NaiveDate,
    ) -> anyhow::Result<risq_api_client::resources::draw::objects::DrawId> {
        Ok(DrawId::from_draw_date_unchecked(draw_date))
    }

    fn next_draw_id(&self, draw_date: chrono::NaiveDate) -> risq_api_client::resources::draw::objects::DrawId {
        DrawId::from_draw_date_unchecked(draw_date)
    }

    fn draw_info(&self, amount: rust_decimal::Decimal) -> risq_api_client::resources::draw::objects::DrawInfo {
        DrawInfo {
            currency: "DUMMY".to_string(),
            prizes: HashMap::from_iter(
                [(
                    "DUMMY".to_string(),
                    vec![DrawPrize {
                        type_: DrawPrizeType::Fixed,
                        estimated_amount: amount,
                        actual_amount: amount,
                        prize_id: "DUMMY".to_string(),
                        primary_winners: 1,
                    }],
                )]
                .into_iter(),
            ),
        }
    }

    fn encode_entry_variant(&self, sequences: &mut dyn Iterator<Item = &[u8]>) -> Result<EntryVariant> {
        Ok(EntryVariant {
            product_id: self.product_id(),
            entry_details: json!(sequences.collect::<Vec<_>>()),
        })
    }

    fn decode_entry_variant(&self, _entry_variant: &EntryVariant) -> anyhow::Result<Vec<Vec<u8>>> {
        todo!()
    }

    fn decode_winning_key(&self, _winning_key: &DrawWinningKey) -> anyhow::Result<Vec<u8>> {
        todo!()
    }
}
