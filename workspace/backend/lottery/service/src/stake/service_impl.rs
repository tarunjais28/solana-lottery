use std::str::FromStr;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use nezha_staking::{
    fixed_point::FPUSDC,
    state::{StakeUpdateRequest, StakeUpdateState as SolanaStakeUpdateState},
};
use solana_program::pubkey::Pubkey;

use crate::{
    model::{
        stake_update::{StakeUpdate, StakeUpdateState, StakeUpdateType},
        transaction::TransactionId,
    },
    solana::{AccountNotFound, Solana, SolanaError, Stake},
};

use super::{StakeService, StakeUpdateRepository};

pub struct DefaultStakeService {
    solana: Box<dyn Solana>,
    stake_update_repo: Box<dyn StakeUpdateRepository>,
}

impl DefaultStakeService {
    pub fn new(solana: Box<dyn Solana>, stake_update_repo: Box<dyn StakeUpdateRepository>) -> Self {
        Self {
            solana,
            stake_update_repo,
        }
    }
}

fn stake_update_request_to_stake_update(attempt: StakeUpdateRequest, usdc_mint: Pubkey) -> StakeUpdate {
    StakeUpdate {
        owner: attempt.owner,
        amount: FPUSDC::from_usdc(attempt.amount.abs() as u64),
        state: match attempt.state {
            SolanaStakeUpdateState::PendingApproval => StakeUpdateState::Pending,
            SolanaStakeUpdateState::Queued => StakeUpdateState::Pending,
        },
        type_: if attempt.amount < 0 {
            StakeUpdateType::Withdraw
        } else {
            StakeUpdateType::Deposit
        },
        currency: "USDC".into(),
        mint: usdc_mint,
        transaction_id: None,
    }
}

#[async_trait]
impl StakeService for DefaultStakeService {
    async fn by_wallet(&self, user_wallet: &str) -> Result<Option<Stake>> {
        let wallet = Pubkey::from_str(user_wallet)?;
        let stake = match self.solana.get_stake_by_wallet(wallet).await {
            Err(SolanaError::AccountNotFound(AccountNotFound::Stake { .. })) => return Ok(None),
            res => res?,
        };

        Ok(Some(stake))
    }

    async fn usdc_balance(&self, user_wallet: &str) -> Result<FPUSDC> {
        let balance = self
            .solana
            .get_usdc_balance_by_wallet(user_wallet.try_into().context("Can't parse the wallet address")?)
            .await?;
        Ok(balance)
    }

    async fn nez_balance(&self, user_wallet: &str) -> Result<FPUSDC> {
        let balance = self
            .solana
            .get_nez_balance_by_wallet(user_wallet.try_into().context("Can't parse the wallet address")?)
            .await?;
        Ok(balance)
    }

    async fn stake_update_by_transaction_id(&self, transaction_id: &TransactionId) -> Result<Option<StakeUpdate>> {
        self.stake_update_repo.by_transaction_id(transaction_id).await
    }

    async fn stake_updates_by_wallet(&self, user_wallet: &str) -> Result<Vec<StakeUpdate>> {
        let wallet = Pubkey::from_str(user_wallet)?;
        let mut stake_updates = Vec::new();
        let stake_update_request = self.solana.get_stake_update_request_by_wallet(wallet).await?;
        if let Some(request) = stake_update_request {
            let stake_update = stake_update_request_to_stake_update(request, self.solana.usdc_mint());
            stake_updates.push(stake_update);
        }
        stake_updates.extend(self.stake_update_repo.by_wallet(&wallet).await?);
        Ok(stake_updates)
    }

    async fn approve_stake_update(&self, user_wallet: &str) -> Result<StakeUpdate> {
        let wallet = Pubkey::from_str(user_wallet)?;
        let request = self.solana.get_stake_update_request_by_wallet(wallet).await?;
        let stake_update = match request {
            Some(request) => stake_update_request_to_stake_update(request, self.solana.usdc_mint()),
            None => bail!("Account not found"),
        };

        self.solana
            .approve_stake_update(
                wallet,
                match stake_update.type_ {
                    StakeUpdateType::Deposit => stake_update.amount.as_usdc_i64(),
                    StakeUpdateType::Withdraw => stake_update.amount.as_usdc_i64() * -1,
                },
            )
            .await?;
        Ok(stake_update)
    }

    async fn complete_stake_update(&self, user_wallet: &str) -> Result<StakeUpdate> {
        let wallet = Pubkey::from_str(user_wallet)?;
        let stake_update_request = self.solana.get_stake_update_request_by_wallet(wallet).await?;
        let mut stake_update = match stake_update_request {
            Some(request) => stake_update_request_to_stake_update(request, self.solana.usdc_mint()),
            None => bail!("Account not found"),
        };

        let transaction_id = self.solana.complete_stake_update(wallet).await?;
        stake_update.transaction_id = Some(transaction_id.into());
        stake_update.state = StakeUpdateState::Completed;
        let stake_update = self.stake_update_repo.store(&stake_update).await?;
        Ok(stake_update)
    }

    async fn all(&self) -> Result<Vec<Stake>> {
        let stakes = self.solana.get_all_stakes().await?;
        Ok(stakes)
    }

    async fn all_stake_update_requests(&self) -> Result<Vec<StakeUpdateRequest>> {
        let stake_update_requests = self.solana.get_all_stake_update_requests().await?;
        Ok(stake_update_requests)
    }
}
