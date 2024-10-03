use super::objects::*;
use crate::{
    client::{AppResult, Client},
    resources::draw::objects::DrawId,
};

use async_trait::async_trait;

#[async_trait]
pub trait BalanceRoutes {
    async fn retrieve_partner_balance_details(&self) -> AppResult<Vec<BalanceDetailsBean>>;

    async fn get_draw_prizes(&self, product_id: &str, draw_id: &DrawId) -> AppResult<DrawPrizes>;

    async fn get_draw_prizes_estimate(&self, product_id: &str, draw_id: &DrawId) -> AppResult<DrawPrizesEstimate>;
}

pub struct BalanceRoutesImpl<T: Client> {
    client: T,
}

impl<T: Client> BalanceRoutesImpl<T> {
    pub fn new(client: T) -> Self {
        Self { client }
    }
}

#[async_trait]
impl<T: Client + Send + Sync> BalanceRoutes for BalanceRoutesImpl<T> {
    async fn retrieve_partner_balance_details(&self) -> AppResult<Vec<BalanceDetailsBean>> {
        self.client.get("pol", "balance").await
    }

    async fn get_draw_prizes(&self, product_id: &str, draw_id: &DrawId) -> AppResult<DrawPrizes> {
        self.client
            .get("lty", &format!("draw/{product_id}/{draw_id}/prizes"))
            .await
    }

    async fn get_draw_prizes_estimate(&self, product_id: &str, draw_id: &DrawId) -> AppResult<DrawPrizesEstimate> {
        self.client
            .get("lty", &format!("drawEstimates/{product_id}/{draw_id}"))
            .await
    }
}
