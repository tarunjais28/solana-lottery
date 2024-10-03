use anyhow::Result;
use rand::{thread_rng, Rng};
use service::{
    model::{
        stake_update::{StakeUpdate, StakeUpdateState, StakeUpdateType},
        transaction::TransactionId,
    },
    solana::FPUSDC,
    stake::StakeUpdateRepository,
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use store::{get_client, stake_update::PostgresStakeUpdateRepository};

use crate::common;

fn create_stake_update() -> StakeUpdate {
    let mut rng = thread_rng();
    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let amount: u64 = rng.gen();
    let currency = "USDC".to_string();
    let state = match rng.gen_range(0..=2) {
        0u8 => StakeUpdateState::Pending,
        1 => StakeUpdateState::Failed,
        _ => StakeUpdateState::Completed,
    };
    let type_ = match rng.gen_range(0..=1) {
        0u8 => StakeUpdateType::Deposit,
        _ => StakeUpdateType::Withdraw,
    };
    let transaction_id = Some(Signature::new_unique().into());
    StakeUpdate {
        owner,
        amount: FPUSDC::from_usdc(amount),
        state,
        type_,
        currency,
        mint,
        transaction_id,
    }
}

#[tokio::test]
async fn test_store() -> Result<()> {
    let pool = common::setup().await;
    let stake_update_repository = PostgresStakeUpdateRepository::new(pool);
    let stake_update = create_stake_update();
    let stored = stake_update_repository.store(&stake_update).await?;
    assert_eq!(stored, stake_update);

    Ok(())
}

#[tokio::test]
async fn test_store_without_transaction_id() -> Result<()> {
    let pool = common::setup().await;
    let stake_update_repository = PostgresStakeUpdateRepository::new(pool);
    let mut stake_update = create_stake_update();
    stake_update.transaction_id = None;
    let stored = stake_update_repository.store(&stake_update).await?;
    assert!(stored.transaction_id.is_none());

    Ok(())
}

#[tokio::test]
async fn test_by_wallet() -> Result<()> {
    let pool = common::setup().await;
    let stake_update_repository = PostgresStakeUpdateRepository::new(pool);
    let owner = Pubkey::new_unique();
    let mut stake_updates = Vec::new();
    for _ in 0..10 {
        let stake_update = StakeUpdate {
            owner,
            ..create_stake_update()
        };
        stake_update_repository.store(&stake_update).await?;
        stake_updates.push(stake_update);
    }
    let stored = stake_update_repository.by_wallet(&owner).await?;
    for stake_update in stake_updates {
        assert!(stored.contains(&stake_update));
    }

    Ok(())
}

#[tokio::test]
async fn test_by_transaction_id() -> Result<()> {
    let pool = common::setup().await;
    let stake_update_repository = PostgresStakeUpdateRepository::new(pool);
    let stake_update = create_stake_update();
    stake_update_repository.store(&stake_update).await?;
    let res = stake_update_repository
        .by_transaction_id(&stake_update.transaction_id.to_owned().unwrap())
        .await?;
    assert!(res.is_some());
    let stored = res.expect("could not find stake_update");
    assert_eq!(stored, stake_update);

    Ok(())
}

#[tokio::test]
async fn test_by_transaction_id_not_found() -> Result<()> {
    let pool = common::setup().await;
    let transaction_id = TransactionId::from(Signature::new_unique());
    let client = get_client(&pool).await?;
    client
        .execute(
            "DELETE FROM stake_update WHERE transaction_id = $1",
            &[&transaction_id.0],
        )
        .await?;
    let stake_update_repository = PostgresStakeUpdateRepository::new(pool);
    let res = stake_update_repository.by_transaction_id(&transaction_id).await?;
    assert!(res.is_none());

    Ok(())
}
