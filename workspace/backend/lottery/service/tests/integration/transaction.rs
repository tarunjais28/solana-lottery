use std::sync::RwLock;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use service::{
    epoch::FPUSDC,
    model::transaction::{Transaction, TransactionId, TransactionType},
    transaction::UserTransactionRepository,
};
use solana_sdk::pubkey::Pubkey;

#[derive(Default)]
pub struct InMemoryTransactionRepository {
    mem: RwLock<Vec<Transaction>>,
}

#[async_trait]
impl UserTransactionRepository for InMemoryTransactionRepository {
    async fn by_transaction_id_and_instruction_index(
        &self,
        transaction_id: &TransactionId,
        instruction_index: u8,
    ) -> Result<Option<Transaction>> {
        let transactions = self.mem.read().unwrap();
        Ok(transactions
            .iter()
            .find(|t| t.transaction_id == *transaction_id && t.instruction_index == instruction_index)
            .cloned())
    }

    async fn by_transaction_id(&self, transaction_id: &TransactionId) -> Result<Vec<Transaction>> {
        let transactions = self.mem.read().unwrap();
        Ok(transactions
            .iter()
            .filter(|t| t.transaction_id == *transaction_id)
            .cloned()
            .collect())
    }

    async fn by_wallet(&self, wallet: &Pubkey, limit: usize, offset: usize) -> Result<Vec<Transaction>> {
        let transactions = self.mem.read().unwrap();
        Ok(transactions
            .iter()
            .filter(|t| t.wallet == wallet.clone())
            .cloned()
            .skip(offset * limit)
            .take(limit)
            .collect())
    }

    async fn by_type(
        &self,
        transaction_type: TransactionType,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>> {
        let transactions = self.mem.read().unwrap();
        Ok(transactions
            .iter()
            .filter(|t| t.transaction_type == transaction_type)
            .cloned()
            .skip(offset * limit)
            .take(limit)
            .collect())
    }

    async fn by_wallet_and_type(
        &self,
        wallet: &Pubkey,
        transaction_type: TransactionType,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Transaction>> {
        let transactions = self.mem.read().unwrap();
        Ok(transactions
            .iter()
            .filter(|t| t.wallet == wallet.clone() && t.transaction_type == transaction_type)
            .cloned()
            .skip(offset * limit)
            .take(limit)
            .collect())
    }

    async fn all(&self, limit: usize, offset: usize) -> Result<Vec<Transaction>> {
        let transactions = self.mem.read().unwrap();
        Ok(transactions.iter().cloned().skip(offset * limit).take(limit).collect())
    }

    async fn store_transaction(&self, transaction: &Transaction) -> Result<()> {
        self.mem.write().unwrap().push(transaction.clone());
        Ok(())
    }

    async fn store_transactions(&self, transactions: &[Transaction]) -> Result<()> {
        self.mem.write().unwrap().extend_from_slice(transactions);
        Ok(())
    }

    async fn total_deposit_by_wallet(&self, wallet: &Pubkey) -> Result<FPUSDC> {
        let transactions = self.mem.read().unwrap();
        let total = transactions
            .iter()
            .filter(|t| t.transaction_type == TransactionType::DepositApproved && &t.wallet == wallet)
            .map(|t| t.amount)
            .try_fold(FPUSDC::zero(), |acc, x| {
                acc.checked_add(x)
                    .ok_or_else(|| anyhow!("Overflow while calculating total deposit amount"))
            })?;
        Ok(total)
    }
}
