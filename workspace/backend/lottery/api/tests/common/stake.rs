use anyhow::Result;
use async_trait::async_trait;
use service::model::{deposit::Deposit, transaction::TransactionId};
use service::stake::{DepositRepository, Stake, StakeRepository, StakeService};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::RwLock;

#[derive(Debug, Default, Clone)]
pub struct InMemoryStakeRepository {
    mem: Vec<Stake>,
}

impl InMemoryStakeRepository {
    pub fn add(&mut self, entry: Stake) {
        self.mem.push(entry)
    }
}

#[async_trait]
impl StakeRepository for InMemoryStakeRepository {
    async fn by_wallet(&self, wallet: &Pubkey) -> Result<Vec<Stake>> {
        Ok(self
            .mem
            .iter()
            .filter(|&v| &v.owner.to_string() == &wallet.to_string())
            .map(|v| v.clone())
            .collect())
    }

    async fn pending_deposit(&self, _wallet: &Pubkey) -> Result<Option<Deposit>> {
        todo!()
    }

    async fn deposits(&self, _wallet: &Pubkey) -> Result<Vec<Deposit>> {
        todo!()
    }

    async fn all(&self) -> Result<Vec<Stake>> {
        Ok(self.mem.clone())
    }
}

#[derive(Debug, Default)]
pub struct InMemoryDepositRepository {
    mem: RwLock<Vec<Deposit>>,
}

#[async_trait]
impl DepositRepository for InMemoryDepositRepository {
    async fn by_transaction_id(&self, transaction_id: &TransactionId) -> Result<Option<Deposit>> {
        let deposits = self.mem.read().unwrap();
        let transaction_id = Some(transaction_id.to_owned());
        Ok(deposits.iter().find(|d| d.transaction_id == transaction_id).cloned())
    }

    async fn by_wallet(&self, wallet: &Pubkey) -> Result<Vec<Deposit>> {
        let deposits = self.mem.read().unwrap();
        Ok(deposits.iter().filter(|d| &d.owner == wallet).cloned().collect())
    }

    async fn store_deposit(&self, deposit: &Deposit) -> Result<Deposit> {
        self.mem.write().unwrap().push(deposit.clone());
        Ok(deposit.clone())
    }
}

pub struct InMemoryStakeService {
    stake_repository: Box<dyn StakeRepository>,
    deposit_repository: Box<dyn DepositRepository>,
}

impl InMemoryStakeService {
    pub fn new(stake_repository: Box<dyn StakeRepository>, deposit_repository: Box<dyn DepositRepository>) -> Self {
        Self {
            stake_repository,
            deposit_repository,
        }
    }
}

#[async_trait]
impl StakeService for InMemoryStakeService {
    async fn by_wallet(&self, user_wallet: &str) -> Result<Vec<Stake>> {
        self.stake_repository.by_wallet(&Pubkey::from_str(user_wallet)?).await
    }

    async fn deposit_by_transaction_id(&self, transaction_id: &TransactionId) -> Result<Option<Deposit>> {
        self.deposit_repository.by_transaction_id(transaction_id).await
    }

    async fn deposits(&self, user_wallet: &str) -> Result<Vec<Deposit>> {
        let mut deposits = self.stake_repository.deposits(&Pubkey::from_str(user_wallet)?).await?;
        deposits.extend(
            self.deposit_repository
                .by_wallet(&Pubkey::from_str(user_wallet)?)
                .await?,
        );
        Ok(deposits)
    }

    async fn approve_deposit(&self, _user_wallet: &str) -> Result<Vec<Deposit>> {
        todo!()
    }

    async fn all(&self) -> Result<Vec<Stake>> {
        todo!()
    }

    async fn all_pending_deposits(&self) -> Result<Vec<DepositAttempt>> {
        todo!()
    }
}
