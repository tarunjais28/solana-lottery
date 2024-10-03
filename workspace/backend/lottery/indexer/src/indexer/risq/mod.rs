use anyhow::{Context, Result};
use chrono::Utc;
use rand::{thread_rng, Rng};
use rust_decimal::Decimal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use risq_api_client::resources::configs::DrawConfig;
use risq_api_client::resources::draw::objects::DrawId;
use risq_api_client::resources::entry::objects::EntryReceiptStatus;
use risq_api_client::resources::{DrawRoutes, EntryRoutes};

use crate::conversions;
use crate::db::risq_epochs::{Epoch, EpochStore};
use crate::nezha_api::{EpochStatus, NezhaAPI, WalletRisqId};

/// This module contains a dummy implementation of RISQ routes
/// so that we can run it without connecting to RISQ on the testnet.
pub mod fake_risq;

pub struct IndexerRisq {
    pub epoch_store: Box<dyn EpochStore + Send + Sync>,
    pub nezha_api: Box<dyn NezhaAPI + Send + Sync>,
    pub risq_draw_routes: Box<dyn DrawRoutes + Send + Sync>,
    pub risq_entry_routes: Box<dyn EntryRoutes + Send + Sync>,
    pub config: IndexerRisqConfig,
}

pub struct IndexerRisqState {
    /// last completed epoch
    last_epoch: Option<(u64, EpochStatus)>,
}

impl IndexerRisqState {
    pub fn new() -> Self {
        Self { last_epoch: None }
    }
}

pub struct IndexerRisqConfig {
    pub risq_licensee_id: String,
    pub risq_product_id: String,
    pub nezha_prize_amount: Decimal,
    pub ticket_batch_size: usize,
    pub sleep_between_batches: Duration,
    pub draw_config: Box<dyn DrawConfig + Send + Sync>,
}

impl IndexerRisq {
    pub async fn run_loop(&self, cancelled: Arc<AtomicBool>) -> Result<()> {
        let mut state = IndexerRisqState::new();
        while !cancelled.load(Ordering::Relaxed) {
            self.run(&mut state).await?;
            tokio::time::sleep(self.config.sleep_between_batches).await;
        }
        Ok(())
    }

    pub async fn run(&self, state: &mut IndexerRisqState) -> Result<()> {
        let latest_epoch = self.nezha_api.get_latest_epoch().await?;

        if latest_epoch.is_none() {
            // Initially when prod is setup for the first time, there could be a small period where there
            // are no epochs
            return Ok(());
        }

        let latest_epoch = latest_epoch.unwrap();

        let epoch_index = latest_epoch.index;
        let epoch_status = latest_epoch.status;

        if let Some((last_epoch_index, last_epoch_status)) = state.last_epoch {
            // Epoch haven't changed since we processed it
            if epoch_index == last_epoch_index && epoch_status == last_epoch_status {
                return Ok(());
            }
        }

        let draw_date = latest_epoch.expected_end_date.naive_utc().date();
        let draw_id = self
            .config
            .draw_config
            .draw_id_try_from_draw_date(draw_date)
            .with_context(|| "Unable to calculate draw date from epoch.expected_end_date")?;

        let epoch = self.epoch_store.load(epoch_index).await?;

        // We use `if`s with `>=` and not `match` or `if..else if` so that
        // even if the indexer got started in the middle of Yielding for some reason,
        // it would still run the initialization steps.

        if epoch_status >= EpochStatus::Running {
            // Send the draw info to RISQ
            if epoch.is_none() || epoch.as_ref().unwrap().draw_info_sent_to_risq_at.is_none() {
                self.send_epoch_to_risq(&draw_id).await?;
                self.epoch_store
                    .save(&Epoch {
                        draw_info_sent_to_risq_at: Some(Utc::now()),
                        ..epoch.unwrap_or(Epoch {
                            index: epoch_index,
                            draw_info_sent_to_risq_at: Some(Utc::now()),
                            tickets_generated_at: None,
                        })
                    })
                    .await?;
            }
            state.last_epoch = Some((epoch_index, EpochStatus::Running));
        }

        if epoch_status >= EpochStatus::Yielding && epoch_status <= EpochStatus::Finalising {
            let finished = self.send_unsubmitted_tickets_to_risq(epoch_index, &draw_id).await?;
            if finished {
                state.last_epoch = Some((epoch_index, EpochStatus::Yielding));
            }
        }

        Ok(())
    }

    async fn send_epoch_to_risq(&self, draw_id: &DrawId) -> Result<()> {
        // Send epoch to RISQ
        let draw_info = self.config.draw_config.draw_info(self.config.nezha_prize_amount);
        self.risq_draw_routes
            .send_draw_information(&self.config.risq_product_id, draw_id, &draw_info)
            .await?;
        Ok(())
    }

    /// returns: are we _sure_ that there are no unsubmitted tickets left
    async fn send_unsubmitted_tickets_to_risq(&self, epoch_index: u64, draw_id: &DrawId) -> Result<bool> {
        let unsubmitted_tickets = self.nezha_api.get_unsubmitted_tickets(epoch_index).await?;
        if unsubmitted_tickets.is_empty() {
            // no more unsubmitted tickets
            return Ok(true);
        }

        if let Some(batch) = unsubmitted_tickets.chunks(self.config.ticket_batch_size).next() {
            let timestamp = Utc::now().timestamp() as u64;
            let batch_id = {
                let mut rng = thread_rng();
                rng.gen::<u64>()
            };
            let entry_batch = conversions::make_nezha_entry_batch_from_tickets(
                self.config.draw_config.as_ref(),
                batch_id.to_string(),
                batch,
                epoch_index,
                draw_id,
                &self.config.risq_licensee_id,
                timestamp,
            );
            let resp = self.risq_entry_routes.send_entry_batch(&entry_batch).await?;
            let mut wallet_risq_ids = Vec::new();
            for receipt in resp.receipts {
                match receipt.status {
                    EntryReceiptStatus::Ok { entry_id, .. } => {
                        let (wallet, _epoch_index) = conversions::entry_ref_to_wallet_epoch_index(&receipt.entry_ref)?;
                        let wallet_risq_id = WalletRisqId {
                            wallet,
                            risq_id: entry_id,
                        };
                        wallet_risq_ids.push(wallet_risq_id);
                    }
                    _ => {}
                }
            }
            self.nezha_api.update_risq_ids(epoch_index, &wallet_risq_ids).await?;
        }
        // we don't know whether all of the tickets were successfully accepted.
        // instead of checking each entry receipt, we simply query the db in the next iteration for
        // unsubmitted tickets.
        Ok(false)
    }
}
