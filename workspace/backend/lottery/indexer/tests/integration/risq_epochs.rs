use anyhow::Result;
use chrono::{DateTime, Duration, Timelike, Utc};
use indexer::db::risq_epochs::{Epoch, EpochStore, PostgresEpochStore};
use rand::{thread_rng, Rng};

use crate::common;

async fn get_repo() -> PostgresEpochStore {
    let pool = common::setup_store().await;
    PostgresEpochStore::new(pool)
}

/// Returns Utc::now() with sub-second part zero because it gets truncated on round-trip to the DB
/// and cause the tests to fail
fn utc_now() -> DateTime<Utc> {
    Utc::now().with_nanosecond(0).unwrap()
}

#[tokio::test]
async fn test_save_and_load() -> Result<()> {
    let mut rng = thread_rng();

    let repo = get_repo().await;
    let epoch_index = rng.gen();

    // Sanity check: Initially doesn't exist
    let epoch_from_db = repo.load(epoch_index).await?;
    assert!(epoch_from_db.is_none());

    // Save and load works
    let epoch = Epoch {
        index: epoch_index,
        tickets_generated_at: None,
        draw_info_sent_to_risq_at: None,
    };
    repo.save(&epoch).await?;

    let epoch_from_db = repo.load(epoch_index).await?;
    assert_eq!(epoch_from_db, Some(epoch));

    // Update works
    let epoch = Epoch {
        index: epoch_index,
        tickets_generated_at: Some(utc_now()),
        draw_info_sent_to_risq_at: Some(utc_now() + Duration::hours(1)),
    };
    repo.save(&epoch).await?;

    let epoch_from_db = repo.load(epoch_index).await?;
    assert_eq!(epoch_from_db.as_ref(), Some(&epoch));

    // Sanity check: a different non existent epoch
    let epoch2_from_db = repo.load(epoch_index + 1).await?;
    assert!(epoch2_from_db.is_none());

    let epoch2 = Epoch {
        index: epoch_index + 1,
        tickets_generated_at: Some(utc_now()),
        draw_info_sent_to_risq_at: Some(utc_now() + Duration::hours(1)),
    };
    repo.save(&epoch2).await?;

    // It got saved
    let epoch2_from_db = repo.load(epoch_index + 1).await?;
    assert_eq!(epoch2_from_db, Some(epoch2));

    // Saving of epoch2 didn't affect the previously saved epoch
    let epoch_from_db = repo.load(epoch_index).await?;
    assert_eq!(epoch_from_db.as_ref(), Some(&epoch));

    // Setting to null works
    let epoch2 = Epoch {
        index: epoch_index + 1,
        tickets_generated_at: None,
        draw_info_sent_to_risq_at: None,
    };
    repo.save(&epoch2).await?;

    let epoch2_from_db = repo.load(epoch_index + 1).await?;
    assert_eq!(epoch2_from_db, Some(epoch2));

    // Update of epoch2 didn't affect the previously saved epoch
    let epoch_from_db = repo.load(epoch_index).await?;
    assert_eq!(epoch_from_db, Some(epoch));

    Ok(())
}
