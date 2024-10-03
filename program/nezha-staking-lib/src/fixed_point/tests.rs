use super::test_utils::*;
use super::{FPInternal, FixedPoint, FPUSDC};
use borsh::{BorshDeserialize, BorshSerialize};

#[test]
fn test_add() {
    let a: FixedPoint<6> = fp(1.0);
    let b: FixedPoint<6> = fp(2.0);

    let c = a.checked_add(b).map(fp_to_f64);
    assert_eq!(c, Some(3.0f64));

    let a: FixedPoint<6> = FixedPoint::new(1_000_001u64.into());
    let b: FixedPoint<6> = a.checked_add(a).unwrap();

    assert_eq!(b, FixedPoint::new(2_000_002u64.into()));
}

#[test]
fn test_sub() {
    let a: FixedPoint<6> = fp(1.0);
    let b: FixedPoint<6> = fp(2.0);

    let c: Option<f64> = a.checked_sub(b).map(fp_to_f64);
    assert_eq!(c, None);

    let c: Option<f64> = b.checked_sub(a).map(fp_to_f64);
    assert_eq!(c, Some(1.0f64));

    let a: FixedPoint<6> = FixedPoint::new(1_000_001u64.into());
    let b: FixedPoint<6> = FixedPoint::new(0_000_001u64.into());
    let c: Option<FixedPoint<6>> = a.checked_sub(b);

    assert_eq!(c, Some(FixedPoint::new(1_000_000u64.into())));
}

#[test]
fn test_mul() {
    let a: FixedPoint<6> = fp(1.0);
    let b: FixedPoint<6> = fp(2.0);

    let c = a.checked_mul(b).map(fp_to_f64);
    assert_eq!(c, Some(2.0f64));

    let a: FixedPoint<6> = FixedPoint::new(1_000_000u64.into());
    let b: FixedPoint<6> = FixedPoint::new(0_000_001u64.into());
    let c: FixedPoint<6> = a.checked_mul(b).unwrap();
    assert_eq!(c, FixedPoint::new(0_000_001u64.into()));

    let a: FixedPoint<6> = FixedPoint::new(0_000_015u64.into());
    let b: FixedPoint<6> = FixedPoint::new(0_100_000u64.into());
    let c: FixedPoint<6> = a.checked_mul(b).unwrap();
    assert_eq!(c, FixedPoint::new(0_000_001u64.into()));

    let a: FixedPoint<6> = FixedPoint::new(0_000_001u64.into());
    let b: FixedPoint<6> = FixedPoint::new(0_000_001u64.into());
    let c: FixedPoint<6> = a.checked_mul(b).unwrap();
    assert_eq!(c, FixedPoint::new(0_000_000u64.into()));
}

#[test]
fn test_div() {
    let a: FixedPoint<6> = fp(1.0);
    let b: FixedPoint<6> = fp(2.0);

    let c = a.checked_div(b).map(fp_to_f64);
    assert_eq!(c, Some(0.5f64));

    let a: FixedPoint<6> = FixedPoint::new(0_000_015u64.into());
    let b: FixedPoint<6> = FixedPoint::new(10_000_000u64.into());
    let c: FixedPoint<6> = a.checked_div(b).unwrap();
    assert_eq!(c, FixedPoint::new(0_000_001u64.into()));
}

#[test]
fn test_from_fixed_point() {
    let a: FixedPoint<6> = fp(1.0);

    let b: FixedPoint<6> = FixedPoint::from_fixed_point_u64(1_000_000, 6);
    assert_eq!(a, b);

    let b: FixedPoint<6> = FixedPoint::from_fixed_point_u64(1_000_00, 5);
    assert_eq!(a, b);

    let b: FixedPoint<6> = FixedPoint::from_fixed_point_u64(1_000_0000, 7);
    assert_eq!(a, b);
}

#[test]
fn test_change_precision() {
    let a: FixedPoint<2> = fp(1.1);
    let b: FixedPoint<3> = a.change_precision();
    let c: FixedPoint<3> = fp(1.1);
    assert_eq!(b, c);

    let b: FixedPoint<0> = a.change_precision();
    let c: FixedPoint<0> = fp(1.0);
    assert_eq!(b, c);
}

#[test]
fn test_borsch() {
    let f: FixedPoint<10> = fp(1.23);
    let mut v = Vec::new();
    f.serialize(&mut v).unwrap();

    let f1: FixedPoint<10> = BorshDeserialize::try_from_slice(&v).unwrap();
    assert_eq!(f, f1);
}

#[test]
fn test_from_str() {
    assert_eq!("1.234567".parse(), Ok(FPUSDC::from_usdc(1_234_567)));
    assert_eq!("1.234567890".parse(), Ok(FPUSDC::from_usdc(1_234_567)));
    assert_eq!("1.234567890".parse(), Ok(FPUSDC::from_usdc(1_234_567)));
    assert_eq!("1.234".parse(), Ok(FPUSDC::from_usdc(1_234_000)));
    assert_eq!("1_000.234_567".parse(), Ok(FPUSDC::from_usdc(1_000_234_567)));
}

#[test]
fn test_str_roundtrip() {
    let fps = [1_234_567, 1_000_567, 0_000_567, 0_000_007, 0_000_000, 9_999_999];

    for i in fps {
        let fp = FPUSDC::from_usdc(i);
        assert_eq!(fp.to_string().parse(), Ok(fp));

        let fp = FPInternal::from_usdc(i);
        assert_eq!(fp.to_string().parse(), Ok(fp));
    }
}
