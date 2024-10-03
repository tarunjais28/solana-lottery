use anyhow::{anyhow, Result};
use rand::Rng;
use service::{epoch::FPUSDC, model::prize::Prize, prize::PrizeRepository};
use solana_sdk::pubkey::Pubkey;
use store::prizes::PostgresPrizeRepository;

use crate::common;

fn create_prize() -> Prize {
    let mut rng = rand::thread_rng();
    Prize {
        wallet: Pubkey::new_unique(),
        epoch_index: rng.gen(),
        page: rng.gen::<i32>().abs() as _,
        winner_index: rng.gen::<i32>().abs() as _,
        tier: rng.gen(),
        amount: FPUSDC::from_usdc(rng.gen()),
        claimable: rng.gen_bool(0.5),
        claimed: rng.gen_bool(0.5),
    }
}

async fn get_repo() -> PostgresPrizeRepository {
    let pool = common::setup().await;
    PostgresPrizeRepository::new(pool, 100)
}

#[tokio::test]
async fn test_by_wallet() -> Result<()> {
    let wallet = Pubkey::new_unique();
    let pool = common::setup().await;
    let client = pool.get().await?;
    client
        .execute("DELETE FROM prize WHERE wallet = $1", &[&wallet.to_string()])
        .await?;
    let repo = get_repo().await;
    let mut expected_prizes = vec![];
    for _ in 0..10 {
        let prize = Prize {
            wallet,
            ..create_prize()
        };
        repo.upsert_prizes(&[prize.clone()]).await?;
        expected_prizes.push(prize);
    }
    let mut prizes = repo.by_wallet(&wallet, 10, 0).await?;
    expected_prizes.sort();
    prizes.sort();
    assert_eq!(prizes, expected_prizes);

    Ok(())
}

#[tokio::test]
async fn test_by_wallet_epoch_and_tier() -> Result<()> {
    let repo = get_repo().await;
    let prize = create_prize();
    repo.upsert_prizes(&[prize.clone()]).await?;
    let stored_prize = repo
        .by_wallet_epoch_and_tier(&prize.wallet, prize.epoch_index, prize.tier)
        .await?;
    assert!(stored_prize.is_some(), "{:?}", stored_prize);
    assert_eq!(stored_prize.unwrap(), prize);
    Ok(())
}

#[tokio::test]
async fn test_total_prize_by_wallet() -> Result<()> {
    let pool = common::setup().await;
    let client = pool.get().await?;
    let wallet = Pubkey::new_unique();
    client
        .execute("DELETE FROM prize WHERE wallet = $1", &[&wallet.to_string()])
        .await?;
    let repo = get_repo().await;
    let mut expected_total = FPUSDC::zero();
    for _ in 0..10 {
        let prize = Prize {
            wallet,
            ..create_prize()
        };
        repo.upsert_prizes(&[prize.clone()]).await?;
        expected_total = expected_total
            .checked_add(prize.amount)
            .ok_or_else(|| anyhow!("overflow"))?;
    }
    let total = repo.total_prize_by_wallet(&wallet).await?;
    assert_eq!(total, expected_total);
    Ok(())
}

#[tokio::test]
async fn test_upsert_prize() -> Result<()> {
    let repo = get_repo().await;
    let prize = create_prize();
    repo.upsert_prizes(&[prize.clone()]).await?;
    let stored_prize = repo
        .by_wallet_epoch_and_tier(&prize.wallet, prize.epoch_index, prize.tier)
        .await?;
    assert!(stored_prize.is_some(), "{:?}", stored_prize);
    assert_eq!(stored_prize.unwrap(), prize);
    Ok(())
}

#[tokio::test]
async fn test_upsert_prizes() -> Result<()> {
    let repo = get_repo().await;
    let mut prizes = vec![];
    for _ in 0..10 {
        prizes.push(create_prize());
    }
    repo.upsert_prizes(&prizes).await?;
    for prize in prizes {
        let stored_prize = repo
            .by_wallet_epoch_and_tier(&prize.wallet, prize.epoch_index, prize.tier)
            .await
            .expect("Failed to get prize");
        assert!(stored_prize.is_some(), "{:?}", stored_prize);
        assert_eq!(stored_prize.unwrap(), prize);
    }
    Ok(())
}
