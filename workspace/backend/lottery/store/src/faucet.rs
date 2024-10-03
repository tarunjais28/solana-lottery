use crate::Pool;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use service::{faucet::FaucetRepository, model::transaction::TransactionId};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tokio_postgres::Row;

struct LatestMintTransaction {
    pub wallet: Pubkey,
    pub amount: u64,
    pub transaction_time: DateTime<Utc>,
    pub transaction_id: TransactionId,
}

impl TryFrom<Row> for LatestMintTransaction {
    type Error = anyhow::Error;

    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(LatestMintTransaction {
            wallet: Pubkey::from_str(&row.get::<_, String>("wallet"))?,
            amount: row.get::<_, String>("amount").parse::<u64>()?,
            transaction_time: row.get::<_, DateTime<Utc>>("transaction_time"),
            transaction_id: TransactionId(row.get::<_, String>("transaction_id")),
        })
    }
}

impl From<LatestMintTransaction> for service::model::faucet::LatestMintTransaction {
    fn from(latest_mint_transaction: LatestMintTransaction) -> Self {
        service::model::faucet::LatestMintTransaction {
            wallet: latest_mint_transaction.wallet,
            amount: latest_mint_transaction.amount,
            transaction_time: latest_mint_transaction.transaction_time,
            transaction_id: latest_mint_transaction.transaction_id,
        }
    }
}

#[derive(Clone)]
pub struct PostgresFaucetRepository {
    pool: Pool,
}

impl PostgresFaucetRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl FaucetRepository for PostgresFaucetRepository {
    async fn latest_transaction(
        &self,
        wallet: &Pubkey,
    ) -> Result<Option<service::model::faucet::LatestMintTransaction>> {
        let client = self.pool.get().await?;
        let transaction = match client
            .query_opt("SELECT * FROM faucet WHERE wallet = $1", &[&wallet.to_string()])
            .await?
        {
            Some(row) => LatestMintTransaction::try_from(row)?,
            None => return Ok(None),
        };

        Ok(Some(transaction.into()))
    }
    async fn store_latest_transaction(
        &self,
        wallet: &Pubkey,
        amount: u64,
        transaction_time: &DateTime<Utc>,
        transaction_id: &TransactionId,
    ) -> Result<service::model::faucet::LatestMintTransaction> {
        let client = self.pool.get().await?;
        let row = client
            .query_one(
                "INSERT INTO faucet (
                    wallet, 
                    amount,
                    transaction_time,
                    transaction_id
                )
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (wallet) DO
                    UPDATE SET amount = $2, transaction_time = $3, transaction_id = $4
                RETURNING *",
                &[
                    &wallet.to_string(),
                    &amount.to_string(),
                    transaction_time,
                    &transaction_id.0,
                ],
            )
            .await?;
        let latest_mint_transaction = LatestMintTransaction::try_from(row)?;
        Ok(latest_mint_transaction.into())
    }
}
