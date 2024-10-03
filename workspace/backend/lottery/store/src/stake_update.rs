use crate::get_client;
use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::Pool;
use service::{
    model::{
        stake_update::{StakeUpdate, StakeUpdateState, StakeUpdateType},
        transaction::TransactionId,
    },
    solana::FPUSDC,
    stake::StakeUpdateRepository,
};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tokio_postgres::Row;
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresStakeUpdateRepository {
    pool: Pool,
}

impl PostgresStakeUpdateRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StakeUpdateRepository for PostgresStakeUpdateRepository {
    async fn by_transaction_id(&self, transaction_id: &TransactionId) -> Result<Option<StakeUpdate>> {
        let client = get_client(&self.pool).await?;
        let row = client
            .query_opt(
                "SELECT * from stake_update where transaction_id = $1",
                &[&transaction_id.0],
            )
            .await?;
        let stake_update = row.map(parse_row).transpose()?;
        Ok(stake_update)
    }

    async fn by_wallet(&self, wallet: &Pubkey) -> Result<Vec<StakeUpdate>> {
        let client = get_client(&self.pool).await?;
        let rows = client
            .query("SELECT * FROM stake_update WHERE wallet = $1", &[&wallet.to_string()])
            .await?;
        let stake_updates: Vec<_> = rows.into_iter().map(parse_row).collect::<Result<Vec<_>>>()?;
        Ok(stake_updates)
    }

    async fn store(&self, stake_update: &StakeUpdate) -> Result<StakeUpdate> {
        let client = get_client(&self.pool).await?;
        let id = Uuid::new_v4();
        let wallet = stake_update.owner.to_string();
        let amount = stake_update.amount.to_string();
        let state = stake_update.state.to_string();
        let type_ = stake_update.type_.to_string();
        let currency = stake_update.currency.to_string();
        let mint = stake_update.mint.to_string();
        let transaction_id = stake_update.transaction_id.to_owned().map(|tx_id| tx_id.0);
        let row = client
            .query_one(
                r#"
            INSERT INTO stake_update(
                id,
                wallet,
                amount,
                state,
                type,
                currency,
                mint,
                transaction_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *
            "#,
                &[&id, &wallet, &amount, &state, &type_, &currency, &mint, &transaction_id],
            )
            .await?;
        let stake_update = parse_row(row)?;
        Ok(stake_update)
    }
}

fn parse_row(row: Row) -> Result<StakeUpdate> {
    Ok(StakeUpdate {
        owner: Pubkey::from_str(row.get::<_, &str>("wallet"))?,
        amount: FPUSDC::from_str(row.get::<_, &str>("amount")).map_err(|s| anyhow::anyhow!(s))?,
        state: StakeUpdateState::from_str(row.get::<_, &str>("state"))?,
        type_: StakeUpdateType::from_str(row.get::<_, &str>("type"))?,
        currency: row.get::<_, String>("currency"),
        mint: Pubkey::from_str(row.get::<_, &str>("mint"))?,
        transaction_id: row
            .get::<_, Option<String>>("transaction_id")
            .map(|tx_id| TransactionId(tx_id)),
    })
}
