use std::{ops::Add, str::FromStr};

use crate::WalletAddr;

use super::{input::*, models::*};
use async_graphql::{Context, FieldResult, Object};
use chrono::{Duration, Utc};
use service::{
    epoch::{EpochManager, FPUSDC},
    model::epoch::UseCache,
    tickets::TicketService,
};
use solana_sdk::pubkey::Pubkey;

#[derive(Default)]
pub struct EpochsQuery;

#[Object]
impl EpochsQuery {
    pub async fn latest_epoch<'a>(
        &self,
        ctx: &'a Context<'_>,
        #[graphql(default = true)] use_cache: bool,
    ) -> FieldResult<Option<Epoch>> {
        let epoch_service = ctx.data::<Box<dyn EpochManager>>()?;

        let use_cache = if use_cache { UseCache::Yes } else { UseCache::No };
        let epoch = epoch_service.latest_epoch(use_cache).await?;
        Ok(epoch.map(Epoch::try_from).transpose()?)
    }

    pub async fn epochs<'a>(&self, ctx: &'a Context<'_>) -> FieldResult<Vec<Epoch>> {
        let epoch_service = ctx.data::<Box<dyn EpochManager>>()?;

        Ok(epoch_service
            .epochs()
            .await?
            .into_iter()
            .flat_map(|epoch| epoch.try_into())
            .collect())
    }

    pub async fn epoch<'a>(
        &self,
        ctx: &'a Context<'_>,
        index: u64,
        #[graphql(default = true)] use_cache: bool,
    ) -> FieldResult<Option<Epoch>> {
        let epoch_service = ctx.data::<Box<dyn EpochManager>>()?;

        let use_cache = if use_cache { UseCache::Yes } else { UseCache::No };
        let epoch = epoch_service.epoch_by_index(index, use_cache).await?;
        Ok(epoch.map(Epoch::try_from).transpose()?)
    }

    pub async fn epoch_by_pubkey<'a>(&self, ctx: &'a Context<'_>, pubkey: WalletAddr) -> FieldResult<Option<Epoch>> {
        let epoch_service = ctx.data::<Box<dyn EpochManager>>()?;

        match epoch_service.epoch_by_pubkey(&pubkey.try_into()?).await? {
            Some(epoch) => Ok(Some(epoch.try_into()?)),
            None => Ok(None),
        }
    }

    pub async fn wallet_prizes<'a>(&self, ctx: &'a Context<'_>, wallet: WalletAddr) -> FieldResult<Vec<UserPrize>> {
        let service = ctx.data::<Box<dyn EpochManager>>()?;

        let wallet = Pubkey::from_str(&wallet.0)?;

        let prizes = service
            .wallet_prizes(&wallet)
            .await?
            .into_iter()
            .map(|wallet_prize| UserPrize {
                epoch_index: wallet_prize.epoch_index,
                page: wallet_prize.page,
                winner_index: wallet_prize.winner.index,
                tier: wallet_prize.winner.tier,
                claimed: wallet_prize.winner.claimed,
                amount: wallet_prize.winner.prize.to_string(),
            })
            .collect();

        Ok(prizes)
    }
}

#[derive(Default)]
pub struct EpochMutation;

#[Object]
impl EpochMutation {
    pub async fn create_epoch<'a>(
        &self,
        ctx: &'a Context<'_>,
        prizes: PrizesInput,
        expected_duration_minutes: u32,
        yield_split_cfg: YieldSplitCfgInput,
    ) -> FieldResult<Epoch> {
        let epoch_service = ctx.data::<Box<dyn EpochManager>>()?;

        let expected_duration = Duration::minutes(expected_duration_minutes as i64);
        let expected_end = Utc::now().add(expected_duration);

        let yield_split_cfg = make_yield_split_cfg(prizes, yield_split_cfg)?;

        Ok(epoch_service.create_epoch(expected_end, yield_split_cfg).await?.into())
    }

    pub async fn publish_winners<'a>(&self, ctx: &'a Context<'_>) -> FieldResult<Epoch> {
        let epoch_service = ctx.data::<Box<dyn EpochManager>>()?;
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;

        let winners = ticket_service.calculate_winners().await?;

        Ok(epoch_service.publish_winners(winners).await?.into())
    }

    pub async fn enter_investment<'a>(&self, ctx: &'a Context<'_>, investor: Investor) -> FieldResult<Epoch> {
        let epoch_service = ctx.data::<Box<dyn EpochManager>>()?;
        Ok(epoch_service.enter_investment(investor.into()).await?.into())
    }

    pub async fn exit_investment<'a>(
        &self,
        ctx: &'a Context<'_>,
        investor: Investor,
        return_amount: Option<String>,
    ) -> FieldResult<Epoch> {
        let epoch_service = ctx.data::<Box<dyn EpochManager>>()?;
        let return_amount = return_amount.as_deref().map(FPUSDC::from_str).transpose()?;
        Ok(epoch_service
            .exit_investment(investor.into(), return_amount)
            .await?
            .into())
    }

    pub async fn fund_jackpot<'a>(&self, ctx: &'a Context<'_>) -> FieldResult<Epoch> {
        let epoch_service = ctx.data::<Box<dyn EpochManager>>()?;
        epoch_service.fund_jackpot().await?;

        // Just to statisfy the graphql requirement that we must return something.
        let latest_epoch = epoch_service
            .latest_epoch(UseCache::No)
            .await?
            .expect("If this epoch didn't exist, prev step would have failed");
        Ok(latest_epoch.into())
    }

    pub async fn publish_winning_combination<'a>(
        &self,
        ctx: &'a Context<'_>,
        winning_combination: [u8; 6],
    ) -> FieldResult<Epoch> {
        let epoch_service = ctx.data::<Box<dyn EpochManager>>()?;

        // TODO: Fix hack with another endpoint
        if winning_combination.eq(&[0; 6]) {
            let r = epoch_service
                .request_random_winning_combination()
                .await
                .map(Into::into)?;
            return Ok(r);
        }

        let epoch = epoch_service
            .publish_winning_combination(&winning_combination)
            .await
            .map(Into::into)?;

        Ok(epoch)
    }
}
