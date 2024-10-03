//! FixedPoint number type.

pub mod test_utils;

#[cfg(test)]
mod tests;

uint::construct_uint! {
    /// 192 bit unsigned number used by the `FixedPoint` type.
    #[derive(BorshSerialize, BorshDeserialize)]
    pub struct U192(3);
}

use std::{fmt::Display, str::FromStr};

use borsh::{BorshDeserialize, BorshSerialize};

/// The FixedPoint type.
/// Helps to do fixed precision calculation with a large precision (192-bits) unsigned number.
/// `D` is the number of digits after the decimal point.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, BorshDeserialize, BorshSerialize)]
pub struct FixedPoint<const D: u8>(pub U192);

impl<const D: u8> std::fmt::Debug for FixedPoint<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string())
    }
}

impl<const D: u8> FixedPoint<D> {
    /// Create a `FixedPoint` from a `U192`.
    pub fn new(amount: U192) -> FixedPoint<D> {
        Self(amount)
    }

    /// Create a fixed point from `u64` and the number of digits after the decimal point.
    pub fn from_fixed_point_u64(amount: u64, num_decimals: u8) -> FixedPoint<D> {
        if num_decimals > D {
            let amount = U192::from(amount);
            let divisor = U192::from(10u64).pow(U192::from(num_decimals - D));
            Self(amount / divisor)
        } else if num_decimals == D {
            let amount = U192::from(amount);
            Self(amount)
        } else {
            let amount = U192::from(amount);
            let multiplier = U192::from(10u64).pow(U192::from(D - num_decimals));
            Self(amount * multiplier)
        }
    }

    /// Zero.
    pub const fn zero() -> FixedPoint<D> {
        Self(U192::zero())
    }

    /// Return `10^D`.
    pub fn decimals_multiplier() -> U192 {
        U192::from(10).pow(D.into())
    }

    pub fn checked_add(&self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    pub fn checked_sub(&self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    pub fn checked_mul(&self, rhs: Self) -> Option<Self> {
        self.0
            .checked_mul(rhs.0)
            .map(|x| {
                // No need to use checked div here because we know that decimals_multiplier >= 1
                // So div by zero won't happen. Nor does div by a number less than 1 causing the
                // result to blow up.
                x / Self::decimals_multiplier()
            })
            .map(Self)
    }

    pub fn checked_div(&self, rhs: Self) -> Option<Self> {
        self.0
            .checked_mul(Self::decimals_multiplier())
            .and_then(|x| x.checked_div(rhs.0))
            .map(Self)
    }

    /// Convert FixedPoint<D> to FixedPoint<D1>.
    pub fn change_precision<const D1: u8>(&self) -> FixedPoint<D1> {
        if D1 > D {
            FixedPoint(self.0 * U192::from(10).pow((D1 - D).into()))
        } else if D1 == D {
            FixedPoint(self.0)
        } else {
            FixedPoint(self.0 / U192::from(10).pow((D - D1).into()))
        }
    }

    /// Maximum size occupied by a FixedPoint in Borsch encoding.
    pub const fn max_len() -> usize {
        192 / 8
    }

    pub fn as_whole_number(&self) -> u64 {
        let f: FixedPoint<0> = self.change_precision();
        f.0.try_into().expect("Couldn't convert to u64")
    }
}

macro_rules! impl_from_uint {
    ($t:ty) => {
        /// Converts a whole number to FixedPoint.
        /// For eg: FixedPoint<2>::from(1) = FixedPoint(100)
        impl<const D: u8> From<$t> for FixedPoint<D> {
            fn from(x: $t) -> Self {
                Self(Self::decimals_multiplier() * U192::from(x))
            }
        }
    };
}

impl_from_uint!(u8);
impl_from_uint!(u16);
impl_from_uint!(u32);
impl_from_uint!(u64);

impl<const D: u8> Display for FixedPoint<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::new();
        let mut x = self.0;
        for _ in 0..D {
            s.push_str(&(x % U192::from(10)).to_string());
            x /= U192::from(10);
        }
        s.push('.');
        if x == U192::from(0) {
            s.push('0');
        } else {
            while x > U192::from(0) {
                s.push_str(&(x % U192::from(10)).to_string());
                x /= U192::from(10);
            }
        }
        let s: String = s.chars().rev().collect();
        s.fmt(f)
    }
}

impl<const D: u8> FromStr for FixedPoint<D> {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut u192 = U192::zero();
        let mut d: Option<u8> = None;
        for i in s.chars() {
            if i.is_ascii_digit() {
                u192 *= 10;
                if let Some(d) = &mut d {
                    *d = *d + 1;
                }
            }
            u192 += U192::from(match i {
                '0' => 0u8,
                '1' => 1,
                '2' => 2,
                '3' => 3,
                '4' => 4,
                '5' => 5,
                '6' => 6,
                '7' => 7,
                '8' => 8,
                '9' => 9,
                '.' => {
                    if let None = d {
                        d = Some(0);
                    } else {
                        return Err(format!("More than 1 decimal points found"));
                    }
                    0
                }
                '_' => 0,
                _ => return Err(format!("Invalid char {}", i)),
            })
        }
        let d: u8 = d.unwrap_or(0);
        if D > d {
            u192 *= 10u64.pow((D - d) as _);
        } else if D < d {
            u192 /= 10u64.pow((d - D) as _);
        }
        let fp: FixedPoint<D> = FixedPoint::new(u192);
        Ok(fp)
    }
}

/// Type alias for USDC precision (6 digits).
pub type FPUSDC = FixedPoint<6>;
/// Type alias for high precision internal calculations (18 digits).
pub type FPInternal = FixedPoint<18>;

impl<const D: u8> FixedPoint<D> {
    /// Convert the FixedPoint to u64 in USDC format.
    pub fn as_usdc(self) -> u64 {
        self.change_precision::<6>().0.as_u64()
    }

    /// Convert a u64 in USDC format to FixedPoint.
    pub fn from_usdc(x: u64) -> Self {
        Self::from_fixed_point_u64(x, 6)
    }

    /// Convert a u64 in USDC format to FixedPoint.
    pub fn from_whole_number(x: u64) -> Self {
        Self::from_fixed_point_u64(x, 0)
    }

    /// Convert the FixedPoint to u64 in USDC format, but try to convert it to i64 if it fits.
    /// Used for encoding deposits/withdraw as positive/negative numbers respectively.
    pub fn as_usdc_i64(self) -> i64 {
        self.as_usdc().try_into().expect("Couldn't convert into i64")
    }
}
