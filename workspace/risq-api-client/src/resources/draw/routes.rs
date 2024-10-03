use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::objects::*;
use crate::client::{AppResult, Client};
use std::collections::HashMap;

#[async_trait]
pub trait DrawRoutes {
    /// Initialize a draw. If called multiple times, will overwrite the existing values.
    /// Will fail if the draw is closed.
    async fn send_draw_information(&self, product_id: &str, draw_id: &DrawId, draw_info: &DrawInfo) -> AppResult<Draw>;

    async fn get_draw_information(&self, product_id: &str, draw_id: &DrawId) -> AppResult<Draw>;

    /// Will fail if the draw is not closed
    async fn get_draw_winning_key(&self, product_id: &str, draw_id: &DrawId) -> AppResult<DrawWinningKey>;

    /// Will return empty if the draw is not closed
    async fn get_draws_winning_keys(
        &self,
        product_id: &str,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        size: Option<u64>,
        page: Option<u64>,
    ) -> AppResult<Vec<DrawWinningKey>>;
}

pub struct DrawRoutesImpl<T: Client> {
    client: T,
}

impl<T: Client> DrawRoutesImpl<T> {
    pub fn new(client: T) -> Self {
        Self { client }
    }
}

#[async_trait]
impl<T: Client + Send + Sync> DrawRoutes for DrawRoutesImpl<T> {
    /// Initialize a draw. If called multiple times, will overwrite the existing values.
    /// Will fail if the draw is closed.
    async fn send_draw_information(&self, product_id: &str, draw_id: &DrawId, draw_info: &DrawInfo) -> AppResult<Draw> {
        self.client
            .post("lty", &format!("draw/{product_id}/{draw_id}"), &draw_info)
            .await
    }

    async fn get_draw_information(&self, product_id: &str, draw_id: &DrawId) -> AppResult<Draw> {
        self.client.get("lty", &format!("draw/{product_id}/{draw_id}")).await
    }

    /// Will fail if the draw is not closed
    async fn get_draw_winning_key(&self, product_id: &str, draw_id: &DrawId) -> AppResult<DrawWinningKey> {
        let res = "lty";
        let url = format!("draw/{product_id}/{draw_id}/winningkey");
        self.client.get(res, &url).await
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
        let res = "lty";
        let url = format!("draw/{product_id}/winningkey");
        let mut query = HashMap::new();
        if let Some(start_date) = start_date {
            let start_date_string = start_date.to_rfc3339();
            query.insert("startDate", start_date_string);
        }
        if let Some(end_date) = end_date {
            let end_date_string = end_date.to_rfc3339();
            query.insert("endDate", end_date_string);
        }
        if let Some(page) = page {
            let page_string = page.to_string();
            query.insert("page", page_string);
        }
        if let Some(size) = size {
            let size_string = size.to_string();
            query.insert("size", size_string);
        }
        self.client.get_with_query(res, &url, &query).await
    }
}
