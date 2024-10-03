// use std::{collections::HashMap, sync::Arc, time::Duration};
//
// use anyhow::Result;
// use borsh::BorshDeserialize;
// use common::{
//     setup_api, setup_context, setup_pubsub, setup_store,
//     util::{airdrop_sol, airdrop_usdc},
// };
// use indexer::{
//     db::risq_epochs::{EpochStore, PostgresEpochStore},
//     indexer::{
//         deposits::{IndexerDeposits, IndexerDepositsConfig},
//         util::{send_and_confirm_transaction, SolanaProgramContext},
//     },
// };
// use nezha_staking::{
//     instruction,
//     state::{Stake, PREFIX, VAULT_PREFIX},
// };
// use rand::{thread_rng, Rng};
// use solana_sdk::{
//     pubkey::Pubkey,
//     signer::{keypair::Keypair, Signer},
// };
// use spl_associated_token_account::get_associated_token_address;
// use spl_token::ui_amount_to_amount;

// mod common;
//
// async fn stake_balance(context: &SolanaProgramContext, owner: &Pubkey) -> Result<u64> {
//     let stake_pubkey = Pubkey::for_stake(owner, &context.staking_program_id);
//     while context.rpc_client.get_account(&stake_pubkey).await.is_err() {
//         tokio::time::sleep(Duration::from_millis(500)).await;
//     }
//     let mut data: &[u8] = &context.rpc_client.get_account_data(&stake_pubkey).await?;
//     let decoded = Stake::deserialize(&mut data)?;
//
//     Ok(decoded.amount)
// }

// Test commented by Farseen.
// Due to time constraints I won't be able to automate the tests for deposit attempts after
// converting it to use the websocket listener in addition to less frequent polling.
// I promise to revisit this later.
//
// How should we test this:
// - Setup API
// - Setup context
// - Setup pubsub:
//   - Copy this code to create a proxy server. (https://github.com/jamesmcm/basic_tcp_proxy/blob/master/src/lib.rs)
//   - Test for fault tolerance by restarting the proxy server midway and making sure that the
//   indexer reconnects to the pubsub.
//  - Test the following:
//      - Deposit created before the indexer runs are approved.
//      - Deposit created during the indexer runs are approved.
//          - Block the websocket using proxy for a bit and make sure the backup mechanism is
//            running.
//          - Test that the websocket loop recovers once the proxy is back up.
//
// #[tokio::test]
// async fn test_auto_approve_deposits() -> Result<()> {
//     let nezha_api = setup_api();
//     let solana_pubsub = setup_pubsub();
//     let epoch_store: Box<dyn EpochStore + Send + Sync> = Box::new(PostgresEpochStore::new(setup_store().await));
//     let indexer = IndexerDeposits {
//         epoch_store,
//         nezha_api,
//         solana_pubsub,
//         config: IndexerDepositsConfig {
//             sleep_between_batches: Default::default(),
//             deposit_batch_size: 10,
//         },
//     };
//
//     let mut rng = thread_rng();
//     let mut wallets_with_amounts = HashMap::new();
//     for _ in 0..10 {
//         let keypair = Keypair::new();
//         let amount = ui_amount_to_amount(rng.gen_range(1..=1000u64) as f64, 6);
//         wallets_with_amounts.insert(keypair.to_base58_string(), amount);
//     }
//     let wallets_with_amounts = Arc::new(wallets_with_amounts);
//
//     let context = setup_context();
//     for (keypair_string, &amount) in wallets_with_amounts.iter() {
//         let owner_keypair = Keypair::from_base58_string(keypair_string);
//         let owner_pubkey = owner_keypair.pubkey();
//         let ctx = context.clone();
//         let _ = tokio::spawn(async move {
//             // airdrop some SOL to owner, otherwise they can't pay for attempt deposit instruction
//             airdrop_sol(&ctx.rpc_client, &owner_pubkey).await.unwrap();
//             airdrop_usdc(&ctx, &owner_pubkey, amount).await.unwrap();
//             attempt_deposit(&ctx, &owner_keypair, amount).await.unwrap();
//         });
//     }
//
//     // wait for all deposit attempts to appear
//     while indexer.nezha_api.all_pending_deposits().await?.len() < 10 {
//         tokio::time::sleep(Duration::from_millis(500)).await;
//     }
//     indexer.auto_approve_deposits().await?;
//
//     let mut check_handles = Vec::new();
//     for (keypair_string, &amount) in wallets_with_amounts.iter() {
//         let wallet = Keypair::from_base58_string(keypair_string).pubkey();
//         let ctx = context.clone();
//         let handle = tokio::spawn(async move {
//             let stake_balance = stake_balance(&ctx, &wallet).await.unwrap();
//             assert_eq!(stake_balance, amount);
//         });
//         check_handles.push(handle);
//     }
//     for handle in check_handles {
//         handle.await?;
//     }
//
//     Ok(())
// }
