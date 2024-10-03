use crate::{accounts::Accounts, actions, setup::setup_test_runtime_without_init};
use anyhow::Result;
use solana_program_test::tokio;
use solana_sdk::{signature::Keypair, signer::Signer};

#[tokio::test]
async fn no_init_twice() -> Result<()> {
    let mut accounts = Accounts::new();
    let mut processor = setup_test_runtime_without_init(&accounts).await?;

    // first init
    actions::init(&accounts, processor.as_mut()).await?;

    // second init
    let res = actions::init(&accounts, processor.as_mut()).await;
    assert!(res.is_err());

    // init, but replace super admin and admin
    accounts.super_admin = Keypair::new();
    accounts.admin = Keypair::new();
    actions::mint_sols(&accounts.super_admin.pubkey(), 1000, processor.as_mut()).await?;

    let res = actions::init(&accounts, processor.as_mut()).await;
    assert!(res.is_err());

    Ok(())
}
