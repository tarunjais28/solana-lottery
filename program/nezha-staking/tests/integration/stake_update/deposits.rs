use nezha_staking_lib::fixed_point::test_utils::fp;

use super::*;

#[tokio::test]
async fn test_deposit_not_approved() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    let balance_before = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;

    let deposit_amt: FPUSDC = usdc("100.0");
    request_stake_update(StakeUpdateOp::Deposit, deposit_amt, &accounts, processor.as_mut()).await?;

    let balance_after = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;
    let diff = balance_before.checked_sub(balance_after).unwrap();
    assert_eq!(diff, deposit_amt);

    assert_balances(
        AssertBalances::deposit_pending(deposit_amt),
        &accounts,
        processor.as_mut(),
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_deposit_not_completed() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    let balance_before = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;
    let deposit_amt: FPUSDC = usdc("100.0");
    request_stake_update(StakeUpdateOp::Deposit, deposit_amt, &accounts, processor.as_mut()).await?;
    approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, deposit_amt).await?;
    let balance_after = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;
    let diff = balance_before.checked_sub(balance_after).unwrap();

    assert_eq!(diff, deposit_amt);

    assert_balances(
        AssertBalances::deposit_pending(deposit_amt),
        &accounts,
        processor.as_mut(),
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_deposit_completed() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    let balance_before = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;
    let deposit_amt: FPUSDC = usdc("100.0");
    request_stake_update(StakeUpdateOp::Deposit, deposit_amt, &accounts, processor.as_mut()).await?;

    let balance_after = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;
    let diff = balance_before.checked_sub(balance_after).unwrap();

    assert_eq!(diff, deposit_amt);

    approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, deposit_amt).await?;
    complete_stake_update(&accounts, processor.as_mut()).await?;

    assert_balances(
        AssertBalances::deposit_complete(deposit_amt),
        &accounts,
        processor.as_mut(),
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn test_cant_complete_before_approval() -> Result<()> {
    let (accounts, mut processor) = setup().await?;
    run_stake_update(StakeUpdateOp::Deposit, fp("200.0"), &accounts, processor.as_mut()).await?;

    request_stake_update(StakeUpdateOp::Deposit, usdc("50.0"), &accounts, processor.as_mut()).await?;
    let res = complete_stake_update(&accounts, processor.as_mut()).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn test_deposit_cancel_by_user() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    let balance_before = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;

    let deposit_amt: FPUSDC = usdc("100.0");
    request_stake_update(StakeUpdateOp::Deposit, deposit_amt, &accounts, processor.as_mut()).await?;
    approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, deposit_amt).await?;
    cancel_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, deposit_amt).await?;

    let balance_after = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;

    assert_eq!(balance_before, balance_after);

    assert_balances(AssertBalances::deposit_cancelled(), &accounts, processor.as_mut()).await?;

    Ok(())
}

#[tokio::test]
async fn test_deposit_cancel_by_admin() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    let balance_before = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;

    let deposit_amt: FPUSDC = usdc("100.0");
    request_stake_update(StakeUpdateOp::Deposit, deposit_amt, &accounts, processor.as_mut()).await?;
    approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, deposit_amt).await?;
    cancel_stake_update_by_admin(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, deposit_amt).await?;

    let balance_after = get_owner_usdc_balance(&accounts, processor.as_mut()).await?;

    assert_eq!(balance_before, balance_after);

    assert_balances(AssertBalances::deposit_cancelled(), &accounts, processor.as_mut()).await?;

    Ok(())
}

#[tokio::test]
async fn test_complete_two_deposits() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    request_stake_update(StakeUpdateOp::Deposit, usdc("100.0"), &accounts, processor.as_mut()).await?;
    approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, usdc("100.0")).await?;
    complete_stake_update(&accounts, processor.as_mut()).await?;

    yield_withdraw_by_investor(1, &accounts, processor.as_mut()).await?;
    yield_deposit_by_investor(usdc("100.0"), &accounts, processor.as_mut()).await?;

    progress_epoch_till(EpochStatus::Running, &accounts, processor.as_mut()).await?;

    request_stake_update(StakeUpdateOp::Deposit, usdc("50.0"), &accounts, processor.as_mut()).await?;
    approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, usdc("50.0")).await?;
    complete_stake_update(&accounts, processor.as_mut()).await?;

    assert_balances(
        AssertBalances::deposit_complete(usdc("150.0")),
        &accounts,
        processor.as_mut(),
    )
    .await?;

    Ok(())
}
