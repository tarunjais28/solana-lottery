use async_trait::async_trait;
use chrono::{DateTime, Utc};
use risq_api_client::resources::draw::objects::*;
use risq_api_client::resources::entry::objects::*;
use risq_api_client::resources::EntryRoutes;
use risq_api_client::{resources::DrawRoutes, AppResult};

pub struct DrawRoutesImpl;

#[async_trait]
impl DrawRoutes for DrawRoutesImpl {
    async fn send_draw_information(&self, product_id: &str, draw_id: &DrawId, draw_info: &DrawInfo) -> AppResult<Draw> {
        Ok(Draw {
            draw_id: draw_id.clone(),
            product_id: product_id.to_string(),
            info: draw_info.clone(),
            stats: None,
        })
    }

    #[allow(unused_variables)]
    async fn get_draw_information(&self, product_id: &str, draw_id: &DrawId) -> AppResult<Draw> {
        todo!();
    }

    #[allow(unused_variables)]
    async fn get_draw_winning_key(&self, product_id: &str, draw_id: &DrawId) -> AppResult<DrawWinningKey> {
        todo!()
    }

    #[allow(unused_variables)]
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

pub struct EntryRoutesImpl;

#[async_trait]
impl EntryRoutes for EntryRoutesImpl {
    async fn send_entry(&self, entry: &Entry) -> AppResult<EntryReceipt> {
        Ok(EntryReceipt {
            entry_ref: entry.entry_ref.clone(),
            status: EntryReceiptStatus::Ok {
                entry_id: entry.entry_ref.clone(),
                timestamp: Utc::now().timestamp().max(0) as u64,
            },
        })
    }

    #[allow(unused_variables)]
    async fn retrieve_entry(&self, id: &str) -> AppResult<EntryDetail> {
        todo!()
    }

    #[allow(unused_variables)]
    async fn delete_entry(&self, id: &str) -> AppResult<()> {
        todo!()
    }

    async fn send_entry_batch(&self, entry_batch: &EntryBatch) -> AppResult<EntryBatchReceipt> {
        let mut entry_receipts = Vec::new();
        for entry in &entry_batch.entries {
            entry_receipts.push(EntryReceipt {
                entry_ref: entry.entry_ref.clone(),
                status: EntryReceiptStatus::Ok {
                    entry_id: entry.entry_ref.clone(),
                    timestamp: Utc::now().timestamp().max(0) as u64,
                },
            });
        }
        Ok(EntryBatchReceipt {
            batch_id: entry_batch.batch_id.clone(),
            receipts: entry_receipts,
        })
    }

    #[allow(unused_variables)]
    async fn retrieve_winners(&self, product_id: &str, draw_id: &DrawId) -> AppResult<RetrieveWinnersOutput> {
        todo!()
    }
}
