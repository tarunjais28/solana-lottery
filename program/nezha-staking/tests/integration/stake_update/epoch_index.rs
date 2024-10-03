use super::*;

#[tokio::test]
async fn test_stake_created_epoch_index_is_correct() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    run_stake_update(StakeUpdateOp::Deposit, usdc("100.0"), &accounts, processor.as_mut()).await?;
    let stake: Stake = get_data(
        *ac::stake(&accounts.program_id, &accounts.owner.pubkey()),
        processor.as_mut(),
    )
    .await?;

    // Set correctly on first deposit
    assert_eq!(stake.created_epoch_index, 1);

    progress_epoch_till(EpochStatus::Ended, &accounts, processor.as_mut()).await?;
    progress_epoch_till(EpochStatus::Running, &accounts, processor.as_mut()).await?;

    let latest_epoch = get_latest_epoch(&accounts, processor.as_mut()).await?;
    assert_eq!(latest_epoch.index, 2);

    run_stake_update(StakeUpdateOp::Deposit, usdc("100.0"), &accounts, processor.as_mut()).await?;

    // Doesn't get overwritten on subsequent deposits
    let stake: Stake = get_data(
        *ac::stake(&accounts.program_id, &accounts.owner.pubkey()),
        processor.as_mut(),
    )
    .await?;
    assert_eq!(stake.created_epoch_index, 1);

    Ok(())
}

#[tokio::test]
async fn test_stake_updated_epoch_index_is_correct() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    run_stake_update(StakeUpdateOp::Deposit, usdc("100.0"), &accounts, processor.as_mut()).await?;

    let stake: Stake = get_data(
        *ac::stake(&accounts.program_id, &accounts.owner.pubkey()),
        processor.as_mut(),
    )
    .await?;

    // Set correctly on first deposit
    assert_eq!(stake.updated_epoch_index, 1);

    progress_epoch_till(EpochStatus::Ended, &accounts, processor.as_mut()).await?;
    progress_epoch_till(EpochStatus::Running, &accounts, processor.as_mut()).await?;

    let latest_epoch = get_latest_epoch(&accounts, processor.as_mut()).await?;
    assert_eq!(latest_epoch.index, 2);

    run_stake_update(StakeUpdateOp::Deposit, usdc("100.0"), &accounts, processor.as_mut()).await?;

    // gets updated on subsequent deposits
    let stake: Stake = get_data(
        *ac::stake(&accounts.program_id, &accounts.owner.pubkey()),
        processor.as_mut(),
    )
    .await?;
    assert_eq!(stake.updated_epoch_index, 2);

    progress_epoch_till(EpochStatus::Ended, &accounts, processor.as_mut()).await?;
    progress_epoch_till(EpochStatus::Running, &accounts, processor.as_mut()).await?;

    let latest_epoch = get_latest_epoch(&accounts, processor.as_mut()).await?;
    assert_eq!(latest_epoch.index, 3);

    run_stake_update(StakeUpdateOp::Withdraw, usdc("100.0"), &accounts, processor.as_mut()).await?;

    // gets updated on withdraws as well
    let stake: Stake = get_data(
        *ac::stake(&accounts.program_id, &accounts.owner.pubkey()),
        processor.as_mut(),
    )
    .await?;
    assert_eq!(stake.updated_epoch_index, 3);

    Ok(())
}
