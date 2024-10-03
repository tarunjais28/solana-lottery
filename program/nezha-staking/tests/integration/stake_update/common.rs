use nezha_staking_lib::fixed_point::test_utils::fp;

use super::*;

#[tokio::test]
async fn test_cant_request_zero() -> Result<()> {
    for op in [StakeUpdateOp::Deposit, StakeUpdateOp::Withdraw] {
        let (accounts, mut processor) = setup().await?;
        run_stake_update(StakeUpdateOp::Deposit, fp("200.0"), &accounts, processor.as_mut()).await?;

        let res = request_stake_update(op, fp("0.0"), &accounts, processor.as_mut()).await;
        assert!(res.is_err());
    }

    Ok(())
}

#[tokio::test]
async fn test_cant_approve_without_request() -> Result<()> {
    for op in [StakeUpdateOp::Deposit, StakeUpdateOp::Withdraw] {
        let (accounts, mut processor) = setup().await?;
        run_stake_update(StakeUpdateOp::Deposit, fp("200.0"), &accounts, processor.as_mut()).await?;

        let res = approve_stake_update(&accounts, processor.as_mut(), op, fp("1.0")).await;
        assert!(res.is_err());
    }

    Ok(())
}

#[tokio::test]
async fn test_cant_complete_when_epoch_not_running() -> Result<()> {
    for op in [StakeUpdateOp::Deposit, StakeUpdateOp::Withdraw] {
        let (accounts, mut processor) = setup().await?;
        run_stake_update(StakeUpdateOp::Deposit, fp("200.0"), &accounts, processor.as_mut()).await?;

        request_stake_update(op, usdc("50.0"), &accounts, processor.as_mut()).await?;

        if op == StakeUpdateOp::Deposit {
            approve_stake_update(&accounts, processor.as_mut(), op, usdc("50.0")).await?;
        }

        yield_withdraw_by_investor(1, &accounts, processor.as_mut()).await?;

        let res = complete_stake_update(&accounts, processor.as_mut()).await;
        assert!(res.is_err());

        yield_deposit_by_investor(usdc("200.0"), &accounts, processor.as_mut()).await?;

        let res = complete_stake_update(&accounts, processor.as_mut()).await;
        assert!(res.is_err());

        progress_epoch_till(EpochStatus::Ended, &accounts, processor.as_mut()).await?;

        let res = complete_stake_update(&accounts, processor.as_mut()).await;
        assert!(res.is_err());
    }
    Ok(())
}

#[tokio::test]
async fn test_cant_request_twice() -> Result<()> {
    for op in [StakeUpdateOp::Deposit, StakeUpdateOp::Withdraw] {
        let (accounts, mut processor) = setup().await?;
        run_stake_update(StakeUpdateOp::Deposit, fp("200.0"), &accounts, processor.as_mut()).await?;

        request_stake_update(op, usdc("100.0"), &accounts, processor.as_mut()).await?;

        let res = request_stake_update(op, usdc("50.0"), &accounts, processor.as_mut()).await;
        assert!(res.is_err());

        if op == StakeUpdateOp::Deposit {
            approve_stake_update(&accounts, processor.as_mut(), op, usdc("100.0")).await?;
        }

        let res = request_stake_update(op, usdc("50.0"), &accounts, processor.as_mut()).await;
        assert!(res.is_err());
    }

    Ok(())
}

#[tokio::test]
async fn test_cant_approve_twice() -> Result<()> {
    for op in [StakeUpdateOp::Deposit, StakeUpdateOp::Withdraw] {
        let (accounts, mut processor) = setup().await?;
        run_stake_update(StakeUpdateOp::Deposit, fp("200.0"), &accounts, processor.as_mut()).await?;

        request_stake_update(op, usdc("50.0"), &accounts, processor.as_mut()).await?;
        if op == StakeUpdateOp::Deposit {
            approve_stake_update(&accounts, processor.as_mut(), op, usdc("50.0")).await?;
        }

        let res = approve_stake_update(&accounts, processor.as_mut(), op, usdc("50.0")).await;
        assert!(res.is_err());
    }

    Ok(())
}

#[tokio::test]
async fn test_cant_complete_twice() -> Result<()> {
    for op in [StakeUpdateOp::Deposit, StakeUpdateOp::Withdraw] {
        let (accounts, mut processor) = setup().await?;
        run_stake_update(StakeUpdateOp::Deposit, fp("200.0"), &accounts, processor.as_mut()).await?;

        request_stake_update(op, usdc("50.0"), &accounts, processor.as_mut()).await?;
        if op == StakeUpdateOp::Deposit {
            approve_stake_update(&accounts, processor.as_mut(), op, usdc("50.0")).await?;
        }
        complete_stake_update(&accounts, processor.as_mut()).await?;

        let res = complete_stake_update(&accounts, processor.as_mut()).await;

        assert!(res.is_err());
    }

    Ok(())
}

#[tokio::test]
async fn test_incorrect_values() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    request_stake_update(StakeUpdateOp::Deposit, usdc("100.0"), &accounts, processor.as_mut()).await?;
    let res = approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, usdc("100.1")).await;
    assert!(res.is_err());
    approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, usdc("100.0")).await?;
    complete_stake_update(&accounts, processor.as_mut()).await?;

    request_stake_update(StakeUpdateOp::Deposit, usdc("200.0"), &accounts, processor.as_mut()).await?;
    let res = approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Withdraw, usdc("200.0")).await;
    assert!(res.is_err());
    approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, usdc("200.0")).await?;
    complete_stake_update(&accounts, processor.as_mut()).await?;

    request_stake_update(StakeUpdateOp::Withdraw, usdc("150.0"), &accounts, processor.as_mut()).await?;
    let res = approve_stake_update(&accounts, processor.as_mut(), StakeUpdateOp::Deposit, usdc("150.0")).await;
    assert!(res.is_err());
    complete_stake_update(&accounts, processor.as_mut()).await?;

    Ok(())
}
