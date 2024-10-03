use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use nezha_staking_lib::{
    accounts as ac,
    instruction::{CreateEpochWinnersMetaArgs, TierWinnersMetaInput, WinnerInput},
};
use nezha_staking_lib::{
    fixed_point::test_utils::{fp, usdc},
    instruction,
    state::{InsuranceCfg, YieldSplitCfg},
};
use nezha_testing::mutations::{mutate, MutationTestIxn, MutationType};
use solana_program::pubkey::Pubkey;
use solana_program_test::tokio;
use solana_sdk::signer::Signer;
use spl_associated_token_account::get_associated_token_address;

use crate::{
    account_names,
    accounts::Accounts,
    actions::{random_tickets_info, set_winning_combination},
    setup::setup_test_runtime_without_init,
};

#[tokio::test]
async fn test_end_to_end_with_account_mutations() -> Result<()> {
    let accounts = Accounts::new();
    let account_names = account_names::build_account_names_map(&accounts);

    let owner_usdc = get_associated_token_address(&accounts.owner.pubkey(), &accounts.usdc_mint.pubkey());
    let investor_usdc = get_associated_token_address(&accounts.investor.pubkey(), &accounts.usdc_mint.pubkey());

    let mut processor = setup_test_runtime_without_init(&accounts).await?;

    // This is needed for publish winners to succeed
    set_winning_combination(1, [0u8; 6], &accounts, processor.as_mut()).await?;

    let account_refs = [
        &accounts.super_admin,
        &accounts.admin,
        &accounts.owner,
        &accounts.investor,
        &accounts.usdc_mint,
        &accounts.random1,
        &accounts.random2,
    ];

    // Some epoch specific stuff
    let epoch_index = 1;

    let winners_meta = CreateEpochWinnersMetaArgs {
        tier1_meta: TierWinnersMetaInput {
            total_num_winners: 1,
            total_num_winning_tickets: 1,
        },
        tier2_meta: TierWinnersMetaInput {
            total_num_winners: 18,
            total_num_winning_tickets: 18,
        },
        tier3_meta: TierWinnersMetaInput {
            total_num_winners: 1,
            total_num_winning_tickets: 1,
        },
    };

    let mut winners = Vec::new();
    winners.push(WinnerInput {
        index: 0,
        address: Pubkey::new_unique(),
        tier: 1,
        num_winning_tickets: 1,
    });

    for i in 0..=17 {
        winners.push(WinnerInput {
            index: i + 1,
            address: Pubkey::new_unique(),
            tier: 2,
            num_winning_tickets: 1,
        });
    }

    let num_winning_tickets: u32 = winners.iter().map(|w| w.num_winning_tickets).sum();

    winners.push(WinnerInput {
        index: 19,
        address: Pubkey::new_unique(),
        tier: 3,
        num_winning_tickets: 1,
    });

    let ixns = vec![
        MutationTestIxn {
            name: "init",
            ixn: instruction::init(
                &accounts.program_id,
                &accounts.super_admin.pubkey(),
                &accounts.admin.pubkey(),
                &accounts.investor.pubkey(),
                &accounts.usdc_mint.pubkey(),
                &accounts.nezha_vrf_program_id,
            ),
            signers: vec![&accounts.super_admin],
            skip_mutating: HashSet::from([
                (accounts.nezha_vrf_program_id, MutationType::Account),
                (accounts.super_admin.pubkey(), MutationType::Account),
                (accounts.admin.pubkey(), MutationType::Account),
                (accounts.investor.pubkey(), MutationType::Account),
            ]),
        },
        MutationTestIxn {
            name: "create epoch",
            ixn: instruction::create_epoch(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                epoch_index,
                {
                    let expected_end_at = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_secs() as i64
                        + 60 * 60 * 24;
                    expected_end_at
                },
                YieldSplitCfg {
                    jackpot: fp("100_000.0"),
                    insurance: InsuranceCfg {
                        premium: fp("3.0"),
                        probability: fp("0.0000000001"),
                    },
                    treasury_ratio: fp("0.5"),
                    tier2_prize_share: 2,
                    tier3_prize_share: 1,
                },
            ),
            signers: vec![&accounts.admin],
            skip_mutating: HashSet::new(),
        },
        MutationTestIxn {
            name: "deposit1",
            ixn: instruction::request_stake_update(
                &accounts.program_id,
                &accounts.owner.pubkey(),
                &owner_usdc,
                usdc("1.0").as_usdc_i64(),
            ),
            signers: vec![&accounts.owner],
            skip_mutating: HashSet::new(),
        },
        MutationTestIxn {
            name: "deposit1 approve",
            ixn: instruction::approve_stake_update(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                &accounts.owner.pubkey(),
                usdc("1.0").as_usdc_i64(),
            ),
            signers: vec![&accounts.admin],
            skip_mutating: HashSet::new(),
        },
        MutationTestIxn {
            name: "deposit1 cancel",
            ixn: instruction::cancel_stake_update(
                &accounts.program_id,
                Some(&accounts.admin.pubkey()),
                &accounts.owner.pubkey(),
                &owner_usdc,
                usdc("1.0").as_usdc_i64(),
            ),
            signers: vec![&accounts.admin],
            skip_mutating: HashSet::from([
                // Deposit can be cancelled by owner as signer instead of admin
                (
                    accounts.admin.pubkey(),
                    MutationType::AccountSpecific(accounts.owner.pubkey()),
                ),
            ]),
        },
        MutationTestIxn {
            name: "deposit2",
            ixn: instruction::request_stake_update(
                &accounts.program_id,
                &accounts.owner.pubkey(),
                &owner_usdc,
                usdc("1.0").as_usdc_i64(),
            ),
            signers: vec![&accounts.owner],
            skip_mutating: HashSet::new(),
        },
        MutationTestIxn {
            name: "deposit2 approve",
            ixn: instruction::approve_stake_update(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                &accounts.owner.pubkey(),
                usdc("1.0").as_usdc_i64(),
            ),
            signers: vec![&accounts.admin],
            skip_mutating: HashSet::new(),
        },
        MutationTestIxn {
            name: "deposit2 complete",
            ixn: instruction::complete_stake_update(
                &accounts.program_id,
                &accounts.random1.pubkey(),
                &accounts.owner.pubkey(),
                &owner_usdc,
            ),
            signers: vec![&accounts.random1],
            skip_mutating: HashSet::from([
                // For completing deposits, we don't care about owner usdc.
                // We use the same instruction for withdrawal, where we use owner_usdc
                // That's why it's present in the instruction
                (owner_usdc, MutationType::Writable),
                (owner_usdc, MutationType::Account),
                // We don't mind who the payer is
                (accounts.random1.pubkey(), MutationType::Account),
            ]),
        },
        MutationTestIxn {
            name: "withdraw1",
            ixn: instruction::request_stake_update(
                &accounts.program_id,
                &accounts.owner.pubkey(),
                &owner_usdc,
                -usdc("0.5").as_usdc_i64(),
            ),
            signers: vec![&accounts.owner],
            skip_mutating: HashSet::from([
                // For requesting withdrawals, we don't care about owner usdc.
                // We use the same instruction for requesting deposits, where we use owner_usdc
                // That's why it's present in the instruction
                (owner_usdc, MutationType::Writable),
                (
                    ac::pending_deposit_vault(&accounts.program_id).pubkey,
                    MutationType::Writable,
                ),
                (owner_usdc, MutationType::Account),
            ]),
        },
        MutationTestIxn {
            name: "withdraw1 complete",
            ixn: instruction::complete_stake_update(
                &accounts.program_id,
                &accounts.random1.pubkey(),
                &accounts.owner.pubkey(),
                &owner_usdc,
            ),
            signers: vec![&accounts.random1],
            skip_mutating: HashSet::from([
                // We don't write to pending deposit vault in withdrawal, only in deposits
                (
                    ac::pending_deposit_vault(&accounts.program_id).pubkey,
                    MutationType::Writable,
                ),
                // We don't mind who the payer is
                (accounts.random1.pubkey(), MutationType::Account),
                (accounts.random1.pubkey(), MutationType::Signer),
            ]),
        },
        MutationTestIxn {
            name: "manual investor - invest",
            ixn: instruction::yield_withdraw_by_investor(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                &investor_usdc,
                epoch_index,
                {
                    let num_tickets = num_winning_tickets * 100;
                    random_tickets_info(num_tickets as _)
                },
            ),
            signers: vec![&accounts.admin],
            skip_mutating: HashSet::new(),
        },
        MutationTestIxn {
            name: "manual investor - return",
            ixn: instruction::yield_deposit_by_investor(
                &accounts.program_id,
                &accounts.investor.pubkey(),
                &investor_usdc,
                epoch_index,
                usdc("100.0").as_usdc(),
            ),
            signers: vec![&accounts.investor],
            skip_mutating: HashSet::new(),
        },
        MutationTestIxn {
            name: "publish winners - upload metadata",
            ixn: instruction::create_epoch_winners_meta(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                epoch_index,
                winners_meta,
                &accounts.nezha_vrf_program_id,
            ),
            signers: vec![&accounts.admin],
            skip_mutating: HashSet::from([
                (ac::latest_epoch(&accounts.program_id).pubkey, MutationType::Writable),
                (
                    ac::epoch(&accounts.program_id, epoch_index).pubkey,
                    MutationType::Writable,
                ),
            ]),
        },
        MutationTestIxn {
            name: "publish winners - upload page 0",
            ixn: instruction::publish_winners(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                epoch_index,
                0,
                winners[0..10].to_vec(),
                &accounts.nezha_vrf_program_id,
            ),
            signers: vec![&accounts.admin],
            skip_mutating: HashSet::from([
                (ac::latest_epoch(&accounts.program_id).pubkey, MutationType::Writable),
                (ac::epoch(&accounts.program_id, 1).pubkey, MutationType::Writable),
            ]),
        },
        MutationTestIxn {
            name: "publish winners - upload page 1",
            ixn: instruction::publish_winners(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                epoch_index,
                1,
                winners[10..20].to_vec(),
                &accounts.nezha_vrf_program_id,
            ),
            signers: vec![&accounts.admin],
            skip_mutating: HashSet::new(),
        },
        // MutationTestIxn {
        //     name: "rotate keys",
        //     ixn: instruction::rotate_key(
        //         &accounts.program_id,
        //         &accounts.super_admin.pubkey(),
        //         instruction::RotateKeyType::Admin,
        //         &accounts.random1.pubkey(),
        //     ),
        //     signers: vec![&accounts.super_admin],
        //     skip_mutating: HashSet::from([(accounts.random1.pubkey(), MutationType::Account)]),
        // },
        // MutationTestIxn {
        //     name: "create epoch",
        //     ixn: instruction::create_epoch(
        //         &accounts.program_id,
        //         &accounts.random1.pubkey(),
        //         {
        //             epoch_index += 1;
        //             epoch_index
        //         },
        //         {
        //             let expected_end_at = SystemTime::now()
        //                 .duration_since(UNIX_EPOCH)
        //                 .expect("Time went backwards")
        //                 .as_secs() as i64
        //                 + 60 * 60 * 24;
        //             expected_end_at
        //         },
        //         YieldSplitCfg {
        //             jackpot: fp("100_000.0"),
        //             insurance: InsuranceCfg {
        //                 premium: fp("3.0"),
        //                 probability: fp("0.0001"),
        //             },
        //             treasury_ratio: fp("0.5"),
        //             tier2_prize_share: 2,
        //             tier3_prize_share: 1,
        //         },
        //     ),
        //     signers: vec![&accounts.random1],
        //     skip_mutating: HashSet::new(),
        // },
    ];

    for ixn in ixns {
        println!(">> Test: {}", ixn.name);
        for (mut_ixn, signers, mutation_name) in mutate(
            &ixn.ixn,
            &ixn.signers,
            &account_refs,
            &account_names,
            &ixn.skip_mutating,
        ) {
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
