use std::collections::HashMap;

use anyhow::{Context, Result};
use nezha_staking_lib::{
    error::StakingError,
    fixed_point::test_utils::{fp, usdc},
    francium,
    instruction::{CreateEpochWinnersMetaArgs, TierWinnersMetaInput},
    state::EpochStatus,
};
use solana_sdk::signer::Signer;

use crate::{account_names, accounts::Accounts, actions::*, processors};

use nezha_testing::{
    solana_emulator::ProcessorFn,
    solana_test_runtime::{self, ErrorFn, SolanaTestRuntime, TestRuntimeType},
};

pub const SOLS_TO_MINT: u64 = 100;
pub const USDC_TO_MINT: u64 = 1000;

pub async fn new_test_runtime(accounts: &Accounts) -> Result<Box<dyn SolanaTestRuntime + Send + Sync + 'static>> {
    let runtime_type = if cfg!(feature = "test-bpf") {
        TestRuntimeType::BPF {
            program_name: "nezha_staking".into(),
            program_id: accounts.program_id,
        }
    } else {
        processors::francium::init(fp(1.2));

        let account_names = account_names::build_account_names_map(accounts);
        let processors = HashMap::from([
            (
                accounts.program_id,
                nezha_staking::processor::process_instruction as ProcessorFn,
            ),
            (
                francium::constants::LENDING_PROGRAM_ID,
                processors::francium::process_francium_lending,
            ),
            (
                francium::constants::LENDING_REWARDS_PROGRAM_ID,
                processors::francium::process_francium_rewards,
            ),
        ]);
        TestRuntimeType::Emulated {
            processors,
            account_names,
        }
    };
    let nezha_staking_error_fn: ErrorFn = |err_code| Some(Box::new(StakingError::from(err_code)));
    let errors = HashMap::from([(accounts.program_id, nezha_staking_error_fn)]);
    let program_ids = vec![accounts.nezha_vrf_program_id];
    let runtime = solana_test_runtime::new_test_runtime(runtime_type, errors, &program_ids).await?;
    Ok(runtime)
}

pub async fn setup_test_runtime(accounts: &Accounts) -> Result<Box<dyn SolanaTestRuntime + Send + Sync>> {
    let mut runtime = setup_test_runtime_without_init(accounts).await?;

    init(&accounts, runtime.as_mut()).await.context("Init")?;

    Ok(runtime)
}

pub async fn setup_test_runtime_without_init(accounts: &Accounts) -> Result<Box<dyn SolanaTestRuntime + Send + Sync>> {
    let mut runtime = new_test_runtime(&accounts).await?;

    create_mint(&accounts.usdc_mint, &accounts.admin.pubkey(), None, runtime.as_mut())
        .await
        .context("Create USDC mint")?;
    for actor in [
        &accounts.owner,
        &accounts.super_admin,
        &accounts.admin,
        &accounts.investor,
        &accounts.random1,
        &accounts.random2,
    ] {
        mint_sols(&actor.pubkey(), SOLS_TO_MINT, runtime.as_mut())
            .await
            .context("Mint SOLs")?;
        create_token_account(&actor.pubkey(), &accounts.usdc_mint.pubkey(), runtime.as_mut())
            .await
            .context("Create USDC ATA")?;
        mint_tokens(
            &actor.pubkey(),
            USDC_TO_MINT * 1_000_000,
            &accounts.usdc_mint.pubkey(),
            &accounts.admin,
            runtime.as_mut(),
        )
        .await
        .context("Mint USDC")?;
    }

    Ok(runtime)
}

pub async fn progress_epoch_till<'a>(
    status: EpochStatus,
    accounts: &'a Accounts,
    runtime: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    loop {
        let latest_epoch = get_latest_epoch(accounts, runtime).await?;
        if latest_epoch.status == status {
            break;
        }
        match latest_epoch.status {
            EpochStatus::Ended => {
                create_epoch(accounts, random_yield_split_cfg(), runtime).await?;
            }
            EpochStatus::Running => {
                request_stake_update(StakeUpdateOp::Deposit, usdc("100"), accounts, runtime).await?;
                approve_stake_update(accounts, runtime, StakeUpdateOp::Deposit, usdc("100")).await?;
                complete_stake_update(accounts, runtime).await?;
                yield_withdraw_by_investor(1, accounts, runtime).await?;
            }
            EpochStatus::Yielding => {
                yield_deposit_by_investor(usdc("200"), accounts, runtime).await?;
            }
            EpochStatus::Finalising => {
                let latest_epoch = get_latest_epoch(accounts, runtime).await?;
                set_winning_combination(latest_epoch.index, [0u8; 6], accounts, runtime).await?;

                let no_winners = TierWinnersMetaInput {
                    total_num_winners: 0,
                    total_num_winning_tickets: 0,
                };
                let meta_args = CreateEpochWinnersMetaArgs {
                    tier1_meta: no_winners.clone(),
                    tier2_meta: no_winners.clone(),
                    tier3_meta: no_winners.clone(),
                };
                publish_epoch_winners(&meta_args, &[], accounts, runtime).await?;
            }
        }
    }
    Ok(())
}
