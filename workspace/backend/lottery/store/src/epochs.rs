use crate::get_client;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_postgres::Pool;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde_json::{json, Value};
use service::{
    epoch::EpochRepository,
    model::epoch::{Epoch, EpochStatus, Returns},
    solana::{InsuranceCfg, YieldSplitCfg},
};
use solana_sdk::pubkey::Pubkey;
use std::{num::TryFromIntError, str::FromStr};
use tokio_postgres::Row;

pub struct PostgresEpoch(Epoch);

impl TryFrom<Row> for PostgresEpoch {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        let winning_combination = match row.get::<_, Option<[i16; 6]>>("winning_combination") {
            Some(winning_combination) => Some(
                <[u8; 6]>::try_from(
                    winning_combination
                        .into_iter()
                        .map(u8::try_from)
                        .collect::<std::result::Result<Vec<u8>, TryFromIntError>>()?,
                )
                .map_err(|_| anyhow!("cannot convert winning combination"))?,
            ),
            None => None,
        };
        Ok(Self(Epoch {
            pubkey: Pubkey::from_str(row.get::<_, &str>("pubkey"))?,
            index: row
                .get::<_, Decimal>("epoch_index")
                .to_u64()
                .ok_or(anyhow!("cannot convert index to u64"))?,
            status: epoch_status_from_str(row.get::<_, &str>("epoch_status"))?,
            winning_combination,
            yield_split_cfg: yield_split_cfg_from_json(&row.get("yield_split_cfg")),
            total_invested: row
                .get::<_, Option<&str>>("total_invested")
                .map(str::parse)
                .transpose()
                .map_err(|e: String| anyhow::anyhow!(e))?,
            returns: row.get::<_, Option<Value>>("returns").as_ref().map(returns_from_json),
            started_at: row.get::<_, DateTime<Utc>>("started_at"),
            expected_end_at: row.get::<_, DateTime<Utc>>("expected_end_at"),
            ended_at: row.get::<_, Option<DateTime<Utc>>>("ended_at"),
            draw_enabled: row.get::<_, Option<bool>>("draw_enabled"),
        }))
    }
}

#[derive(Clone)]
pub struct PostgresEpochRepository {
    pool: Pool,
}

impl PostgresEpochRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl EpochRepository for PostgresEpochRepository {
    async fn by_index(&self, epoch_index: u64) -> Result<Option<Epoch>> {
        let client = get_client(&self.pool).await?;
        let row = client
            .query_opt(
                "SELECT * from epoch where epoch_index = $1",
                &[&(Decimal::from(epoch_index))],
            )
            .await?;
        let epoch = row.map(PostgresEpoch::try_from).transpose()?.map(|ep| ep.0);
        Ok(epoch)
    }

    async fn by_pubkey(&self, pubkey: &Pubkey) -> Result<Option<Epoch>> {
        let client = get_client(&self.pool).await?;
        let row = client
            .query_opt("SELECT * from epoch where pubkey = $1", &[&pubkey.to_string()])
            .await?;
        let epoch = row.map(PostgresEpoch::try_from).transpose()?.map(|ep| ep.0);
        Ok(epoch)
    }

    async fn all(&self) -> Result<Vec<Epoch>> {
        let client = get_client(&self.pool).await?;
        let rows = client.query("SELECT * from epoch", &[]).await?;
        let epochs = rows
            .into_iter()
            .map(|row| PostgresEpoch::try_from(row).map(|ep| ep.0))
            .collect::<Result<Vec<_>>>()?;
        Ok(epochs)
    }

    async fn latest_epoch(&self) -> Result<Option<Epoch>> {
        let client = get_client(&self.pool).await?;
        let row = client
            .query_opt("SELECT * from epoch ORDER BY epoch_index DESC LIMIT 1", &[])
            .await?;
        let epoch = row.map(PostgresEpoch::try_from).transpose()?.map(|ep| ep.0);
        Ok(epoch)
    }

    async fn create_or_update_epoch(&self, epoch: &Epoch) -> Result<Epoch> {
        let client = get_client(&self.pool).await?;
        let row = client
            .query_one(
                "INSERT INTO epoch (
                    pubkey,
                    epoch_index,
                    epoch_status,
                    winning_combination,
                    yield_split_cfg,
                    total_invested,
                    returns,
                    started_at,
                    expected_end_at,
                    ended_at,
                    draw_enabled
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                ON CONFLICT (pubkey) DO UPDATE SET
                    epoch_status = $3,
                    winning_combination = $4,
                    total_invested = $6,
                    returns = $7,
                    ended_at = $10,
                    draw_enabled = $11
                RETURNING *",
                &[
                    &epoch.pubkey.to_string(),
                    &(Decimal::from(epoch.index)),
                    &epoch_status_to_str(epoch.status),
                    &epoch.winning_combination.map(|winning_combination| {
                        winning_combination.into_iter().map(|n| n as i16).collect::<Vec<_>>()
                    }),
                    &yield_split_cfg_to_json(&epoch.yield_split_cfg),
                    &epoch.total_invested.map(|total_invested| total_invested.to_string()),
                    &epoch.returns.as_ref().map(returns_to_json),
                    &epoch.started_at,
                    &epoch.expected_end_at,
                    &epoch.ended_at,
                    &epoch.draw_enabled,
                ],
            )
            .await?;

        let epoch = PostgresEpoch::try_from(row)?;
        Ok(epoch.0)
    }
}

fn yield_split_cfg_to_json(yield_split_cfg: &YieldSplitCfg) -> Value {
    json!({
    "jackpot": yield_split_cfg.jackpot.to_string(),
    "insurance": json!({
            "probability": yield_split_cfg.insurance.probability.to_string(),
            "premium": yield_split_cfg.insurance.premium.to_string(),
    }),
    "treasury_ratio": yield_split_cfg.treasury_ratio.to_string(),
    "tier2_prize_share": yield_split_cfg.tier2_prize_share,
    "tier3_prize_share": yield_split_cfg.tier3_prize_share,
    })
}

fn returns_to_json(returns: &Returns) -> Value {
    json!({
    "total": returns.total.to_string(),
    "deposit_back": returns.deposit_back.to_string(),
    "insurance": returns.insurance.to_string(),
    "treasury": returns.treasury.to_string(),
    "tier2_prize": returns.tier2_prize.to_string(),
    "tier3_prize": returns.tier3_prize.to_string(),
    })
}

fn yield_split_cfg_from_json(json: &Value) -> YieldSplitCfg {
    let obj = json.as_object().unwrap();
    YieldSplitCfg {
        jackpot: obj.get("jackpot").unwrap().as_str().unwrap().parse().unwrap(),
        insurance: {
            let insurance = obj.get("insurance").unwrap().as_object().unwrap();
            InsuranceCfg {
                premium: insurance.get("premium").unwrap().as_str().unwrap().parse().unwrap(),
                probability: insurance.get("probability").unwrap().as_str().unwrap().parse().unwrap(),
            }
        },
        treasury_ratio: obj.get("treasury_ratio").unwrap().as_str().unwrap().parse().unwrap(),
        tier2_prize_share: obj.get("tier2_prize_share").unwrap().as_u64().unwrap() as _,
        tier3_prize_share: obj.get("tier3_prize_share").unwrap().as_u64().unwrap() as _,
    }
}

fn returns_from_json(json: &Value) -> Returns {
    let obj = json.as_object().unwrap();
    Returns {
        total: obj.get("total").unwrap().as_str().unwrap().parse().unwrap(),
        deposit_back: obj.get("deposit_back").unwrap().as_str().unwrap().parse().unwrap(),
        insurance: obj.get("insurance").unwrap().as_str().unwrap().parse().unwrap(),
        treasury: obj.get("treasury").unwrap().as_str().unwrap().parse().unwrap(),
        tier2_prize: obj.get("tier2_prize").unwrap().as_str().unwrap().parse().unwrap(),
        tier3_prize: obj.get("tier3_prize").unwrap().as_str().unwrap().parse().unwrap(),
    }
}

fn epoch_status_from_str(s: &str) -> Result<EpochStatus, anyhow::Error> {
    match s {
        "running" => Ok(EpochStatus::Running),
        "yielding" => Ok(EpochStatus::Yielding),
        "finalising" => Ok(EpochStatus::Finalising),
        "ended" => Ok(EpochStatus::Ended),
        s => Err(anyhow!("invalid epoch status: {}", s)),
    }
}

fn epoch_status_to_str(s: EpochStatus) -> &'static str {
    match s {
        EpochStatus::Running => "running",
        EpochStatus::Yielding => "yielding",
        EpochStatus::Finalising => "finalising",
        EpochStatus::Ended => "ended",
    }
}
