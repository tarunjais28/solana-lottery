use std::cmp;

use anyhow::Result;
use async_trait::async_trait;
use nezha_staking::fixed_point::FPUSDC;

#[async_trait]
pub trait BonusInfoService: Sync + Send {
    async fn min_stake_amount(&self) -> FPUSDC;
    async fn num_signup_bonus_sequences(&self, normal_sequence_count: u32) -> Result<u32>;
}

#[derive(Debug, Copy, Clone)]
pub enum BonusSequenceCount {
    Constant(u32),
    Multiplier { mul: f32, min_count: u32 },
}

#[derive(Debug, Copy, Clone)]
// sub = Sign Up Bonus
pub struct BonusInfo {
    pub sub_seq_count: BonusSequenceCount,
    pub sub_seq_min_stake: FPUSDC,
}

pub struct DefaultBonusInfoService {
    bonus_info: BonusInfo,
}

impl DefaultBonusInfoService {
    pub fn new(bonus_info: BonusInfo) -> Self {
        Self { bonus_info }
    }
}

#[async_trait]
impl BonusInfoService for DefaultBonusInfoService {
    async fn min_stake_amount(&self) -> FPUSDC {
        self.bonus_info.sub_seq_min_stake
    }

    async fn num_signup_bonus_sequences(&self, normal_sequence_count: u32) -> Result<u32> {
        if normal_sequence_count > 0 {
            match self.bonus_info.sub_seq_count {
                BonusSequenceCount::Constant(count) => Ok(count),
                BonusSequenceCount::Multiplier { mul, min_count } => {
                    let count = (normal_sequence_count as f32 * mul).trunc() as u32;
                    Ok(cmp::max(count, min_count))
                }
            }
        } else {
            Ok(0)
        }
    }
}
