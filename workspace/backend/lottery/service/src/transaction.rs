use anyhow::Result;
use async_trait::async_trait;
use nezha_staking::fixed_point::FPUSDC;
use solana_sdk::pubkey::Pubkey;

use crate::model::transaction::{Transaction, TransactionId, TransactionType};

#[async_trait]
pub trait UserTransactionRepository: Send + Sync {
    async fn by_transaction_id_and_instruction_index(
        &self,
        transaction_id: &TransactionId,
        instruction_index: u8,
    ) -> Result<Option<Transaction>>;
    async fn by_transaction_id(&self, transaction_id: &TransactionId) -> Result<Vec<Transaction>>;
    async fn by_wallet(&self, wallet: &Pubkey, limit: usize, offset: usize) -> Result<Vec<Transaction>>;
    async fn by_type(&self, transaction_type: TransactionType, limit: usize, offset: usize)
        -> Result<Vec<Transaction>>;
    async fn by_wallet_and_type(
        &self,
        wallet: &Pubkey,
        transaction_type: TransactionType,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>>;
    async fn all(&self, limit: usize, offset: usize) -> Result<Vec<Transaction>>;
    async fn store_transaction(&self, transaction: &Transaction) -> Result<()>;
    async fn store_transactions(&self, transactions: &[Transaction]) -> Result<()>;
    async fn total_deposit_by_wallet(&self, wallet: &Pubkey) -> Result<FPUSDC>;
}

#[async_trait]
pub trait TransactionHistoryRepository: Send + Sync {
    async fn last_saved(&self) -> Result<Option<TransactionId>>;
    async fn save_transaction_id(&self, transaction_id: &TransactionId) -> Result<()>;
    async fn save_transaction_ids(&self, transaction_ids: &[TransactionId]) -> Result<()>;
}

#[async_trait]
pub trait UserTransactionService: Send + Sync {
    async fn by_transaction_id_and_instruction_index(
        &self,
        transaction_id: &TransactionId,
        instruction_index: u8,
    ) -> Result<Option<Transaction>>;
    async fn by_transaction_id(&self, transaction_id: &TransactionId) -> Result<Vec<Transaction>>;
    async fn by_wallet(&self, wallet: &Pubkey, limit: usize, offset: usize) -> Result<Vec<Transaction>>;
    async fn by_type(&self, transaction_type: TransactionType, limit: usize, offset: usize)
        -> Result<Vec<Transaction>>;
    async fn by_wallet_and_type(
        &self,
        wallet: &Pubkey,
        transaction_type: TransactionType,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>>;
    async fn all(&self, limit: usize, offset: usize) -> Result<Vec<Transaction>>;
    async fn total_deposit_by_wallet(&self, wallet: &Pubkey) -> Result<FPUSDC>;
}

pub struct UserTransactionServiceImpl {
    user_transaction_repository: Box<dyn UserTransactionRepository>,
}

impl UserTransactionServiceImpl {
    pub fn new(user_transaction_repository: Box<dyn UserTransactionRepository>) -> Self {
        Self {
            user_transaction_repository,
        }
    }
}

#[async_trait]
impl UserTransactionService for UserTransactionServiceImpl {
    async fn by_transaction_id_and_instruction_index(
        &self,
        transaction_id: &TransactionId,
        instruction_index: u8,
    ) -> Result<Option<Transaction>> {
        self.user_transaction_repository
            .by_transaction_id_and_instruction_index(transaction_id, instruction_index)
            .await
    }

    async fn by_transaction_id(&self, transaction_id: &TransactionId) -> Result<Vec<Transaction>> {
        self.user_transaction_repository.by_transaction_id(transaction_id).await
    }

    async fn by_wallet(&self, wallet: &Pubkey, limit: usize, offset: usize) -> Result<Vec<Transaction>> {
        self.user_transaction_repository.by_wallet(wallet, limit, offset).await
    }

    async fn by_type(
        &self,
        transaction_type: TransactionType,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>> {
        self.user_transaction_repository
            .by_type(transaction_type, limit, offset)
            .await
    }

    async fn by_wallet_and_type(
        &self,
        wallet: &Pubkey,
        transaction_type: TransactionType,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>> {
        self.user_transaction_repository
            .by_wallet_and_type(wallet, transaction_type, limit, offset)
            .await
    }

    async fn all(&self, limit: usize, offset: usize) -> Result<Vec<Transaction>> {
        self.user_transaction_repository.all(limit, offset).await
    }

    async fn total_deposit_by_wallet(&self, user_wallet: &Pubkey) -> Result<FPUSDC> {
        self.user_transaction_repository
            .total_deposit_by_wallet(user_wallet)
            .await
    }
}
