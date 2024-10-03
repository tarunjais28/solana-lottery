use crate::{accounts::Accounts, actions, setup::setup_test_runtime};
use anyhow::Result;
use nezha_staking_lib::state::EpochStatus;
use nezha_vrf_lib::state::{NezhaVrfRequest, NezhaVrfRequestStatus};
use solana_program_test::tokio;

#[tokio::test]
async fn request_vrf() -> Result<()> {
    let accounts = Accounts::new();
    let mut processor = setup_test_runtime(&accounts).await?;

    actions::set_epoch_index_and_status(1, EpochStatus::Finalising, &accounts, processor.as_mut()).await?;

    actions::request_vrf(1, &accounts, processor.as_mut()).await?;

    let vrf_request: NezhaVrfRequest = actions::get_vrf_request(1, &accounts, processor.as_mut()).await?;
    assert_eq!(vrf_request.status, NezhaVrfRequestStatus::Waiting);

    actions::consume_vrf(1, &accounts, processor.as_mut()).await?;
    let vrf_request: NezhaVrfRequest = actions::get_vrf_request(1, &accounts, processor.as_mut()).await?;
    assert_eq!(vrf_request.status, NezhaVrfRequestStatus::Success);

    // Test that it won't work for non-current epoch

    let res = actions::request_vrf(2, &accounts, processor.as_mut()).await;
    assert!(res.is_err());

    actions::set_epoch_index_and_status(2, EpochStatus::Finalising, &accounts, processor.as_mut()).await?;

    actions::request_vrf(2, &accounts, processor.as_mut()).await?;

    // Test that it won't work for non-finalising epoch

    actions::set_epoch_index_and_status(3, EpochStatus::Yielding, &accounts, processor.as_mut()).await?;

    let res = actions::request_vrf(3, &accounts, processor.as_mut()).await;
    assert!(res.is_err());

    actions::set_epoch_index_and_status(3, EpochStatus::Finalising, &accounts, processor.as_mut()).await?;

    actions::request_vrf(3, &accounts, processor.as_mut()).await?;

    Ok(())
}

#[tokio::test]
async fn request_vrf_race_condition() -> Result<()> {
    let accounts = Accounts::new();
    let mut processor = setup_test_runtime(&accounts).await?;

    actions::set_epoch_index_and_status(1, EpochStatus::Finalising, &accounts, processor.as_mut()).await?;
    actions::request_vrf(1, &accounts, processor.as_mut()).await?;

    actions::set_epoch_index_and_status(2, EpochStatus::Finalising, &accounts, processor.as_mut()).await?;
    actions::request_vrf(2, &accounts, processor.as_mut()).await?;

    actions::consume_vrf(1, &accounts, processor.as_mut()).await?;
    actions::consume_vrf(2, &accounts, processor.as_mut()).await?;

    let vrf_request_1: NezhaVrfRequest = actions::get_vrf_request(1, &accounts, processor.as_mut()).await?;
    let vrf_request_2: NezhaVrfRequest = actions::get_vrf_request(2, &accounts, processor.as_mut()).await?;
    assert_ne!(vrf_request_1.winning_combination, vrf_request_2.winning_combination);

    actions::set_epoch_index_and_status(3, EpochStatus::Finalising, &accounts, processor.as_mut()).await?;

    actions::request_vrf(3, &accounts, processor.as_mut()).await?;
    actions::consume_vrf(3, &accounts, processor.as_mut()).await?;

    let vrf_request_2: NezhaVrfRequest = actions::get_vrf_request(2, &accounts, processor.as_mut()).await?;
    let vrf_request_3: NezhaVrfRequest = actions::get_vrf_request(3, &accounts, processor.as_mut()).await?;
    assert_ne!(vrf_request_2.winning_combination, vrf_request_3.winning_combination);

    assert!(vrf_request_3.winning_combination.is_some());

    Ok(())
}

#[tokio::test]
async fn allow_retrying_stuck_vrf_request() -> Result<()> {
    let accounts = Accounts::new();
    let mut processor = setup_test_runtime(&accounts).await?;

    actions::set_epoch_index_and_status(1, EpochStatus::Finalising, &accounts, processor.as_mut()).await?;

    actions::request_vrf(1, &accounts, processor.as_mut()).await?;
    let vrf_request = actions::get_vrf_request(1, &accounts, processor.as_mut()).await?;
    assert!(vrf_request.winning_combination.is_none());

    actions::request_vrf(1, &accounts, processor.as_mut()).await?;
    let vrf_request = actions::get_vrf_request(1, &accounts, processor.as_mut()).await?;
    assert!(vrf_request.winning_combination.is_none());

    actions::consume_vrf(1, &accounts, processor.as_mut()).await?;
    let vrf_request = actions::get_vrf_request(1, &accounts, processor.as_mut()).await?;
    assert!(vrf_request.winning_combination.is_some());

    Ok(())
}
