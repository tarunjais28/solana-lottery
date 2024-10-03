//! Returns distribution.
use solana_program::msg;

use crate::{
    error::StakingError,
    fixed_point::{FixedPoint, FPUSDC},
    state::{CumulativeReturnRate, PendingFunds, Ratio, Returns},
};

pub struct YieldSplitCfgInternal {
    pub insurance_amount: FPUSDC,
    pub treasury_ratio: FixedPoint<3>,
    pub tier2_prize_share: u8,
    pub tier3_prize_share: u8,
}

pub struct ReturnsInfo {
    pub returns: Returns,
    pub cumulative_return_rate: CumulativeReturnRate,
    pub draw_enabled: bool,
    pub pending_funds: PendingFunds,
}

pub fn distribute_returns(
    return_amount: FPUSDC,
    total_invested: FPUSDC,
    cumulative_return_rate: CumulativeReturnRate,
    pending_funds: PendingFunds,
    yield_split_cfg: YieldSplitCfgInternal,
) -> Result<ReturnsInfo, StakingError> {
    if total_invested == FixedPoint::zero() {
        if return_amount != FixedPoint::zero() {
            return Err(StakingError::ReturnAmountIsNonZeroButInvestedIsZero);
        }

        let deposit_back = FixedPoint::zero();

        let r#yield = FixedPoint::zero();
        let yield_split = distribute_yield(r#yield, pending_funds, yield_split_cfg)?;

        let returns = Returns {
            total: return_amount,
            deposit_back,
            insurance: yield_split.insurance,
            treasury: yield_split.treasury,
            tier2_prize: yield_split.tier2_prize,
            tier3_prize: yield_split.tier3_prize,
        };

        return Ok(ReturnsInfo {
            returns,
            cumulative_return_rate,
            draw_enabled: yield_split.draw_enabled,
            pending_funds: yield_split.pending_funds,
        });
    }

    if return_amount == FixedPoint::zero() {
        return Err(StakingError::ReturnAmountIsZero);
    }

    if return_amount < total_invested {
        // Loss
        let deposit_back = return_amount;
        let cumulative_return_rate = cumulative_return_rate
            .checked_mul(Ratio {
                numerator: return_amount,
                denominator: total_invested,
            })
            .ok_or(StakingError::NumericalOverflow)?;

        let r#yield = FixedPoint::zero();
        let yield_split = distribute_yield(r#yield, pending_funds, yield_split_cfg)?;
        let returns = Returns {
            total: return_amount,
            deposit_back,
            insurance: yield_split.insurance,
            treasury: yield_split.treasury,
            tier2_prize: yield_split.tier2_prize,
            tier3_prize: yield_split.tier3_prize,
        };

        return Ok(ReturnsInfo {
            returns,
            cumulative_return_rate,
            draw_enabled: yield_split.draw_enabled,
            pending_funds: yield_split.pending_funds,
        });
    }

    let deposit_back = total_invested;
    let cumulative_return_rate = cumulative_return_rate; // No change

    let r#yield = return_amount
        .checked_sub(total_invested)
        .expect("we ensured that return_amount >= total_invested above");
    let yield_split = distribute_yield(r#yield, pending_funds, yield_split_cfg)?;

    let returns = Returns {
        total: return_amount,
        deposit_back,
        insurance: yield_split.insurance,
        treasury: yield_split.treasury,
        tier2_prize: yield_split.tier2_prize,
        tier3_prize: yield_split.tier3_prize,
    };

    Ok(ReturnsInfo {
        returns,
        cumulative_return_rate,
        draw_enabled: yield_split.draw_enabled,
        pending_funds: yield_split.pending_funds,
    })
}

//

struct YieldSplitInfo {
    insurance: FPUSDC,
    treasury: FPUSDC,
    tier2_prize: FPUSDC,
    tier3_prize: FPUSDC,
    draw_enabled: bool,
    pending_funds: PendingFunds,
}

fn distribute_yield(
    yield_amount: FPUSDC,
    pending_funds: PendingFunds,
    yield_split_cfg: YieldSplitCfgInternal,
) -> Result<YieldSplitInfo, StakingError> {
    let insurance_needed = yield_split_cfg
        .insurance_amount
        .checked_sub(pending_funds.insurance)
        .unwrap_or(FPUSDC::zero());

    let amount = yield_amount;

    if amount < insurance_needed {
        msg!("Amount less than insurance needed: {} < {}", amount, insurance_needed);
        let pending_funds = PendingFunds {
            insurance: pending_funds
                .insurance
                .checked_add(amount)
                .ok_or(StakingError::NumericalOverflow)?,
            ..pending_funds
        };
        let draw_enabled = false;

        return Ok(YieldSplitInfo {
            insurance: amount,
            treasury: FixedPoint::zero(),
            tier2_prize: FixedPoint::zero(),
            tier3_prize: FixedPoint::zero(),
            pending_funds,
            draw_enabled,
        });
    }

    let insurance = insurance_needed;

    let amount = amount
        .checked_sub(insurance)
        .expect("we just checked that amount >= insurance_needed");

    let treasury = yield_split_cfg
        .treasury_ratio
        .change_precision()
        .checked_mul(amount)
        .ok_or(StakingError::NumericalOverflow)?;

    let amount = amount.checked_sub(treasury).ok_or(StakingError::NumericalOverflow)?;

    let total_shares: u16 = yield_split_cfg.tier2_prize_share as u16 + yield_split_cfg.tier3_prize_share as u16;

    let tier2_prize = amount
        .checked_mul(FPUSDC::from(yield_split_cfg.tier2_prize_share))
        .ok_or(StakingError::NumericalOverflow)?
        .checked_div(FPUSDC::from(total_shares))
        .ok_or(StakingError::NumericalOverflow)?;

    let tier3_prize = amount.checked_sub(tier2_prize).unwrap_or(FixedPoint::zero());

    let pending_funds = PendingFunds {
        // if insurance < pending_funds.insurance:
        //   put back what's left of pending_funds.insurance after taking insurance
        // else:
        //   pending_funds.insurance = 0
        insurance: pending_funds
            .insurance
            .checked_sub(yield_split_cfg.insurance_amount)
            .unwrap_or(FixedPoint::zero()),
        tier2_prize: pending_funds
            .tier2_prize
            .checked_add(tier2_prize)
            .ok_or(StakingError::NumericalOverflow)?,
        tier3_prize: pending_funds
            .tier3_prize
            .checked_add(tier3_prize)
            .ok_or(StakingError::NumericalOverflow)?,
    };

    let draw_enabled = true;

    Ok(YieldSplitInfo {
        insurance,
        treasury,
        tier2_prize,
        tier3_prize,
        draw_enabled,
        pending_funds,
    })
}
