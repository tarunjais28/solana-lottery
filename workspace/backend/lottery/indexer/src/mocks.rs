#![allow(unused_variables)]
use crate::db::risq_epochs::EpochStore;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use nezha_staking::fixed_point::FPUSDC;
use rand::{thread_rng, Rng};
use risq_api_client::resources::draw::objects::{Draw, DrawId, DrawInfo, DrawWinningKey};
use risq_api_client::resources::entry::objects::{
    Entry, EntryBatch, EntryBatchReceipt, EntryDetail, EntryReceipt, EntryReceiptStatus, RetrieveWinnersOutput,
};
use risq_api_client::resources::{DrawRoutes, EntryRoutes};
use risq_api_client::AppResult;
use solana_sdk::pubkey::Pubkey;
use std::sync::{Arc, Mutex};

use crate::db::risq_epochs::Epoch as DBEpoch;
use crate::nezha_api::{
    Epoch as APIEpoch, Investor, NezhaAPI, Sequence, StakeUpdateRequest, StakeUpdateRequestState, Ticket, TieredPrizes,
    WalletRisqId, YieldSplitCfg,
};

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum StakeUpdateState {
    PendingApproval,
    Queued,
    Completed,
}

#[derive(Clone)]
pub struct NezhaApiImpl {
    pub latest_epoch: Arc<Mutex<Option<APIEpoch>>>,
    /// (user_pubkey, deposit_state)
    pub wallets: Arc<Mutex<Vec<(Pubkey, StakeUpdateState)>>>,
    pub tickets: Arc<Mutex<Vec<TicketExt>>>,
}

#[derive(Clone)]
pub struct TicketExt {
    pub ticket: Ticket,
    pub epoch_index: u64,
    pub risq_id: Option<String>,
}

impl NezhaApiImpl {
    pub fn new() -> Self {
        Self {
            latest_epoch: Arc::new(Mutex::new(None)),
            wallets: Arc::new(Mutex::new(Vec::new())),
            tickets: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn insert_wallet(&self, wallet: Pubkey) {
        let mut wallets = self.wallets.lock().unwrap();
        wallets.push((wallet.clone(), StakeUpdateState::Completed));
    }

    pub fn insert_unapproved_wallet(&self, wallet: Pubkey) {
        let mut wallets = self.wallets.lock().unwrap();
        wallets.push((wallet.clone(), StakeUpdateState::PendingApproval));
    }
}

#[async_trait]
impl NezhaAPI for NezhaApiImpl {
    async fn publish_winning_combination(&self, combination: [u8; 6]) -> Result<()> {
        unimplemented!()
    }
    async fn get_latest_epoch(&self) -> Result<Option<APIEpoch>> {
        let latest_epoch = self.latest_epoch.lock().unwrap();
        Ok(latest_epoch.clone())
    }

    async fn generate_tickets_for_all(&self) -> Result<Vec<Pubkey>> {
        let wallets = self.wallets.lock().unwrap();
        let epoch_index = self.latest_epoch.lock().unwrap().as_ref().unwrap().index;
        let mut tickets = self.tickets.lock().unwrap();
        let mut tickets_generated_for = Vec::new();
        for (w, state) in &*wallets {
            if *state == StakeUpdateState::Completed && tickets.iter_mut().find(|t| t.ticket.wallet == *w).is_none() {
                tickets_generated_for.push(*w);
                tickets.push(TicketExt {
                    ticket: Ticket {
                        wallet: *w,
                        sequences: Vec::new(),
                    },
                    epoch_index,
                    risq_id: None,
                });
            }
        }

        Ok(tickets_generated_for)
    }

    async fn generate_ticket(&self, wallet: &Pubkey) -> Result<Vec<Sequence>> {
        todo!()
    }

    async fn get_unsubmitted_tickets(&self, epoch_index: u64) -> Result<Vec<Ticket>> {
        let tickets = self.tickets.lock().unwrap();
        let filtered = tickets
            .iter()
            .filter(|t| t.epoch_index == epoch_index && t.risq_id.is_none())
            .map(|t| &t.ticket)
            .cloned()
            .collect();
        Ok(filtered)
    }

    async fn update_risq_ids(&self, epoch_index: u64, risq_ids: &[WalletRisqId]) -> Result<()> {
        let mut tickets = self.tickets.lock().unwrap();
        for ticket in &mut *tickets {
            if ticket.epoch_index != epoch_index {
                continue;
            }
            for risq_id in risq_ids {
                if risq_id.wallet == ticket.ticket.wallet {
                    ticket.risq_id = Some(risq_id.risq_id.clone());
                }
            }
        }
        Ok(())
    }

    async fn all_stake_update_requests(&self) -> Result<Vec<StakeUpdateRequest>> {
        let wallets = self.wallets.lock().unwrap();
        Ok(wallets
            .iter()
            .filter_map(|(w, state)| {
                let state = match state {
                    StakeUpdateState::Completed => return None,
                    StakeUpdateState::PendingApproval => StakeUpdateRequestState::PendingApproval,
                    StakeUpdateState::Queued => StakeUpdateRequestState::Queued,
                };
                Some(StakeUpdateRequest { owner: *w, state })
            })
            .collect())
    }

    async fn approve_stake_update(&self, wallet: &Pubkey) -> Result<()> {
        let mut wallets = self.wallets.lock().unwrap();
        for (wallet_, state) in &mut *wallets {
            if wallet_ == wallet {
                assert!(*state == StakeUpdateState::PendingApproval);
                *state = StakeUpdateState::Queued;
            }
        }
        Ok(())
    }

    async fn complete_stake_update(&self, wallet: &Pubkey) -> Result<()> {
        let mut wallets = self.wallets.lock().unwrap();
        for (wallet_, state) in &mut *wallets {
            if wallet_ == wallet {
                assert!(*state == StakeUpdateState::Queued);
                *state = StakeUpdateState::Completed;
            }
        }
        Ok(())
    }

    async fn create_epoch(
        &self,
        prizes: TieredPrizes,
        expected_duration_minutes: u32,
        yield_split_cfg: YieldSplitCfg,
    ) -> Result<()> {
        todo!()
    }

    async fn enter_investment(&self, _investor: Investor) -> Result<()> {
        todo!()
    }

    async fn exit_investment(&self, _investor: Investor, _yield_percent: Option<FPUSDC>) -> Result<()> {
        todo!()
    }

    async fn publish_winners(&self) -> Result<()> {
        todo!()
    }

    async fn calculate_optimal_winning_combination(&self) -> Result<Option<[u8; 6]>> {
        todo!()
    }

    async fn random_winning_combination(&self) -> Result<Option<[u8; 6]>> {
        todo!()
    }
}

#[derive(Clone)]
pub struct DrawRoutesImpl {
    pub draws: Arc<Mutex<Vec<Draw>>>,
}

impl DrawRoutesImpl {
    pub fn new() -> Self {
        Self {
            draws: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl DrawRoutes for DrawRoutesImpl {
    /// Initialize a draw. If called multiple times, will overwrite the existing values.
    /// Will fail if the draw is closed.
    async fn send_draw_information(&self, product_id: &str, draw_id: &DrawId, draw_info: &DrawInfo) -> AppResult<Draw> {
        let mut draws = self.draws.lock().unwrap();
        let draw = Draw {
            product_id: product_id.to_string(),
            draw_id: draw_id.clone(),
            info: draw_info.clone(),
            stats: None,
        };
        draws.push(draw.clone());
        Ok(draw)
    }

    async fn get_draw_information(&self, product_id: &str, draw_id: &DrawId) -> AppResult<Draw> {
        todo!();
    }

    /// Will fail if the draw is not closed
    async fn get_draw_winning_key(&self, product_id: &str, draw_id: &DrawId) -> AppResult<DrawWinningKey> {
        todo!()
    }

    /// Will return empty if the draw is not closed
    async fn get_draws_winning_keys(
        &self,
        product_id: &str,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        size: Option<u64>,
        page: Option<u64>,
    ) -> AppResult<Vec<DrawWinningKey>> {
        todo!()
    }
}

#[derive(Clone)]
pub struct EntryRoutesImpl {
    pub entries: Arc<Mutex<Vec<SavedEntry>>>,
    pub success_count: Arc<Mutex<usize>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SavedEntry {
    pub risq_id: String,
    pub batch_id: String,
    pub entry: Entry,
}

impl EntryRoutesImpl {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
            success_count: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait]
impl EntryRoutes for EntryRoutesImpl {
    async fn send_entry(&self, entry: &Entry) -> AppResult<EntryReceipt> {
        todo!()
    }

    async fn retrieve_entry(&self, id: &str) -> AppResult<EntryDetail> {
        todo!()
    }

    async fn delete_entry(&self, id: &str) -> AppResult<()> {
        todo!()
    }

    async fn send_entry_batch(&self, entry_batch: &EntryBatch) -> AppResult<EntryBatchReceipt> {
        let mut entries = self.entries.lock().unwrap();
        let success_count = self.success_count.lock().unwrap();

        let mut receipts = Vec::new();
        let mut rng = thread_rng();
        for (i, entry) in entry_batch.entries.iter().enumerate() {
            let success = i < *success_count;
            if success {
                let entry_id = rng.gen::<u64>().to_string();
                let receipt = EntryReceipt {
                    entry_ref: entry.entry_ref.clone(),
                    status: EntryReceiptStatus::Ok {
                        entry_id: entry_id.clone(),
                        timestamp: 0,
                    },
                };
                receipts.push(receipt);
                entries.push(SavedEntry {
                    risq_id: entry_id,
                    batch_id: entry_batch.batch_id.clone(),
                    entry: entry.clone(),
                });
            } else {
                let receipt = EntryReceipt {
                    entry_ref: entry.entry_ref.clone(),
                    status: EntryReceiptStatus::Failed { error: String::new() },
                };
                receipts.push(receipt);
            }
        }
        let batch_receipt = EntryBatchReceipt {
            batch_id: entry_batch.batch_id.clone(),
            receipts,
        };
        Ok(batch_receipt)
    }

    async fn retrieve_winners(&self, product_id: &str, draw_id: &DrawId) -> AppResult<RetrieveWinnersOutput> {
        todo!()
    }
}

#[derive(Clone, Default)]
pub struct EpochStoreImpl {
    pub data: Arc<Mutex<Vec<DBEpoch>>>,
}

#[async_trait]
impl EpochStore for EpochStoreImpl {
    async fn save(&self, epoch: &DBEpoch) -> Result<DBEpoch> {
        let mut data = self.data.lock().unwrap();

        let mut found = false;
        for epoch_saved in &mut *data {
            if epoch_saved.index == epoch.index {
                *epoch_saved = epoch.clone();
                found = true;
                break;
            }
        }

        if !found {
            data.push(epoch.clone())
        }

        Ok(epoch.clone())
    }

    async fn load(&self, epoch_index: u64) -> Result<Option<DBEpoch>> {
        let data = self.data.lock().unwrap();
        Ok(data.iter().find(|e| e.index == epoch_index).cloned())
    }
}
