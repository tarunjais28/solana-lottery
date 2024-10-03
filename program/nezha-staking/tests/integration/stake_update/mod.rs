use anyhow::{Context, Result};
use nezha_staking_lib::{
    accounts as ac,
    fixed_point::{test_utils::usdc, FPUSDC},
    state::{EpochStatus, Stake},
};
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

use crate::{accounts::Accounts, actions::*, setup::*};
use utils::*;

mod common;
mod deposits;
mod epoch_index;
mod utils;
mod withdrawal;

#[tokio::test]
async fn test_happy_path() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    run_stake_update(StakeUpdateOp::Deposit, usdc("100.0"), &accounts, processor.as_mut())
        .await
        .context("First deposit")?;

    yield_withdraw_by_investor(1, &accounts, processor.as_mut())
        .await
        .context("Yield Withdraw")?;
    yield_deposit_by_investor(usdc("100.0"), &accounts, processor.as_mut())
        .await
        .context("Yield deposit")?;

    progress_epoch_till(EpochStatus::Running, &accounts, processor.as_mut())
        .await
        .context("Cycle Epoch")?;

    run_stake_update(StakeUpdateOp::Deposit, usdc("50.0"), &accounts, processor.as_mut())
        .await
        .context("Second deposit")?;

    yield_withdraw_by_investor(1, &accounts, processor.as_mut())
        .await
        .context("Yield Withdraw 2")?;
    yield_deposit_by_investor(usdc("150.0"), &accounts, processor.as_mut())
        .await
        .context("Yield Deposit 2")?;

    progress_epoch_till(EpochStatus::Running, &accounts, processor.as_mut()).await?;

    run_stake_update(StakeUpdateOp::Withdraw, usdc("20.0"), &accounts, processor.as_mut())
        .await
        .context("Withdraw")?;

    assert_balances(
        AssertBalances::deposit_complete(usdc("130.0")),
        &accounts,
        processor.as_mut(),
    )
    .await?;

    Ok(())
}
