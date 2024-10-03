use super::models::*;
use crate::WalletAddr;
use anyhow::anyhow;
use async_graphql::{Context, ErrorExtensions, FieldResult, Object};
use itertools::Itertools;
use service::{epoch::FPUSDC, tickets::TicketService};

#[derive(Default)]
pub struct TicketsQuery;

#[Object]
impl TicketsQuery {
    pub async fn ticket<'a>(
        &self,
        ctx: &'a Context<'_>,
        wallet: WalletAddr,
        epoch_index: u64,
    ) -> FieldResult<Option<Ticket>> {
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;

        Ok(ticket_service
            .read_ticket_by_wallet_and_epoch_index(&wallet.try_into()?, epoch_index)
            .await?
            .map(|ticket| ticket.into()))
    }

    pub async fn ticket_price<'a>(&self, ctx: &'a Context<'_>) -> FieldResult<String> {
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;
        let price = ticket_service.ticket_price().await?;
        Ok(price.to_string())
    }

    pub async fn tickets_by_epoch_index_and_prefix<'a>(
        &self,
        ctx: &'a Context<'_>,
        epoch_index: u64,
        limit: u8,
        prefix: Vec<u8>,
    ) -> FieldResult<TicketsWithCount> {
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;
        Ok(ticket_service
            .read_tickets_by_epoch_index_and_prefix(epoch_index, limit, &prefix)
            .await?
            .into())
    }

    pub async fn unsubmitted_tickets<'a>(&self, ctx: &'a Context<'_>, epoch_index: u64) -> FieldResult<Vec<Ticket>> {
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;

        Ok(ticket_service
            .get_unsubmitted_tickets_in_epoch(epoch_index)
            .await?
            .into_iter()
            .map(|ticket| ticket.into())
            .collect())
    }

    pub async fn calculate_optimal_winning_combination(&self, ctx: &Context<'_>) -> FieldResult<Option<[u8; 6]>> {
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;

        let sequence = ticket_service.calculate_optimal_winning_combination().await?;
        Ok(sequence.map(|seq| seq))
    }

    pub async fn random_winning_combination(&self, ctx: &Context<'_>) -> FieldResult<Option<[u8; 6]>> {
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;

        let sequence = ticket_service.random_winning_combination().await?;
        Ok(sequence.map(|seq| seq))
    }

    pub async fn num_signup_bonus_sequences<'a>(
        &self,
        ctx: &'a Context<'_>,
        wallet: WalletAddr,
        amount: String,
    ) -> FieldResult<u32> {
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;
        let amount = amount.parse::<FPUSDC>().map_err(|e| anyhow!(e))?;

        Ok(ticket_service
            .num_signup_bonus_sequences(&wallet.try_into()?, amount)
            .await?)
    }

    pub async fn draws_played_by_wallet<'a>(&self, ctx: &'a Context<'_>, wallet: WalletAddr) -> FieldResult<u64> {
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;

        Ok(ticket_service.draws_played_by_wallet(&wallet.try_into()?).await?)
    }
}

#[derive(Default)]
pub struct TicketMutation;

#[Object]
impl TicketMutation {
    pub async fn generate_ticket(&self, ctx: &Context<'_>, wallet: WalletAddr) -> FieldResult<Ticket> {
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;

        Ok(ticket_service
            .generate_ticket_for_wallet(&wallet.try_into()?, None)
            .await?
            .into())
    }

    pub async fn generate_tickets_for_all(&self, ctx: &Context<'_>) -> FieldResult<Vec<Ticket>> {
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;
        let tickets = ticket_service.generate_tickets_for_all().await?;

        Ok(tickets
            .into_iter()
            .filter_map(|v| match v {
                Ok(ticket) => Some(ticket.into()),
                Err(e) => {
                    log::error!("Error generating ticket: {}", e);
                    None
                }
            })
            .collect())
    }

    pub async fn update_arweave_url(
        &self,
        ctx: &Context<'_>,
        wallet: WalletAddr,
        epoch_index: u64,
        arweave_url: String,
    ) -> FieldResult<Option<Ticket>> {
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;

        Ok(ticket_service
            .update_arweave_url(&wallet.try_into()?, epoch_index, arweave_url)
            .await?
            .map(|ticket| ticket.into()))
    }

    pub async fn update_risq_ids(
        &self,
        ctx: &Context<'_>,
        epoch_index: u64,
        risq_ids: Vec<WalletRisqId>,
    ) -> FieldResult<Vec<Ticket>> {
        let ticket_service = ctx.data::<Box<dyn TicketService>>()?;

        let risq_ids = bail_if_any_fail(
            risq_ids.into_iter().map(|x| x.try_into()),
            "failed to update risq ids, see details",
        )?;

        Ok(ticket_service
            .update_risq_ids(epoch_index, &risq_ids)
            .await?
            .into_iter()
            .map(|ticket| ticket.into())
            .collect())
    }
}

fn bail_if_any_fail<T>(
    results: impl Iterator<Item = anyhow::Result<T>>,
    msg: &'static str,
) -> async_graphql::Result<Vec<T>> {
    let (oks, errors): (Vec<_>, Vec<_>) = results.partition_result();

    let errors: Vec<_> = errors
        .into_iter()
        .dedup_by(|a, b: &anyhow::Error| a.to_string() == b.to_string())
        .collect();

    if errors.is_empty() {
        let values: Vec<_> = oks.into_iter().collect();
        Ok(values)
    } else {
        Err(anyhow!(msg).extend_with(|_, e| {
            e.set(
                "details",
                errors.into_iter().map(|v| v.to_string()).collect::<Vec<String>>(),
            )
        }))
    }
}
