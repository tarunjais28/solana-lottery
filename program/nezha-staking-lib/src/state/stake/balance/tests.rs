use crate::fixed_point::test_utils::fp;

use super::*;

#[test]
fn balance() {
    let mut balance = FloatingBalance::new(fp(1.0), CumulativeReturnRate::unity());
    let mut current_return_rate = CumulativeReturnRate(fp(1.0));

    assert_eq!(balance.get_amount(current_return_rate).unwrap(), fp(1.0));

    balance.amount = fp(10.0);
    assert_eq!(balance.get_amount(current_return_rate).unwrap(), fp(10.0));

    balance.amount = fp(1.0);
    balance.starting_rate.0 = fp(1.0);
    current_return_rate.0 = fp(2.0);
    assert_eq!(balance.get_amount(current_return_rate).unwrap(), fp(2.0));

    current_return_rate.0 = fp(0.5);
    assert_eq!(balance.get_amount(current_return_rate).unwrap(), fp(0.5));

    balance.amount = fp(1.0);
    balance.starting_rate.0 = fp(2.0);

    current_return_rate.0 = fp(1.0);
    assert_eq!(balance.get_amount(current_return_rate).unwrap(), fp(0.5));

    balance.starting_rate.0 = fp(0.5);
    assert_eq!(balance.get_amount(current_return_rate).unwrap(), fp(2.0));
}

#[test]
fn stake() {
    let mut balance = FloatingBalance::new(fp(1.0), CumulativeReturnRate::unity());
    let mut current_return_rate = CumulativeReturnRate(fp(1.0));

    current_return_rate.0 = fp(1.0);
    balance.amount = fp(1.0);
    balance.starting_rate.0 = fp(1.0);
    balance = balance.checked_add(fp(1.0), current_return_rate).unwrap();
    assert_eq!(balance.amount, fp(2.0));
    assert_eq!(balance.get_amount(current_return_rate).unwrap(), fp(2.0));

    current_return_rate.0 = fp(2.0);
    balance.amount = fp(1.0);
    balance.starting_rate.0 = fp(1.0);
    balance = balance.checked_add(fp(1.0), current_return_rate).unwrap();
    assert_eq!(balance.get_amount(current_return_rate).unwrap(), fp(3.0));
}

#[test]
fn unstake() {
    let mut balance = FloatingBalance::new(fp(1.0), CumulativeReturnRate::unity());
    let mut current_return_rate = CumulativeReturnRate(fp(1.0));

    current_return_rate.0 = fp(1.0);
    balance.amount = fp(10.0);
    balance.starting_rate.0 = fp(1.0);
    balance = balance.checked_sub(fp(1.0), current_return_rate).unwrap();
    assert_eq!(balance.amount, fp(9.0));
    assert_eq!(balance.get_amount(current_return_rate).unwrap(), fp(9.0));

    current_return_rate.0 = fp(2.0);
    balance.amount = fp(1.0);
    balance.starting_rate.0 = fp(1.0);
    balance = balance.checked_sub(fp(1.0), current_return_rate).unwrap();
    assert_eq!(balance.get_amount(current_return_rate).unwrap(), fp(1.0));
}
