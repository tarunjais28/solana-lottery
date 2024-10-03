use super::models::*;
use crate::{TransactionId, WalletAddr};
use async_graphql::{Context, FieldResult, Object};
use service::{faucet::FaucetService, prize::PrizeService, stake::StakeService, transaction::UserTransactionService};

#[derive(Default)]
pub struct UsersQuery;

#[Object]
impl UsersQuery {
    pub async fn balances<'a>(&self, ctx: &'a Context<'_>, wallet: WalletAddr) -> FieldResult<Vec<Balance>> {
        let service = ctx.data::<Box<dyn StakeService>>()?;

        Ok(service
            .by_wallet(&wallet.0)
            .await?
            .into_iter()
            .map(|v| v.into())
            .collect())
    }

    async fn balance<'a>(&self, ctx: &'a Context<'_>, wallet: WalletAddr) -> FieldResult<String> {
        let service = ctx.data::<Box<dyn StakeService>>()?;
        let balance = service.usdc_balance(&wallet.0).await?;
        Ok(balance.to_string())
    }

    async fn nez_balance<'a>(&self, ctx: &'a Context<'_>, wallet: WalletAddr) -> FieldResult<String> {
        let service = ctx.data::<Box<dyn StakeService>>()?;
        let balance = service.nez_balance(&wallet.0).await?;
        Ok(balance.to_string())
    }

    pub async fn stake_updates_by_wallet<'a>(
        &self,
        ctx: &'a Context<'_>,
        wallet: WalletAddr,
    ) -> FieldResult<Vec<StakeUpdate>> {
        let service = ctx.data::<Box<dyn StakeService>>()?;

        let stake_updates = service.stake_updates_by_wallet(&wallet.0).await?;
        let stake_updates = stake_updates.into_iter().map(Into::into).collect();

        Ok(stake_updates)
    }

    pub async fn stake_update<'a>(
        &self,
        ctx: &'a Context<'_>,
        transaction_id: TransactionId,
    ) -> FieldResult<Option<StakeUpdate>> {
        let service = ctx.data::<Box<dyn StakeService>>()?;
        let stake_update = service.stake_update_by_transaction_id(&transaction_id.into()).await?;
        Ok(stake_update.map(Into::into))
    }

    pub async fn all_stake_update_requests<'a>(&self, ctx: &'a Context<'_>) -> FieldResult<Vec<StakeUpdateRequest>> {
        let service = ctx.data::<Box<dyn StakeService>>()?;
        let stake_update_requests = service.all_stake_update_requests().await?;
        Ok(stake_update_requests
            .into_iter()
            .map(StakeUpdateRequest::from)
            .collect())
    }

    pub async fn number_of_users<'a>(&self, ctx: &'a Context<'_>) -> FieldResult<usize> {
        let service = ctx.data::<Box<dyn StakeService>>()?;
        Ok(service.all().await?.len())
    }

    pub async fn transactions_by_wallet<'a>(
        &self,
        ctx: &'a Context<'_>,
        wallet: WalletAddr,
        limit: usize,
        offset: usize,
    ) -> FieldResult<Vec<Transaction>> {
        let service = ctx.data::<Box<dyn UserTransactionService>>()?;
        let transactions = service.by_wallet(&wallet.try_into()?, limit, offset).await?;
        Ok(transactions.into_iter().map(Into::into).collect())
    }

    pub async fn transactions_by_type<'a>(
        &self,
        ctx: &'a Context<'_>,
        transaction_type: TransactionType,
        limit: usize,
        offset: usize,
    ) -> FieldResult<Vec<Transaction>> {
        let service = ctx.data::<Box<dyn UserTransactionService>>()?;
        let txns = service.by_type(transaction_type.into(), limit, offset).await?;
        Ok(txns.into_iter().map(Into::into).collect())
    }

    pub async fn transactions_by_wallet_and_type<'a>(
        &self,
        ctx: &'a Context<'_>,
        wallet: WalletAddr,
        transaction_type: TransactionType,
        limit: usize,
        offset: usize,
    ) -> FieldResult<Vec<Transaction>> {
        let service = ctx.data::<Box<dyn UserTransactionService>>()?;
        let txns = service
            .by_wallet_and_type(&wallet.try_into()?, transaction_type.into(), limit, offset)
            .await?;
        Ok(txns.into_iter().map(Into::into).collect())
    }

    pub async fn total_deposit_by_wallet<'a>(&self, ctx: &'a Context<'_>, wallet: WalletAddr) -> FieldResult<String> {
        let service = ctx.data::<Box<dyn UserTransactionService>>()?;
        let total = service.total_deposit_by_wallet(&wallet.try_into()?).await?;
        Ok(total.to_string())
    }

    pub async fn prizes_by_wallet(
        &self,
        ctx: &Context<'_>,
        wallet: WalletAddr,
        limit: usize,
        offset: usize,
    ) -> FieldResult<Vec<Prize>> {
        let service = ctx.data::<Box<dyn PrizeService>>()?;
        let prizes = service.by_wallet(&wallet.try_into()?, limit, offset).await?;
        Ok(prizes.into_iter().map(Into::into).collect())
    }

    pub async fn total_prize_by_wallet(&self, ctx: &Context<'_>, wallet: WalletAddr) -> FieldResult<String> {
        let service = ctx.data::<Box<dyn PrizeService>>()?;
        let total = service.total_prize_by_wallet(&wallet.try_into()?).await?;
        Ok(total.to_string())
    }
}

#[derive(Default)]
pub struct UserMutation;

#[Object]
impl UserMutation {
    pub async fn approve_stake_update<'a>(&self, ctx: &'a Context<'_>, wallet: WalletAddr) -> FieldResult<StakeUpdate> {
        let service = ctx.data::<Box<dyn StakeService>>()?;
        let approved = service.approve_stake_update(&wallet.0).await?;
        Ok(approved.into())
    }

    pub async fn complete_stake_update<'a>(
        &self,
        ctx: &'a Context<'_>,
        wallet: WalletAddr,
    ) -> FieldResult<StakeUpdate> {
        let service = ctx.data::<Box<dyn StakeService>>()?;
        let approved = service.complete_stake_update(&wallet.0).await?;
        Ok(approved.into())
    }

    pub async fn mint_devnet_usdc<'a>(
        &self,
        ctx: &'a Context<'_>,
        wallet: WalletAddr,
    ) -> FieldResult<LatestMintTransaction> {
        let service = ctx.data::<Box<dyn FaucetService>>()?;
        Ok(service.mint_devnet_usdc(&wallet.try_into()?).await?.into())
    }
}
