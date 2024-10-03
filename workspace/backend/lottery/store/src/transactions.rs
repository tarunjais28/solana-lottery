use std::{cmp, str::FromStr};

use crate::{get_client, Pool};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use postgres_types::ToSql;
use service::{
    epoch::FPUSDC,
    model::transaction::{TransactionId, TransactionType},
    transaction::{TransactionHistoryRepository, UserTransactionRepository},
};
use solana_sdk::pubkey::Pubkey;
use tokio_postgres::Row;

struct Transaction {
    transaction_id: String,
    instruction_index: i16,
    wallet: String,
    amount: String,
    mint: String,
    time: Option<DateTime<Utc>>,
    transaction_type: String,
}

impl From<service::model::transaction::Transaction> for Transaction {
    fn from(transaction: service::model::transaction::Transaction) -> Self {
        Self {
            transaction_id: transaction.transaction_id.0,
            instruction_index: transaction.instruction_index as i16,
            wallet: transaction.wallet.to_string(),
            amount: transaction.amount.to_string(),
            mint: transaction.mint.to_string(),
            time: transaction.time,
            transaction_type: transaction.transaction_type.to_string(),
        }
    }
}

impl TryFrom<Transaction> for service::model::transaction::Transaction {
    type Error = anyhow::Error;

    fn try_from(transaction: Transaction) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_id: TransactionId(transaction.transaction_id),
            instruction_index: transaction.instruction_index.try_into()?,
            wallet: Pubkey::from_str(&transaction.wallet)?,
            amount: transaction.amount.parse().map_err(|e: String| anyhow!(e))?,
            mint: Pubkey::from_str(&transaction.mint)?,
            time: transaction.time,
            transaction_type: transaction.transaction_type.parse()?,
        })
    }
}

impl From<Row> for Transaction {
    fn from(row: Row) -> Self {
        Self {
            transaction_id: row.get("transaction_id"),
            instruction_index: row.get("instruction_index"),
            wallet: row.get("wallet"),
            amount: row.get("amount"),
            mint: row.get("mint"),
            time: row.get("transaction_time"),
            transaction_type: row.get("transaction_type"),
        }
    }
}

#[derive(Clone)]
pub struct PostgresUserTransactionRepository {
    pool: Pool,
    max_limit: i64,
}

impl PostgresUserTransactionRepository {
    pub fn new(pool: Pool, max_limit: i64) -> Self {
        Self { pool, max_limit }
    }
}

#[async_trait]
impl UserTransactionRepository for PostgresUserTransactionRepository {
    async fn by_transaction_id_and_instruction_index(
        &self,
        transaction_id: &TransactionId,
        instruction_index: u8,
    ) -> Result<Option<service::model::transaction::Transaction>> {
        let client = get_client(&self.pool).await?;
        let row = client
            .query_opt(
                "SELECT * FROM user_transaction WHERE transaction_id = $1 AND instruction_index = $2",
                &[&transaction_id.0, &(instruction_index as i16)],
            )
            .await?;
        row.map(Transaction::from)
            .map(service::model::transaction::Transaction::try_from)
            .transpose()
    }

    async fn by_transaction_id(
        &self,
        transaction_id: &TransactionId,
    ) -> Result<Vec<service::model::transaction::Transaction>> {
        let client = get_client(&self.pool).await?;
        let rows = client
            .query(
                "SELECT * FROM user_transaction WHERE transaction_id = $1",
                &[&transaction_id.0],
            )
            .await?;
        rows.into_iter()
            .map(Transaction::from)
            .map(service::model::transaction::Transaction::try_from)
            .collect()
    }

    async fn by_wallet(
        &self,
        wallet: &Pubkey,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<service::model::transaction::Transaction>> {
        let limit = cmp::min(limit as i64, self.max_limit);
        let client = get_client(&self.pool).await?;
        let rows = client
            .query(
                "SELECT * FROM user_transaction WHERE wallet = $1 ORDER BY sl_no DESC LIMIT $2 OFFSET $3",
                &[&wallet.to_string(), &limit, &(offset as i64)],
            )
            .await?;
        rows.into_iter()
            .map(Transaction::from)
            .map(service::model::transaction::Transaction::try_from)
            .collect()
    }

    async fn by_type(
        &self,
        transaction_type: TransactionType,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<service::model::transaction::Transaction>> {
        let limit = cmp::min(limit as i64, self.max_limit);
        let client = get_client(&self.pool).await?;
        let rows = client
            .query(
                "SELECT * FROM user_transaction WHERE transaction_type = $1 ORDER BY sl_no DESC LIMIT $2 OFFSET $3",
                &[&transaction_type.to_string(), &limit, &(offset as i64)],
            )
            .await?;
        rows.into_iter()
            .map(Transaction::from)
            .map(service::model::transaction::Transaction::try_from)
            .collect()
    }

    async fn by_wallet_and_type(
        &self,
        wallet: &Pubkey,
        transaction_type: TransactionType,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<service::model::transaction::Transaction>> {
        let limit = cmp::min(limit as i64, self.max_limit);
        let client = get_client(&self.pool).await?;
        let rows = client
            .query(
                "SELECT * FROM user_transaction WHERE wallet = $1 AND transaction_type = $2 ORDER BY sl_no DESC LIMIT $3 OFFSET $4",
                &[&wallet.to_string(), &transaction_type.to_string(), &limit, &(offset as i64)],
            )
            .await?;
        rows.into_iter()
            .map(Transaction::from)
            .map(service::model::transaction::Transaction::try_from)
            .collect()
    }

    async fn all(&self, limit: usize, offset: usize) -> Result<Vec<service::model::transaction::Transaction>> {
        let limit = cmp::min(limit as i64, self.max_limit);
        let client = get_client(&self.pool).await?;
        let rows = client
            .query(
                "SELECT * FROM user_transaction ORDER BY sl_no DESC LIMIT $1 OFFSET $2",
                &[&limit, &(offset as i64)],
            )
            .await?;
        rows.into_iter()
            .map(Transaction::from)
            .map(service::model::transaction::Transaction::try_from)
            .collect()
    }

    async fn store_transaction(&self, transaction: &service::model::transaction::Transaction) -> Result<()> {
        let client = get_client(&self.pool).await?;
        let transaction = Transaction::from(transaction.clone());
        let query = "INSERT INTO user_transaction (transaction_id, instruction_index, wallet, amount, mint, transaction_time, transaction_type)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (transaction_id, instruction_index) DO NOTHING";
        let params: Vec<&(dyn ToSql + Sync)> = vec![
            &transaction.transaction_id,
            &transaction.instruction_index,
            &transaction.wallet,
            &transaction.amount,
            &transaction.mint,
            &transaction.time,
            &transaction.transaction_type,
        ];

        client.execute(query, &params).await?;
        Ok(())
    }

    async fn store_transactions(&self, transactions: &[service::model::transaction::Transaction]) -> Result<()> {
        if transactions.is_empty() {
            return Ok(());
        }
        let client = get_client(&self.pool).await?;
        let transactions = transactions.iter().cloned().map(Transaction::from).collect::<Vec<_>>();
        let mut query =
            "INSERT INTO user_transaction (transaction_id, instruction_index, wallet, amount, mint, transaction_time, transaction_type) VALUES "
                .to_string();
        let mut params: Vec<&(dyn ToSql + Sync)> = vec![];
        for (index, transaction) in transactions.iter().enumerate() {
            query += &format!(
                "(${}, ${}, ${}, ${}, ${}, ${}, ${})",
                index * 7 + 1,
                index * 7 + 2,
                index * 7 + 3,
                index * 7 + 4,
                index * 7 + 5,
                index * 7 + 6,
                index * 7 + 7
            );
            params.push(&transaction.transaction_id);
            params.push(&transaction.instruction_index);
            params.push(&transaction.wallet);
            params.push(&transaction.amount);
            params.push(&transaction.mint);
            params.push(&transaction.time);
            params.push(&transaction.transaction_type);

            if index < transactions.len() - 1 {
                query += ", ";
            }
        }
        query += " ON CONFLICT (transaction_id, instruction_index) DO NOTHING";

        client.execute(&query, &params).await?;
        Ok(())
    }

    async fn total_deposit_by_wallet(&self, wallet: &Pubkey) -> Result<FPUSDC> {
        let client = get_client(&self.pool).await?;
        let rows = client
            .query(
                "SELECT amount FROM user_transaction WHERE transaction_type=$1 AND wallet=$2",
                &[&TransactionType::DepositCompleted.to_string(), &wallet.to_string()],
            )
            .await?;
        let total = rows
            .into_iter()
            .map(|row| row.get::<_, String>(0))
            .map(|amount| amount.parse::<FPUSDC>().map_err(|e| anyhow!(e)))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .try_fold(FPUSDC::zero(), |acc, amount| {
                acc.checked_add(amount)
                    .ok_or_else(|| anyhow!("Overflow while calculating total deposit amount"))
            })?;
        Ok(total)
    }
}

pub struct PostgresTransactionHistoryRepository {
    pool: Pool,
}

impl PostgresTransactionHistoryRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransactionHistoryRepository for PostgresTransactionHistoryRepository {
    async fn last_saved(&self) -> Result<Option<TransactionId>> {
        let client = get_client(&self.pool).await?;
        let row = client
            .query_opt("SELECT * FROM transaction_history ORDER BY sl_no DESC LIMIT 1", &[])
            .await?;
        Ok(row.map(|row| TransactionId(row.get::<_, String>("transaction_id"))))
    }

    async fn save_transaction_id(&self, transaction_id: &TransactionId) -> Result<()> {
        let client = get_client(&self.pool).await?;
        client
            .execute(
                "INSERT INTO transaction_history (transaction_id) VALUES ($1) ON CONFLICT (transaction_id) DO NOTHING",
                &[&transaction_id.0],
            )
            .await?;
        Ok(())
    }

    async fn save_transaction_ids(&self, transaction_ids: &[TransactionId]) -> Result<()> {
        if transaction_ids.is_empty() {
            return Ok(());
        }
        let client = get_client(&self.pool).await?;
        let mut query = "INSERT INTO transaction_history (transaction_id) VALUES ".to_string();
        let mut params: Vec<&(dyn ToSql + Sync)> = vec![];
        for (index, transaction_id) in transaction_ids.iter().enumerate() {
            query += &format!("(${})", index + 1);
            params.push(&transaction_id.0);

            if index < transaction_ids.len() - 1 {
                query += ", ";
            }
        }
        query += " ON CONFLICT (transaction_id) DO NOTHING";
        client.execute(&query, &params).await?;
        Ok(())
    }
}
