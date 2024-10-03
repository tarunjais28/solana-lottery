use std::{
    collections::HashMap,
    env,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::NaiveDate;
use risq_api_client::{
    resources::{
        configs::{self, DrawConfig},
        draw::objects::DrawId,
        entry,
    },
    AppError, HttpStatusCode, RISQClient,
};
use rust_decimal::Decimal;

// TODO add tests for exact winning key

// Constants

pub(crate) const PRODUCT_ID: &'static str = "nezha-draw";
pub(crate) const LICENSEE_ID: &'static str = "1";
pub(crate) const ENTRY_REF: &'static str = "entry1";
pub(crate) const PLAYER_ID: &'static str = "player1";

fn config() -> configs::nezha::Nezha {
    configs::nezha::Nezha
}

/// Generate draw id for an open draw.
fn draw_id() -> DrawId {
    config().next_draw_id(chrono::Utc::now().date().naive_utc())
}

/// Generate draw id for a closed draw.
fn draw_id_closed() -> DrawId {
    // This is a date in past, so the draw is closed
    // Also, Victor from RISQ did some tweaks to make sure we can test all the functions of a
    // closed draw using this draw_id
    config()
        .draw_id_try_from_draw_date(NaiveDate::from_ymd(2022, 04, 16))
        .unwrap()
}

fn expect_env(var: &str) -> String {
    env::var(var).expect(&format!("Failed to load env var {var}"))
}

fn get_client() -> RISQClient {
    // Load .env files if they exist.
    // We don't want to fail if .env doesn't exist because sometimes we would want to give the env
    // vars without the .env file, for example in CI.
    let _ = dotenv::dotenv();

    let _ = env_logger::builder().format_timestamp(None).is_test(true).try_init();

    risq_api_client::new_client(
        expect_env("RISQ_TEST_BASE_URL"),
        expect_env("RISQ_TEST_PARTNER_ID"),
        expect_env("RISQ_TEST_API_KEY"),
    )
}

// Tests

#[tokio::test]
async fn test_risq_api() -> Result<(), anyhow::Error> {
    send_draw_information().await?;
    get_draw_information().await?;
    get_draw_winning_key().await?;
    get_draw_prizes().await?;
    get_draw_prizes_estimate().await?;
    send_entry().await?;
    delete_entry().await?;
    send_entry_batch().await?;
    retrieve_winners().await?;
    retrieve_partner_balance_details().await?;
    Ok(())
}

async fn send_draw_information() -> Result<(), anyhow::Error> {
    let draw_info = config().draw_info(Decimal::new(1000, 0));
    let draw = get_client()
        .draw
        .send_draw_information(PRODUCT_ID, &draw_id(), &draw_info)
        .await?;
    dbg!(draw);
    Ok(())
}

async fn get_draw_information() -> Result<(), anyhow::Error> {
    let resp = get_client().draw.get_draw_information(PRODUCT_ID, &draw_id()).await?;
    dbg!(&resp);
    Ok(())
}

async fn get_draw_winning_key() -> Result<(), anyhow::Error> {
    let resp = get_client()
        .draw
        .get_draw_winning_key(PRODUCT_ID, &draw_id_closed())
        .await?;
    dbg!(&resp);
    let wk = config().decode_winning_key(&resp)?;
    assert_eq!(wk, [1, 2, 3, 4, 5, 1]);
    Ok(())
}

async fn get_draw_prizes() -> Result<(), anyhow::Error> {
    let resp = get_client()
        .balance
        .get_draw_prizes(PRODUCT_ID, &draw_id_closed())
        .await?;
    dbg!(&resp);
    Ok(())
}

async fn get_draw_prizes_estimate() -> Result<(), anyhow::Error> {
    let resp = get_client()
        .balance
        .get_draw_prizes_estimate(PRODUCT_ID, &draw_id_closed())
        .await?;
    dbg!(&resp);
    assert_ne!(resp.estimate_prizes.len(), 0);
    Ok(())
}

async fn send_entry() -> Result<(), anyhow::Error> {
    let draw_id = draw_id();
    let entry = entry::objects::Entry {
        draw_id: draw_id.clone(),
        licensee_id: LICENSEE_ID.to_string(),
        player_id: PLAYER_ID.to_string(),
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        entry_ref: ENTRY_REF.to_string(),
        variant: config().encode_entry_variant(&mut [[1, 2, 3, 4, 5, 1]].iter().map(|e| e.as_ref()))?,
    };

    let resp = get_client().entry.send_entry(&entry).await?;
    dbg!(&resp);

    let entry_id = match resp.status {
        entry::objects::EntryReceiptStatus::Ok { entry_id, .. } => entry_id,
        x => panic!("Expected Ok, received {x:?}"),
    };

    let resp = get_client().entry.retrieve_entry(&entry_id).await?;
    dbg!(&resp);

    Ok(())
}

async fn delete_entry() -> Result<(), anyhow::Error> {
    let entry_ref = "entry_to_delete";
    let entry = entry::objects::Entry {
        draw_id: draw_id(),
        licensee_id: LICENSEE_ID.to_string(),
        player_id: PLAYER_ID.to_string(),
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        entry_ref: entry_ref.to_string(),
        variant: config().encode_entry_variant(&mut [[1, 2, 3, 4, 5, 1]].iter().map(|e| e.as_ref()))?,
    };

    let resp = get_client().entry.send_entry(&entry).await?;
    dbg!(&resp);

    let entry_id = match resp.status {
        entry::objects::EntryReceiptStatus::Ok { entry_id, .. } => entry_id,
        x => panic!("Expected Ok, received {x:?}"),
    };

    let resp = get_client().entry.delete_entry(&entry_id).await?;
    dbg!(&resp);

    let resp = get_client().entry.retrieve_entry(&entry_id).await;
    dbg!(&resp);
    match resp {
        Err(AppError::ServerError { http_code, code, .. }) => {
            assert_eq!(http_code, HttpStatusCode::NotFound);
            assert_eq!(code, "EntryNotFound".to_string());
        }
        x => panic!("Expected NotFound, received {x:?}"),
    }
    Ok(())
}

async fn send_entry_batch() -> Result<(), anyhow::Error> {
    let entry = entry::objects::Entry {
        draw_id: draw_id(),
        licensee_id: LICENSEE_ID.to_string(),
        player_id: PLAYER_ID.to_string(),
        timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        entry_ref: ENTRY_REF.to_string(),
        variant: config().encode_entry_variant(&mut [[1, 2, 3, 4, 5, 1]].iter().map(|e| e.as_ref()))?,
    };

    let entry_batch = entry::objects::EntryBatch {
        batch_id: "1".to_string(),
        entries: vec![entry],
        signatures: HashMap::new(),
    };

    let resp = get_client().entry.send_entry_batch(&entry_batch).await?;
    dbg!(&resp);
    let entry_id = match &resp.receipts[0].status {
        entry::objects::EntryReceiptStatus::Ok { entry_id, .. } => entry_id.clone(),
        x => panic!("Expected Ok, received {x:?}"),
    };
    let resp = get_client().entry.retrieve_entry(&entry_id).await?;
    dbg!(&resp);
    Ok(())
}

async fn retrieve_winners() -> Result<(), anyhow::Error> {
    let resp = get_client()
        .entry
        .retrieve_winners(PRODUCT_ID, &draw_id_closed())
        .await?;
    dbg!(&resp);
    Ok(())
}

async fn retrieve_partner_balance_details() -> Result<(), anyhow::Error> {
    let resp = get_client().balance.retrieve_partner_balance_details().await?;
    dbg!(&resp);
    Ok(())
}
