use nezha_staking_lib::{
    fixed_point::test_utils::fp,
    state::{InsuranceCfg, YieldSplitCfg},
};

use super::*;

#[tokio::test]
async fn test_withdrawal_completed() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    let balance_before = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;

    run_stake_update(StakeUpdateOp::Deposit, usdc("100.0"), &accounts, processor.as_mut()).await?;
    run_stake_update(StakeUpdateOp::Withdraw, usdc("50.0"), &accounts, processor.as_mut()).await?;

    let balance_after = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;
    let diff = balance_before.checked_sub(balance_after).unwrap();

    assert_eq!(diff, usdc("50.0"));

    assert_balances(
        AssertBalances::deposit_complete(usdc("50.0")),
        &accounts,
        processor.as_mut(),
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_approval_not_needed() -> Result<()> {
    let (accounts, mut processor) = setup().await?;
    run_stake_update(StakeUpdateOp::Deposit, fp("200.0"), &accounts, processor.as_mut()).await?;

    request_stake_update(StakeUpdateOp::Withdraw, usdc("50.0"), &accounts, processor.as_mut()).await?;
    complete_stake_update(&accounts, processor.as_mut()).await?;

    Ok(())
}

#[tokio::test]
async fn test_withdraw_more_than_balance() -> Result<()> {
    let accounts = Accounts::new();
    let mut processor = setup_test_runtime(&accounts).await?;

    create_epoch(
        &accounts,
        YieldSplitCfg {
            jackpot: fp("1000"),
            insurance: InsuranceCfg {
                premium: fp("2.0"),
                probability: fp("0.0005"),
            },
            treasury_ratio: fp("0.5"),
            tier2_prize_share: 2,
            tier3_prize_share: 1,
        },
        processor.as_mut(),
    )
    .await?;

    run_stake_update(StakeUpdateOp::Deposit, usdc("100.0"), &accounts, processor.as_mut()).await?;

    // Can't request more than what we have
    let res = request_stake_update(StakeUpdateOp::Withdraw, usdc("100.1"), &accounts, processor.as_mut()).await;
    assert!(res.is_err());

    let balance_before = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;

    request_stake_update(StakeUpdateOp::Withdraw, usdc("100.0"), &accounts, processor.as_mut()).await?;
    yield_withdraw_by_investor(1, &accounts, processor.as_mut()).await?;
    yield_deposit_by_investor(usdc("50.0"), &accounts, processor.as_mut()).await?;

    progress_epoch_till(EpochStatus::Running, &accounts, processor.as_mut()).await?;

    // Completes what was already requested, if request can't be satisfied due to loss, give
    // whatever is remaining.
    complete_stake_update(&accounts, processor.as_mut()).await?;

    let balance_after = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;
    let diff = balance_after.checked_sub(balance_before).unwrap();

    assert_eq!(diff, usdc("50.0"));

    Ok(())
}

#[tokio::test]
async fn test_withdrawal_cancel_by_user() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    request_stake_update(StakeUpdateOp::Deposit, usdc("100.0"), &accounts, processor.as_mut()).await?;
    approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, usdc("100.0")).await?;
    complete_stake_update(&accounts, processor.as_mut()).await?;

    assert_balances(
        AssertBalances::deposit_complete(usdc("100.0")),
        &accounts,
        processor.as_mut(),
    )
    .await?;

    let balance_before = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;

    request_stake_update(StakeUpdateOp::Withdraw, usdc("10.0"), &accounts, processor.as_mut()).await?;
    cancel_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Withdraw, usdc("10.0")).await?;

    let balance_after = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;
    assert_eq!(balance_before, balance_after);

    request_stake_update(StakeUpdateOp::Withdraw, usdc("10.0"), &accounts, processor.as_mut()).await?;
    cancel_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Withdraw, usdc("10.0")).await?;

    let balance_after = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;
    assert_eq!(balance_before, balance_after);

    assert_balances(
        AssertBalances::deposit_complete(usdc("100.0")),
        &accounts,
        processor.as_mut(),
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_withdrawal_cancel_by_admin() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    request_stake_update(StakeUpdateOp::Deposit, usdc("100.0"), &accounts, processor.as_mut()).await?;
    approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, usdc("100.0")).await?;
    complete_stake_update(&accounts, processor.as_mut()).await?;

    assert_balances(
        AssertBalances::deposit_complete(usdc("100.0")),
        &accounts,
        processor.as_mut(),
    )
    .await?;

    let balance_before = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;

    request_stake_update(StakeUpdateOp::Withdraw, usdc("10.0"), &accounts, processor.as_mut()).await?;
    cancel_stake_update_by_admin(&accounts, processor.as_mut(), StakeUpdateOp::Withdraw, usdc("10.0")).await?;

    let balance_after = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;
    assert_eq!(balance_before, balance_after);

    request_stake_update(StakeUpdateOp::Withdraw, usdc("10.0"), &accounts, processor.as_mut()).await?;
    cancel_stake_update_by_admin(&accounts, processor.as_mut(), StakeUpdateOp::Withdraw, usdc("10.0")).await?;

    let balance_after = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;
    assert_eq!(balance_before, balance_after);

    assert_balances(
        AssertBalances::deposit_complete(usdc("100.0")),
        &accounts,
        processor.as_mut(),
    )
    .await?;

    Ok(())
}
