use crate::{accounts::Accounts, actions, setup::setup_test_runtime_without_init};
use anyhow::Result;
use nezha_staking_lib::state::EpochStatus;
use nezha_vrf_lib::instruction;
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

#[tokio::test]
async fn works() -> Result<()> {
    let mut accounts = Accounts::new();
    let mut processor = setup_test_runtime_without_init(&accounts).await?;

    // first init
    actions::init(&accounts, processor.as_mut()).await?;

    processor
        .send_ixns(
            &[instruction::rotate_key(
                &accounts.program_id,
                &accounts.super_admin.pubkey(),
                instruction::RotateKeyType::Admin,
                &accounts.random1.pubkey(),
            )],
            &[&accounts.super_admin],
        )
        .await?;

    actions::set_epoch_index_and_status(1, EpochStatus::Finalising, &accounts, processor.as_mut()).await?;
    let res = actions::request_vrf(1, &accounts, processor.as_mut()).await;
    assert!(res.is_err());

    std::mem::swap(&mut accounts.admin, &mut accounts.random1);
    actions::request_vrf(1, &accounts, processor.as_mut()).await?;

    Ok(())
}
