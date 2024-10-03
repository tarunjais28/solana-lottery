use std::collections::HashMap;

use anchor_lang::Discriminator;
use anyhow::{Context, Result};
use nezha_vrf_lib::error::NezhaVrfError;
use solana_program::system_program;
use solana_sdk::signer::Signer;
use switchboard_v2::OracleQueueAccountData;

use crate::{account_names, accounts::Accounts, actions::*, processors};

use nezha_testing::{
    solana_emulator::ProcessorFn,
    solana_test_runtime::{self, Account, ErrorFn, SolanaTestRuntime, TestRuntimeType},
};

pub const SOLS_TO_MINT: u64 = 100;

pub async fn new_test_runtime(accounts: &Accounts) -> Result<Box<dyn SolanaTestRuntime + Send + Sync + 'static>> {
    let runtime_type = if cfg!(feature = "test-bpf") {
        TestRuntimeType::BPF {
            program_name: "nezha_vrf".into(),
            program_id: accounts.program_id,
        }
    } else {
        let account_names = account_names::build_account_names_map(accounts);
        let processors = HashMap::from([
            (
                accounts.program_id,
                nezha_vrf::processor::process_instruction as ProcessorFn,
            ),
            (switchboard_v2::ID, processors::switchboard::process as ProcessorFn),
        ]);
        TestRuntimeType::Emulated {
            processors,
            account_names,
        }
    };
    let nezha_vrf_error_fn: ErrorFn = |err_code| Some(Box::new(NezhaVrfError::from(err_code)));
    let errors = HashMap::from([(accounts.program_id, nezha_vrf_error_fn)]);
    let program_ids = vec![
        switchboard_v2::ID,
        spl_associated_token_account::ID,
        spl_token::ID,
        system_program::ID,
    ];
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

    for actor in [
        &accounts.super_admin,
        &accounts.admin,
        &accounts.random1,
        &accounts.random2,
    ] {
        mint_sols(&actor.pubkey(), SOLS_TO_MINT, runtime.as_mut())
            .await
            .context("Mint SOLs")?;
    }

    let queue_account = OracleQueueAccountData {
        name: [96u8; 32],
        metadata: [0u8; 64],
        authority: accounts.switchboard_queue_authority,
        oracle_timeout: 0,
        reward: 0,
        min_stake: 0,
        slashing_enabled: false,
        variance_tolerance_multiplier: switchboard_v2::SwitchboardDecimal { mantissa: 0, scale: 0 },
        feed_probation_period: 0,
        curr_idx: 0,
        size: 0,
        gc_idx: 0,
        consecutive_feed_failure_limit: 0,
        consecutive_oracle_failure_limit: 0,
        unpermissioned_feeds_enabled: false,
        unpermissioned_vrf_enabled: false,
        curator_reward_cut: switchboard_v2::SwitchboardDecimal { mantissa: 0, scale: 0 },
        lock_lease_funding: false,
        mint: accounts.switchboard_queue_mint,
        enable_buffer_relayers: false,
        _ebuf: [0u8; 968],
        max_size: 0,
        data_buffer: accounts.switchboard_queue_data_buffer,
    };
    let mut queue_account_data = Vec::new();
    queue_account_data.extend_from_slice(&OracleQueueAccountData::DISCRIMINATOR);
    queue_account_data.extend_from_slice(&bytemuck::bytes_of(&queue_account));

    runtime.set_account(
        &accounts.switchboard_queue,
        &Account {
            lamports: 1,
            owner: switchboard_v2::ID,
            data: queue_account_data,
        },
    );

    // create_mint(
    //     &accounts.switchboard_queue_mint,
    //     &accounts.switchboard_queue_authority.pubkey(),
    //     None,
    //     runtime.as_mut(),
    // )
    // .await?;
    Ok(runtime)
}
