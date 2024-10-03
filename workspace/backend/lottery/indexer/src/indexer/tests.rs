use anyhow::Result;
use chrono::{DateTime, NaiveDate, Utc};
use pretty_assertions::assert_eq;
use rust_decimal::Decimal;
use solana_sdk::pubkey::Pubkey;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use crate::dummy_draw_config::DummyDrawConfig;
use crate::mocks::StakeUpdateState;
use crate::nezha_api::{DrawEnabled, Epoch as APIEpoch, EpochStatus, NezhaAPI, TieredPrizes};

use super::super::mocks;
use super::deposits::polling::{AcceptDeposits, AcceptDepositsConfig, GenerateTickets};
use super::risq::{IndexerRisq, IndexerRisqConfig, IndexerRisqState};

struct IndexerSetup {
    epoch_store: mocks::EpochStoreImpl,
    draw_routes: mocks::DrawRoutesImpl,
    entry_routes: mocks::EntryRoutesImpl,
    nezha_api: mocks::NezhaApiImpl,
    indexer_risq_state: IndexerRisqState,
}

impl IndexerSetup {
    fn new() -> Self {
        let epoch_store = mocks::EpochStoreImpl::default();
        let draw_routes = mocks::DrawRoutesImpl::new();
        let entry_routes = mocks::EntryRoutesImpl::new();
        let nezha_api = mocks::NezhaApiImpl::new();

        let indexer_risq_state = IndexerRisqState::new();

        Self {
            epoch_store,
            draw_routes,
            entry_routes,
            nezha_api,
            indexer_risq_state,
        }
    }

    fn new_risq_indexer(&self, batch_size: usize) -> IndexerRisq {
        let amount = Decimal::from(1000u64);
        let config = IndexerRisqConfig {
            risq_product_id: "product".to_string(),
            risq_licensee_id: "licensee_id".to_string(),
            ticket_batch_size: batch_size,
            nezha_prize_amount: amount,
            sleep_between_batches: Duration::from_millis(10),
            draw_config: Box::new(DummyDrawConfig),
        };
        IndexerRisq {
            nezha_api: Box::new(self.nezha_api.clone()),
            epoch_store: Box::new(self.epoch_store.clone()),
            risq_draw_routes: Box::new(self.draw_routes.clone()),
            risq_entry_routes: Box::new(self.entry_routes.clone()),
            config,
        }
    }

    fn new_deposits_indexer(&self, batch_size: usize) -> AcceptDeposits {
        let config = AcceptDepositsConfig {
            batch_size,
            batch_gap: Duration::from_millis(10),
        };
        AcceptDeposits {
            nezha_api: Arc::new(self.nezha_api.clone()),
            config,
        }
    }

    fn new_tickets_indexer(&self) -> GenerateTickets {
        GenerateTickets {
            nezha_api: Box::new(self.nezha_api.clone()),
        }
    }

    /// Reset indexer state
    fn restart_risq_indexer(&mut self) {
        self.indexer_risq_state = IndexerRisqState::new();
    }

    fn insert_wallets(&self, count: usize) {
        for _ in 0..count {
            let wallet = Pubkey::new_unique();
            self.nezha_api.insert_wallet(wallet);
        }
    }

    fn insert_unapproved_wallets(&self, count: usize) {
        for _ in 0..count {
            let wallet = Pubkey::new_unique();
            self.nezha_api.insert_unapproved_wallet(wallet);
        }
    }

    fn set_latest_epoch(&self, epoch: Option<APIEpoch>) {
        let mut latest_epoch = self.nezha_api.latest_epoch.lock().unwrap();
        *latest_epoch = epoch;
    }

    fn assert_draws_sent(&self, count: usize) {
        let draws = self.draw_routes.draws.lock().unwrap();
        assert_eq!(draws.len(), count);
    }

    fn assert_entries_sent(&self, count: usize) {
        let entries = self.entry_routes.entries.lock().unwrap();
        assert_eq!(entries.len(), count);
    }

    fn assert_approved(&self, count: usize) {
        let wallets = self.nezha_api.wallets.lock().unwrap();
        let approved_count = wallets
            .iter()
            .filter(|(_, state)| *state != StakeUpdateState::PendingApproval)
            .count();
        assert_eq!(approved_count, count);
    }

    fn assert_tickets_generated(&self, count: usize) {
        let tickets = self.nezha_api.tickets.lock().unwrap();
        assert_eq!(tickets.len(), count);
    }

    fn set_success_count(&self, count: usize) {
        *self.entry_routes.success_count.lock().unwrap() = count;
    }
}

fn nezha_draw_date() -> NaiveDate {
    NaiveDate::from_ymd(2022, 4, 02)
}

fn nezha_draw_date_utc() -> DateTime<Utc> {
    DateTime::from_utc(nezha_draw_date().and_hms(0, 0, 0), Utc)
}

#[tokio::test]
async fn when_latest_epoch_is_not_present_doesnt_crash_and_no_draws_are_sent() -> Result<()> {
    let mut setup = IndexerSetup::new();
    let indexer = setup.new_risq_indexer(10);

    // Sanity check: Latest epoch is none.
    assert!(setup.nezha_api.get_latest_epoch().await?.is_none());

    indexer.run(&mut setup.indexer_risq_state).await?;

    setup.assert_draws_sent(0);

    Ok(())
}

#[tokio::test]
async fn sends_the_epoch_to_risq() -> Result<()> {
    let mut setup = IndexerSetup::new();
    let indexer = setup.new_risq_indexer(10);
    setup.set_latest_epoch(Some(APIEpoch {
        index: 1,
        pubkey: Pubkey::default(),
        status: EpochStatus::Running,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));

    indexer.run(&mut setup.indexer_risq_state).await?;

    // exactly one draw is sent
    let draws = setup.draw_routes.draws.lock().unwrap();
    assert_eq!(draws.len(), 1);

    let epochs = setup.epoch_store.data.lock().unwrap();
    assert_eq!(epochs.len(), 1);
    let epoch = &epochs[0];

    assert!(epoch.draw_info_sent_to_risq_at.is_some());

    Ok(())
}

#[tokio::test]
async fn sends_the_epoch_even_if_yielding() -> Result<()> {
    let mut setup = IndexerSetup::new();
    let indexer = setup.new_risq_indexer(10);
    setup.set_latest_epoch(Some(APIEpoch {
        index: 1,
        pubkey: Pubkey::default(),
        status: EpochStatus::Yielding,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));

    indexer.run(&mut setup.indexer_risq_state).await?;

    // exactly one draw is sent
    setup.assert_draws_sent(1);

    let epochs = setup.epoch_store.data.lock().unwrap();
    assert_eq!(epochs.len(), 1);

    let epoch = &epochs[0];
    assert!(epoch.draw_info_sent_to_risq_at.is_some());

    Ok(())
}

#[tokio::test]
async fn sends_the_unsubmitted_tickets() -> Result<()> {
    let batch_size = 5;
    let num_sequences = 7;
    let epoch_index = 10;

    let mut setup = IndexerSetup::new();
    let indexer = setup.new_risq_indexer(batch_size);

    setup.set_latest_epoch(Some(APIEpoch {
        index: epoch_index,
        pubkey: Pubkey::default(),
        status: EpochStatus::Yielding,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));

    setup.insert_wallets(num_sequences);
    setup.nezha_api.generate_tickets_for_all().await?;

    setup.set_success_count(2);
    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_entries_sent(2);

    {
        let entries = setup.entry_routes.entries.lock().unwrap();
        assert_ne!(&entries[0], &entries[1]);
    }

    setup.set_success_count(10);
    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_entries_sent(2 + 5); // 2 + batch_size

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_entries_sent(2 + 5); // no new entries are sent

    Ok(())
}

#[tokio::test]
async fn sends_epoch_when_changed() -> Result<()> {
    let mut setup = IndexerSetup::new();
    let indexer = setup.new_risq_indexer(100);

    setup.set_latest_epoch(Some(APIEpoch {
        index: 1,
        pubkey: Pubkey::default(),
        status: EpochStatus::Running,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_draws_sent(1);

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_draws_sent(1);

    setup.set_latest_epoch(Some(APIEpoch {
        index: 1,
        pubkey: Pubkey::default(),
        status: EpochStatus::Yielding,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));
    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_draws_sent(1);

    setup.set_latest_epoch(Some(APIEpoch {
        index: 2,
        pubkey: Pubkey::default(),
        status: EpochStatus::Running,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));
    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_draws_sent(2);

    setup.set_latest_epoch(Some(APIEpoch {
        index: 3,
        pubkey: Pubkey::default(),
        status: EpochStatus::Running,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));
    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_draws_sent(3);

    Ok(())
}

#[tokio::test]
async fn survives_restart_between_sending_epoch_and_sending_tickets() -> Result<()> {
    let mut setup = IndexerSetup::new();
    let indexer = setup.new_risq_indexer(10);
    setup.set_latest_epoch(Some(APIEpoch {
        index: 1,
        pubkey: Pubkey::default(),
        status: EpochStatus::Running,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_draws_sent(1);

    setup.restart_risq_indexer();

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_draws_sent(1);

    // Set to yielding and generate tickets

    setup.set_latest_epoch(Some(APIEpoch {
        index: 1,
        pubkey: Pubkey::default(),
        status: EpochStatus::Yielding,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));
    setup.insert_wallets(10);
    setup.nezha_api.generate_tickets_for_all().await?;
    setup.set_success_count(100);

    setup.restart_risq_indexer();

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_draws_sent(1);
    setup.assert_entries_sent(10);
    Ok(())
}

#[tokio::test]
async fn survives_restart_between_sending_two_epochs() -> Result<()> {
    let mut setup = IndexerSetup::new();
    let indexer = setup.new_risq_indexer(10);

    setup.set_latest_epoch(Some(APIEpoch {
        index: 1,
        pubkey: Pubkey::default(),
        status: EpochStatus::Running,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_draws_sent(1);

    setup.restart_risq_indexer();

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_draws_sent(1);

    setup.restart_risq_indexer();

    setup.set_latest_epoch(Some(APIEpoch {
        index: 2,
        pubkey: Pubkey::default(),
        status: EpochStatus::Running,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_draws_sent(2);

    Ok(())
}

#[tokio::test]
async fn survives_restart_between_sending_batches_of_tickets() -> Result<()> {
    let epoch_index = 1;
    let batch_size = 5;
    let mut setup = IndexerSetup::new();
    let indexer = setup.new_risq_indexer(batch_size);

    setup.set_latest_epoch(Some(APIEpoch {
        index: epoch_index,
        pubkey: Pubkey::default(),
        status: EpochStatus::Yielding,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));
    setup.set_success_count(3);
    setup.insert_wallets(20);
    setup.nezha_api.generate_tickets_for_all().await?;

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_entries_sent(3);

    setup.restart_risq_indexer();

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_entries_sent(6);

    setup.restart_risq_indexer();

    setup.set_success_count(100);
    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_entries_sent(11);

    setup.restart_risq_indexer();

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_entries_sent(16);

    setup.restart_risq_indexer();

    indexer.run(&mut setup.indexer_risq_state).await?;
    setup.assert_entries_sent(20);

    Ok(())
}

#[tokio::test]
async fn full_loop_test() -> Result<()> {
    let epoch_index = 1;
    let batch_size = 5;
    let setup = IndexerSetup::new();
    let risq_indexer = setup.new_risq_indexer(batch_size);
    let deposits_indexer = setup.new_deposits_indexer(batch_size);
    let tickets_indexer = setup.new_tickets_indexer();
    let cancelled = Arc::new(AtomicBool::new(false));

    setup.set_latest_epoch(Some(APIEpoch {
        index: epoch_index,
        pubkey: Pubkey::default(),
        status: EpochStatus::Running,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));
    setup.insert_unapproved_wallets(20);
    setup.set_success_count(4);

    let sleep_between_batches = risq_indexer.config.sleep_between_batches;

    let cancelled_ = cancelled.clone();
    tokio::spawn(async move { risq_indexer.run_loop(cancelled_).await });

    let cancelled_ = cancelled.clone();
    tokio::spawn(async move {
        while !cancelled_.load(Ordering::Relaxed) {
            deposits_indexer.run(cancelled_.clone()).await.unwrap();
            // yield from the loop so that single threaded runtimes don't get stuck
            tokio::time::sleep(Duration::from_millis(0)).await;
        }
    });

    let cancelled_ = cancelled.clone();
    tokio::spawn(async move {
        while !cancelled_.load(Ordering::Relaxed) {
            tickets_indexer.run().await.unwrap();
            // yield from the loop so that single threaded runtimes don't get stuck
            tokio::time::sleep(Duration::from_millis(0)).await;
        }
    });

    tokio::time::sleep(sleep_between_batches).await;
    setup.assert_approved(5);
    setup.assert_tickets_generated(5);

    tokio::time::sleep(sleep_between_batches).await;
    setup.assert_approved(10);
    setup.assert_tickets_generated(10);

    tokio::time::sleep(sleep_between_batches).await;
    setup.assert_approved(15);
    setup.assert_tickets_generated(15);

    tokio::time::sleep(sleep_between_batches).await;
    setup.assert_approved(20);
    setup.assert_tickets_generated(20);

    setup.assert_draws_sent(1);
    setup.assert_entries_sent(0);

    setup.set_latest_epoch(Some(APIEpoch {
        index: epoch_index,
        pubkey: Pubkey::default(),
        status: EpochStatus::Yielding,
        prizes: TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        },
        total_value_locked: None,
        winning_combination: None,
        winners: None,
        expected_end_date: nezha_draw_date_utc(),
        draw_enabled: DrawEnabled::NoDraw,
    }));

    tokio::time::sleep(sleep_between_batches).await;
    setup.assert_draws_sent(1);
    setup.assert_entries_sent(4);

    tokio::time::sleep(sleep_between_batches).await;
    setup.assert_entries_sent(8);

    tokio::time::sleep(sleep_between_batches).await;
    setup.assert_entries_sent(12);

    tokio::time::sleep(sleep_between_batches).await;
    setup.assert_entries_sent(16);

    tokio::time::sleep(sleep_between_batches).await;
    setup.assert_entries_sent(20);

    // No spurious tickets are generated/sent
    tokio::time::sleep(sleep_between_batches).await;
    setup.assert_tickets_generated(20);
    setup.assert_entries_sent(20);

    cancelled.store(true, Ordering::Relaxed);

    Ok(())
}
