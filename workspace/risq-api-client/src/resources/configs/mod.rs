use anyhow::Result;
use chrono::NaiveDate;
use rust_decimal::Decimal;

use super::{
    draw::objects::{DrawId, DrawInfo, DrawWinningKey},
    entry::objects::EntryVariant,
};

pub mod nezha;

pub trait DrawConfig {
    fn product_id(&self) -> String;
    fn draw_id_try_from_draw_date(&self, draw_date: NaiveDate) -> Result<DrawId>;
    fn next_draw_id(&self, draw_date: NaiveDate) -> DrawId;
    fn draw_info(&self, amount: Decimal) -> DrawInfo;
    fn encode_entry_variant(&self, sequences: &mut dyn Iterator<Item = &[u8]>) -> Result<EntryVariant>;
    fn decode_entry_variant(&self, entry_variant: &EntryVariant) -> Result<Vec<Vec<u8>>>;
    fn decode_winning_key(&self, winning_key: &DrawWinningKey) -> Result<Vec<u8>>;
}
