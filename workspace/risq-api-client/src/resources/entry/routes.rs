use async_trait::async_trait;

use super::objects::*;
use crate::{
    client::{AppResult, Client},
    resources::draw::objects::DrawId,
};

#[async_trait]
pub trait EntryRoutes {
    async fn send_entry(&self, entry: &Entry) -> AppResult<EntryReceipt>;

    async fn retrieve_entry(&self, id: &str) -> AppResult<EntryDetail>;

    async fn delete_entry(&self, id: &str) -> AppResult<()>;

    async fn send_entry_batch(&self, entry_batch: &EntryBatch) -> AppResult<EntryBatchReceipt>;

    async fn retrieve_winners(&self, product_id: &str, draw_id: &DrawId) -> AppResult<RetrieveWinnersOutput>;
}

pub struct EntryRoutesImpl<T: Client> {
    client: T,
}

impl<T: Client> EntryRoutesImpl<T> {
    pub fn new(client: T) -> Self {
        Self { client }
    }
}

#[async_trait]
impl<T: Client + Send + Sync> EntryRoutes for EntryRoutesImpl<T> {
    async fn send_entry(&self, entry: &Entry) -> AppResult<EntryReceipt> {
        self.client.post("lty", "entry", entry).await
    }

    async fn retrieve_entry(&self, id: &str) -> AppResult<EntryDetail> {
        self.client.get("lty", &format!("entry/{id}")).await
    }

    async fn delete_entry(&self, id: &str) -> AppResult<()> {
        let (status, _) = self.client.delete_raw("lty", &format!("entry/{id}/delete")).await?;
        if status == 204 {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Expected HTTP 204. Received {status}"))?
        }
    }

    async fn send_entry_batch(&self, entry_batch: &EntryBatch) -> AppResult<EntryBatchReceipt> {
        self.client.post("lty", "entries", entry_batch).await
    }

    async fn retrieve_winners(&self, product_id: &str, draw_id: &DrawId) -> AppResult<RetrieveWinnersOutput> {
        self.client
            .get("lty", &format!("drawWinners/{product_id}/{draw_id}"))
            .await
    }
}
