use anyhow::Result;
use chrono::{DateTime, Utc};
use rand::Rng;
use rust_decimal::Decimal;
use service::{
    epoch::EpochRepository,
    model::epoch::{Epoch, EpochStatus},
    solana::YieldSplitCfg,
};
use solana_sdk::{signature::Keypair, signer::Signer};
use store::epochs::PostgresEpochRepository;

use crate::common;

fn create_epoch() -> Epoch {
    let mut rng = rand::thread_rng();
    let now = DateTime::parse_from_rfc3339(&Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
        .unwrap()
        .with_timezone(&Utc);
    Epoch {
        pubkey: Keypair::new().pubkey(),
        index: rng.gen_range(1_000_000..2_000_000),
        status: EpochStatus::Running,
        winning_combination: None,
        yield_split_cfg: YieldSplitCfg {
            jackpot: "1_000_000".parse().unwrap(),
            insurance: service::solana::InsuranceCfg {
                premium: 0u8.into(),
                probability: 0u8.into(),
            },
            treasury_ratio: "0.5".parse().unwrap(),
            tier2_prize_share: 1,
            tier3_prize_share: 1,
        },
        total_invested: None,
        returns: None,
        started_at: now,
        expected_end_at: now,
        ended_at: None,
        draw_enabled: None,
    }
}

#[tokio::test]
async fn test_create_or_update_epoch() -> Result<()> {
    let pool = common::setup().await;
    let epoch_repository = PostgresEpochRepository::new(pool.clone());
    let expected = create_epoch();
    epoch_repository.create_or_update_epoch(&expected).await?;
    let stored = epoch_repository.by_index(expected.index).await?;
    assert!(stored.is_some());
    let stored = stored.expect("could not get stored epoch");
    assert_eq!(stored, expected);

    let expected = Epoch {
        winning_combination: Some([1, 2, 3, 4, 5, 6]),
        ..expected
    };
    epoch_repository.create_or_update_epoch(&expected).await?;
    let stored = epoch_repository.by_index(expected.index).await?;
    assert!(stored.is_some());
    let stored = stored.expect("could not get stored epoch");
    assert_eq!(stored, expected);

    // Remove the epoch from the database
    let client = pool.get().await?;
    client
        .execute(
            "DELETE FROM epoch WHERE epoch_index = $1",
            &[&Decimal::from(expected.index)],
        )
        .await?;

    Ok(())
}
