//! Return rate calculations.
use std::ops::Deref;

use borsh::{BorshDeserialize, BorshSerialize};

use crate::fixed_point::{FPInternal, FixedPoint};

/// See [`stake_calculation`](super::stake_calculation) for a simplified version of how this is used.
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct CumulativeReturnRate(pub(super) FPInternal);

impl CumulativeReturnRate {
    pub fn new(rate: FPInternal) -> Option<Self> {
        if rate == FPInternal::zero() {
            None
        } else {
            Some(Self(rate))
        }
    }

    pub fn unity() -> Self {
        Self(FPInternal::from(1u8))
    }

    pub fn checked_mul<const D1: u8>(self, other: Ratio<D1>) -> Option<Self> {
        Some(Self(
            self.0
                .checked_mul(other.numerator.change_precision())?
                .checked_div(other.denominator.change_precision())?,
        ))
    }

    pub const fn max_len() -> usize {
        FPInternal::max_len()
    }
}

impl Deref for CumulativeReturnRate {
    type Target = FPInternal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Ratio<const D: u8> {
    pub numerator: FixedPoint<D>,
    pub denominator: FixedPoint<D>,
}
