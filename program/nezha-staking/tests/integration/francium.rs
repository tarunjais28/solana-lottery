use crate::{
    accounts::Accounts,
    actions::{self, create_mint, random_tickets_info, random_yield_split_cfg},
    setup::setup_test_runtime_without_init,
};
use anyhow::Result;
use nezha_staking_lib::{
    fixed_point::test_utils::fp,
    francium::constants::{set_mints, Mints},
    instruction,
};
use solana_program_test::tokio;
use solana_sdk::{signature::Keypair, signer::Signer};

use nezha_testing::solana_test_runtime::SolanaTestRuntime;

struct FranciumAccounts {
    share_token_mint: Keypair,
    rewards_token_mint: Keypair,
    rewards_token_b_mint: Keypair,
}

impl FranciumAccounts {
    fn new() -> Self {
        Self {
            share_token_mint: Keypair::new(),
            rewards_token_mint: Keypair::new(),
            rewards_token_b_mint: Keypair::new(),
        }
    }
}

fn get_mints(accounts: &Accounts, francium_accounts: &FranciumAccounts) -> Mints {
    Mints {
        usdc_mint: accounts.usdc_mint.pubkey(),
        share_token_mint: francium_accounts.share_token_mint.pubkey(),
        rewards_token_mint: francium_accounts.rewards_token_mint.pubkey(),
        rewards_token_b_mint: francium_accounts.rewards_token_b_mint.pubkey(),
    }
}

async fn create_mints(
    accounts: &Accounts,
    francium_accounts: &FranciumAccounts,
    processor: &mut (dyn SolanaTestRuntime + Send + Sync),
) -> Result<()> {
    for mint in &[
        &francium_accounts.share_token_mint,
        &francium_accounts.rewards_token_mint,
        &francium_accounts.rewards_token_b_mint,
    ] {
        create_mint(mint, &accounts.admin.pubkey(), None, processor).await?;
    }
    Ok(())
}

#[tokio::test]
async fn happy_path() -> Result<()> {
    // We can't test francium using bpf tests because we don't have their code nor blobs.
    if cfg!(feature = "test-bpf") {
        return Ok(());
    }

    let accounts = Accounts::new();
    let francium_accounts = FranciumAccounts::new();
    let mints = get_mints(&accounts, &francium_accounts);
    set_mints(mints.clone());

    let mut processor = setup_test_runtime_without_init(&accounts).await?;

    create_mints(&accounts, &francium_accounts, processor.as_mut()).await?;

    actions::init(&accounts, processor.as_mut()).await?;
    francium_init(&accounts, &mints, processor.as_mut()).await?;
    actions::create_epoch(&accounts, random_yield_split_cfg(), processor.as_mut()).await?;

    actions::request_stake_update(
        actions::StakeUpdateOp::Deposit,
        fp("100.0"),
        &accounts,
        processor.as_mut(),
    )
    .await?;
    actions::approve_stake_update(
        &accounts,
        processor.as_mut(),
        actions::StakeUpdateOp::Deposit,
        fp("100.0"),
    )
    .await?;
    actions::complete_stake_update(&accounts, processor.as_mut()).await?;

    francium_invest(1, 1, &accounts, &mints, processor.as_mut()).await?;
    francium_withdraw(1, &accounts, &mints, processor.as_mut()).await?;

    Ok(())
}

pub async fn francium_init(
    accounts: &Accounts,
    mints: &Mints,
    processor: &mut (dyn SolanaTestRuntime + Send + Sync),
) -> Result<()> {
    processor
        .send_ixns(
            &[instruction::francium_init(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                mints,
            )],
            &[&accounts.admin],
        )
        .await
}

pub async fn francium_invest(
    epoch_index: u64,
    num_tickets: u64,
    accounts: &Accounts,
    mints: &Mints,
    processor: &mut (dyn SolanaTestRuntime + Send + Sync),
) -> Result<()> {
    processor
        .send_ixns(
            &[instruction::francium_invest(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                epoch_index,
                random_tickets_info(num_tickets),
                mints,
            )],
            &[&accounts.admin],
        )
        .await
}

pub async fn francium_withdraw(
    epoch_index: u64,
    accounts: &Accounts,
    mints: &Mints,
    processor: &mut (dyn SolanaTestRuntime + Send + Sync),
) -> Result<()> {
    processor
        .send_ixns(
            &[instruction::francium_withdraw(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                epoch_index,
                mints,
            )],
            &[&accounts.admin],
        )
        .await
}
