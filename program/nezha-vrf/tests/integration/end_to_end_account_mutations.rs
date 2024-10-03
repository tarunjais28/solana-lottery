use std::collections::HashSet;

use anyhow::{Context, Result};
use nezha_staking_lib::{accounts as staking_ac, state::EpochStatus};
use nezha_testing::mutations::{mutate, MutationTestIxn, MutationType};
use nezha_vrf_lib::instruction;
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

use crate::{
    account_names, accounts::Accounts, actions::set_epoch_index_and_status, setup::setup_test_runtime_without_init,
};

#[tokio::test]
async fn test_end_to_end_with_account_mutations() -> Result<()> {
    let accounts = Accounts::new();
    let account_names = account_names::build_account_names_map(&accounts);

    let mut processor = setup_test_runtime_without_init(&accounts).await?;

    let account_refs = [
        &accounts.super_admin,
        &accounts.admin,
        &accounts.random1,
        &accounts.random2,
    ];

    // Some epoch specific stuff
    let epoch_index = 2;

    set_epoch_index_and_status(epoch_index, EpochStatus::Finalising, &accounts, processor.as_mut()).await?;

    let ixns = vec![
        MutationTestIxn {
            name: "init",
            ixn: instruction::init(
                &accounts.program_id,
                &accounts.super_admin.pubkey(),
                &accounts.admin.pubkey(),
                &switchboard_v2::ID,
                &accounts.switchboard_queue,
                &accounts.switchboard_queue_authority,
                &accounts.switchboard_queue_mint,
                &accounts.nezha_staking_program_id,
            ),
            signers: vec![&accounts.super_admin, &accounts.admin],
            skip_mutating: HashSet::from([
                (accounts.super_admin.pubkey(), MutationType::Account),
                (accounts.admin.pubkey(), MutationType::Account),
                (accounts.switchboard_queue_authority, MutationType::Account),
                (accounts.nezha_staking_program_id, MutationType::Account),
            ]),
        },
        MutationTestIxn {
            name: "request vrf",
            ixn: instruction::request_vrf(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                &switchboard_v2::ID,
                &accounts.switchboard_queue,
                &accounts.switchboard_queue_authority,
                &accounts.switchboard_queue_mint,
                &accounts.switchboard_queue_data_buffer,
                &staking_ac::latest_epoch(&accounts.nezha_staking_program_id),
                epoch_index,
            ),
            signers: vec![&accounts.admin],
            skip_mutating: HashSet::from([
                (accounts.switchboard_queue, MutationType::Account),
                (accounts.switchboard_queue_authority, MutationType::Account),
                (accounts.switchboard_queue_mint, MutationType::Account),
            ]),
        },
        MutationTestIxn {
            name: "consume vrf",
            ixn: instruction::consume_vrf(&accounts.program_id, epoch_index),
            signers: vec![],
            skip_mutating: HashSet::from([]),
        },
        MutationTestIxn {
            name: "rotate keys",
            ixn: instruction::rotate_key(
                &accounts.program_id,
                &accounts.super_admin.pubkey(),
                instruction::RotateKeyType::Admin,
                &accounts.random1.pubkey(),
            ),
            signers: vec![&accounts.super_admin],
            skip_mutating: HashSet::from([(accounts.random1.pubkey(), MutationType::Account)]),
        },
    ];

    for ixn in ixns {
        println!(">>>> Test: {}", ixn.name);
        for (mut_ixn, signers, mutation_name) in mutate(
            &ixn.ixn,
            &ixn.signers,
            &account_refs,
            &account_names,
            &ixn.skip_mutating,
        ) {
            println!(">> Mutation: {}", mutation_name);
            let res = processor.send_ixns(&[mut_ixn], &signers).await;
            assert!(
                res.is_err(),
                "Test supposed to fail but didn't: {}: {}",
                ixn.name,
                &mutation_name
            );
        }
        processor
            .send_ixns(&[ixn.ixn], &ixn.signers)
            .await
            .context(format!("Error executing ixn {}", ixn.name))?;
    }

    Ok(())
}
