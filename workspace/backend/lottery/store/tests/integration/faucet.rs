use anyhow::Result;
use chrono::{DateTime, Utc};
use service::{faucet::FaucetRepository, model::faucet::LatestMintTransaction};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use store::faucet::PostgresFaucetRepository;

use crate::common;

async fn get_repo() -> PostgresFaucetRepository {
    let pool = common::setup().await;
    PostgresFaucetRepository::new(pool)
}

#[tokio::test]
async fn test_latest_mint_transaction() -> Result<()> {
    let repo = get_repo().await;
    let wallet = Pubkey::new_unique();
    let amount = rand::random::<u64>();
    let transaction_time = chrono::Utc::now();
    let transaction_id = Signature::new_unique().into();
    let actual = repo
        .store_latest_transaction(&wallet, amount, &transaction_time, &transaction_id)
        .await?;
    let expected = LatestMintTransaction {
        wallet,
        amount,
        // we have to make sure the time is in the same format as the db (Postgres resolves to microseconds)
        transaction_time: DateTime::parse_from_rfc3339(
            &transaction_time.to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
        )?
        .with_timezone(&Utc),
        transaction_id,
    };
    assert_eq!(expected, actual);

    // we should be able to update the latest transaction with the same wallet
    let res = repo
        .store_latest_transaction(
            &wallet,
            rand::random(),
            &chrono::Utc::now(),
            &Signature::new_unique().into(),
        )
        .await;
    assert!(res.is_ok(), "{res:?}");

    Ok(())
}
