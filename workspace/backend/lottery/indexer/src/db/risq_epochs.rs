use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use rust_decimal::{prelude::ToPrimitive, Decimal};

use super::get_client;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Epoch {
    pub index: u64,
    pub draw_info_sent_to_risq_at: Option<DateTime<Utc>>,
    pub tickets_generated_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait EpochStore: Sync + Send {
    async fn save(&self, epoch: &Epoch) -> Result<Epoch>;
    async fn load(&self, epoch_index: u64) -> Result<Option<Epoch>>;
}

#[derive(Clone)]
pub struct PostgresEpochStore {
    pool: Pool,
}

impl PostgresEpochStore {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EpochStore for PostgresEpochStore {
    async fn save(&self, epoch: &Epoch) -> Result<Epoch> {
        let client = get_client(&self.pool).await?;

        let rows = client
            .query(
                "
                INSERT INTO epoch(
                    index,
                    draw_info_sent_to_risq_at,
                    tickets_generated_at
                ) VALUES (
                    $1,
                    $2,
                    $3
                ) ON CONFLICT (index) DO UPDATE SET 
                    draw_info_sent_to_risq_at = $2,
                    tickets_generated_at = $3
                RETURNING *
                ",
                &[
                    &Decimal::from(epoch.index),
                    &epoch.draw_info_sent_to_risq_at,
                    &epoch.tickets_generated_at,
                ],
            )
            .await?;

        let row = &rows[0];
        let epoch = Epoch {
            index: row
                .get::<_, Decimal>("index")
                .to_u64()
                .with_context(|| "Can't convert epoch index to u64")?,
            draw_info_sent_to_risq_at: row.get::<_, Option<DateTime<Utc>>>("draw_info_sent_to_risq_at"),
            tickets_generated_at: row.get::<_, Option<DateTime<Utc>>>("tickets_generated_at"),
        };

        Ok(epoch)
    }

    async fn load(&self, epoch_index: u64) -> Result<Option<Epoch>> {
        let client = get_client(&self.pool).await?;
        let rows = client
            .query("SELECT * FROM epoch WHERE index = $1", &[&Decimal::from(epoch_index)])
            .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        let epoch = Epoch {
            index: row
                .get::<_, Decimal>("index")
                .to_u64()
                .with_context(|| "Can't convert epoch index to u64")?,
            draw_info_sent_to_risq_at: row.get::<_, Option<DateTime<Utc>>>("draw_info_sent_to_risq_at"),
            tickets_generated_at: row.get::<_, Option<DateTime<Utc>>>("tickets_generated_at"),
        };

        Ok(Some(epoch))
    }
}
