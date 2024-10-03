//! Util functions for writing tests less cumbersome.
//! Don't use these in prod code as they may introduce performance penalty or precision issues.

use super::{FixedPoint, FPUSDC};

pub fn fp<const D: u8>(x: impl StrOrFloat) -> FixedPoint<D> {
    x.to_string().parse().unwrap()
}

pub fn usdc(x: &str) -> FPUSDC {
    fp(x)
}

pub fn fp_to_f64<const D: u8>(x: FixedPoint<D>) -> f64 {
    x.to_string().parse().unwrap()
}

pub trait StrOrFloat {
    fn to_string(self) -> String;
}

impl StrOrFloat for &str {
    fn to_string(self) -> String {
        self.to_owned()
    }
}

impl StrOrFloat for f64 {
    fn to_string(self) -> String {
        ToString::to_string(&self)
    }
}
