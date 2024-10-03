#![cfg(feature = "test-bpf")]
use anchor_lang::AccountDeserialize;
use chrono::Utc;
use solana_program_test::{tokio, ProgramTest};
use solana_sdk::{clock::Clock, signer::Signer};
use staking::UserBalance;

mod common;
mod util;

#[tokio::test]
async fn create_deposit() {
    let program_id = staking::id();
    let mut program_test = ProgramTest::new("staking", program_id.clone(), None);

    let owner = common::setup_keypair(&mut program_test, 100);
    let mut ctx = program_test.start_with_context().await;

    let now = Utc::now().timestamp();
    let clock = Clock {
        unix_timestamp: now,
        ..Default::default()
    };
    ctx.set_sysvar(&clock);

    let amount = 100_000_000;
    let deposit = util::deposit(&mut ctx, owner, amount).await;

    let balance_acc = ctx
        .banks_client
        .get_account(deposit.user_balance)
        .await
        .unwrap()
        .unwrap();
    let user_balance = UserBalance::try_deserialize_unchecked(&mut balance_acc.data.as_ref()).unwrap();
    let UserBalance {
        owner,
        mint,
        amount,
        last_deposit_ts,
    } = user_balance;
    assert_eq!(owner, deposit.owner.pubkey());
    assert_eq!(mint, deposit.mint.pubkey());
    assert_eq!(amount, deposit.amount);
    assert_eq!(now, last_deposit_ts);
}
