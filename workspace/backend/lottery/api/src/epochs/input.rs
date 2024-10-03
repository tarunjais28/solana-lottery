use anyhow::{anyhow, Result};
use async_graphql::InputObject;
use service::model::epoch;

/// Represents all the prizes of an epoch.
///
/// PrizesInput enforces that every tier has a defined prizes.
#[derive(InputObject, Debug)]
pub struct PrizesInput {
    tier1: String,
    tier2_yield_share: u8,
    tier3_yield_share: u8,
}

#[derive(InputObject, Debug)]
pub struct YieldSplitCfgInput {
    insurance_premium: String,
    insurance_probability: String,
    treasury_ratio: String,
}

pub fn make_yield_split_cfg(prizes: PrizesInput, yield_split_cfg: YieldSplitCfgInput) -> Result<epoch::YieldSplitCfg> {
    Ok(epoch::YieldSplitCfg {
        insurance: service::solana::InsuranceCfg {
            premium: yield_split_cfg
                .insurance_premium
                .parse()
                .map_err(|x| anyhow!("Can't parse insurance premium: {}", x))?,
            probability: yield_split_cfg
                .insurance_probability
                .parse()
                .map_err(|x| anyhow!("Can't parse insurance probability: {}", x))?,
        },
        jackpot: prizes
            .tier1
            .parse()
            .map_err(|x| anyhow!("Can't parse jackpot: {}", x))?,
        treasury_ratio: yield_split_cfg
            .treasury_ratio
            .parse()
            .map_err(|x| anyhow!("Can't parse treasury ratio: {}", x))?,
        tier2_prize_share: prizes.tier2_yield_share,
        tier3_prize_share: prizes.tier3_yield_share,
    })
}
