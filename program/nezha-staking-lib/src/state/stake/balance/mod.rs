#[cfg(test)]
mod tests;

use borsh::{BorshDeserialize, BorshSerialize};

use crate::fixed_point::FPInternal;
use crate::state::CumulativeReturnRate;

/// See [`stake_calculation`](super::super::stake_calculation)
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct FloatingBalance {
    amount: FPInternal,
    starting_rate: CumulativeReturnRate,
}

impl FloatingBalance {
    pub fn new(amount: FPInternal, starting_rate: CumulativeReturnRate) -> Self {
        Self { amount, starting_rate }
    }

    pub fn get_amount(&self, current_rate: CumulativeReturnRate) -> Option<FPInternal> {
        self.amount
            .checked_mul(current_rate.0)?
            .checked_div(self.starting_rate.0)
    }

    pub fn apply_current_rate(&self, current_rate: CumulativeReturnRate) -> Option<Self> {
        let amount = self.get_amount(current_rate)?;
        let starting_rate = current_rate;
        Some(Self { amount, starting_rate })
    }

    pub fn checked_add(&self, amount: FPInternal, current_rate: CumulativeReturnRate) -> Option<Self> {
        let mut x = self.apply_current_rate(current_rate)?;
        x.amount = x.amount.checked_add(amount)?;
        Some(x)
    }

    pub fn checked_sub(&self, amount: FPInternal, current_rate: CumulativeReturnRate) -> Option<Self> {
        let mut x = self.apply_current_rate(current_rate)?;
        x.amount = x.amount.checked_sub(amount)?;
        Some(x)
    }

    pub const fn max_len() -> usize {
        FPInternal::max_len() +             // amount
        CumulativeReturnRate::max_len() +   // starting_rate
        0
    }
}
