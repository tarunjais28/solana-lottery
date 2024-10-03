use std::{cmp, str::FromStr};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use deadpool_postgres::Pool;
use postgres_types::ToSql;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use service::{epoch::FPUSDC, prize::PrizeRepository};
use solana_sdk::pubkey::Pubkey;
use tokio_postgres::Row;

pub struct Prize {
    pub wallet: String,
    pub epoch_index: Decimal,
    pub page: i32,
    pub winner_index: i32,
    pub tier: i16,
    pub amount: String,
    pub claimable: bool,
    pub claimed: bool,
}

impl From<Row> for Prize {
    fn from(row: Row) -> Self {
        Self {
            wallet: row.get::<_, String>("wallet"),
            epoch_index: row.get::<_, Decimal>("epoch_index"),
            tier: row.get::<_, i16>("tier"),
            page: row.get::<_, i32>("page"),
            winner_index: row.get::<_, i32>("winner_index"),
            amount: row.get::<_, String>("amount"),
            claimable: row.get::<_, bool>("claimable"),
            claimed: row.get::<_, bool>("claimed"),
        }
    }
}

impl From<service::model::prize::Prize> for Prize {
    fn from(prize: service::model::prize::Prize) -> Self {
        Self {
            wallet: prize.wallet.to_string(),
            epoch_index: Decimal::from(prize.epoch_index),
            tier: prize.tier as i16,
            page: prize.page as i32,
            winner_index: prize.winner_index as i32,
            amount: prize.amount.to_string(),
            claimable: prize.claimable,
            claimed: prize.claimed,
        }
    }
}

impl TryFrom<Prize> for service::model::prize::Prize {
    type Error = anyhow::Error;

    fn try_from(prize: Prize) -> Result<Self> {
        Ok(Self {
            wallet: Pubkey::from_str(&prize.wallet)?,
            epoch_index: prize
                .epoch_index
                .to_u64()
                .ok_or_else(|| anyhow!("epoch_index is not u64"))?,
            tier: prize.tier.try_into()?,
            page: prize.page.try_into()?,
            winner_index: prize.winner_index.try_into()?,
            amount: prize.amount.parse().map_err(|e: String| anyhow!(e))?,
            claimable: prize.claimable,
            claimed: prize.claimed,
        })
    }
}

#[derive(Clone)]
pub struct PostgresPrizeRepository {
    pool: Pool,
    max_query_limit: i64,
}

impl PostgresPrizeRepository {
    pub fn new(pool: Pool, max_query_limit: i64) -> Self {
        Self { pool, max_query_limit }
    }
}

#[async_trait]
impl PrizeRepository for PostgresPrizeRepository {
    async fn by_wallet(
        &self,
        wallet: &Pubkey,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<service::model::prize::Prize>> {
        let limit = cmp::min(limit as i64, self.max_query_limit);
        let client = self.pool.get().await?;
        let rows = client
            .query(
                "
                SELECT 
                    wallet,
                    epoch_index,
                    page,
                    winner_index,
                    tier,
                    amount,
                    claimable,
                    claimed 
                FROM
                    prize
                WHERE 
                    wallet = $1 
                ORDER BY 
                    epoch_index DESC 
                LIMIT 
                    $2 
                OFFSET 
                    $3
                ",
                &[&wallet.to_string(), &limit, &(offset as i64)],
            )
            .await?;
        let prizes = rows
            .into_iter()
            .map(Prize::from)
            .map(service::model::prize::Prize::try_from)
            .collect::<Result<Vec<_>>>()?;
        Ok(prizes)
    }

    async fn by_wallet_epoch_and_tier(
        &self,
        wallet: &Pubkey,
        epoch_index: u64,
        tier: u8,
    ) -> Result<Option<service::model::prize::Prize>> {
        let client = self.pool.get().await?;
        let row = client
            .query_opt(
                "
                SELECT 
                    wallet,
                    epoch_index,
                    page,
                    winner_index,
                    tier,
                    amount,
                    claimable,
                    claimed 
                FROM
                    prize
                WHERE 
                    wallet = $1 AND 
                    epoch_index = $2 AND 
                    tier = $3
                ",
                &[&wallet.to_string(), &Decimal::from(epoch_index), &(tier as i16)],
            )
            .await?;
        row.map(Prize::from)
            .map(service::model::prize::Prize::try_from)
            .transpose()
    }

    async fn total_prize_by_wallet(&self, wallet: &Pubkey) -> Result<FPUSDC> {
        let client = self.pool.get().await?;
        let rows = client
            .query("SELECT amount FROM prize WHERE wallet = $1", &[&wallet.to_string()])
            .await?;
        let total_prize = rows
            .into_iter()
            .map(|row| row.get::<_, String>(0))
            .map(|amount| amount.parse().map_err(|e: String| anyhow!(e)))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .try_fold(FPUSDC::zero(), |acc, amount| {
                acc.checked_add(amount)
                    .ok_or_else(|| anyhow!("Error while calculating total prize amount"))
            })?;
        Ok(total_prize)
    }

    async fn upsert_prizes(&self, prizes: &[service::model::prize::Prize]) -> Result<()> {
        if prizes.is_empty() {
            return Ok(());
        }
        let client = self.pool.get().await?;
        let prizes = prizes.iter().cloned().map(Prize::from).collect::<Vec<_>>();
        let mut query = "
            INSERT INTO 
                prize (
                    wallet, 
                    epoch_index, 
                    page, 
                    winner_index, 
                    tier, 
                    amount, 
                    claimable, 
                    claimed
                )
            VALUES
        "
        .to_string();
        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();
        for (i, prize) in prizes.iter().enumerate() {
            query += &format!(
                "(${}, ${}, ${}, ${}, ${}, ${}, ${}, ${})",
                i * 8 + 1,
                i * 8 + 2,
                i * 8 + 3,
                i * 8 + 4,
                i * 8 + 5,
                i * 8 + 6,
                i * 8 + 7,
                i * 8 + 8
            );
            params.push(&prize.wallet);
            params.push(&prize.epoch_index);
            params.push(&prize.page);
            params.push(&prize.winner_index);
            params.push(&prize.tier);
            params.push(&prize.amount);
            params.push(&prize.claimable);
            params.push(&prize.claimed);
            if i != prizes.len() - 1 {
                query += ", ";
            }
        }
        query += "
            ON CONFLICT (
                wallet,
                epoch_index,
                page,
                winner_index
            ) DO UPDATE SET 
                tier = EXCLUDED.tier, 
                amount = EXCLUDED.amount, 
                claimable = EXCLUDED.claimable, 
                claimed = EXCLUDED.claimed
        ";
        client.execute(&query, &params).await?;
        Ok(())
    }
}
