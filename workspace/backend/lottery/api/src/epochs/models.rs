use crate::WalletAddr;
use async_graphql::{ComplexObject, Context, Enum, FieldResult, SimpleObject};
use chrono::{DateTime, Utc};
use service::{
    epoch::EpochManager,
    model::{epoch, winner},
};

/// Represents a prize where the user is part of the winners, the Epoch is a pubkey, which can be used to retrieve
/// the epoch as well. This is a slow operation so the pubkey is returned to avoid unnecessary operations.
#[derive(SimpleObject, Debug)]
pub struct UserPrize {
    pub epoch_index: u64,
    pub page: u32,
    pub winner_index: u32,
    pub tier: u8,
    pub amount: String,
    pub claimed: bool,
}

/// TieredPrizes represent the prize definition for an epoch. For won prizes see [`EpochWinners`].
#[derive(SimpleObject, Debug)]
pub struct TieredPrizes {
    pub tier1: String,
    pub tier2_yield_share: u8,
    pub tier3_yield_share: u8,
}

/// Epoch represents the on-chain epoch.
///
/// Prizes are the defined prizes for the epoch, winners represent the wallets that have won and how much they won.
/// If if there are no wins the prize sizes are calculated and present, they will just have the empty Winner vec.
/// total_value_locked is in amount equivalent (USDC*10^6)
#[derive(SimpleObject, Debug)]
#[graphql(complex)]
pub struct Epoch {
    pub(crate) pubkey: WalletAddr,
    pub(crate) index: u64,
    pub(crate) status: EpochStatus,
    pub(crate) prizes: TieredPrizes,
    pub(crate) winning_combination: Option<[u8; 6]>,
    pub(crate) total_value_locked: Option<String>,
    pub(crate) total_returned: Option<String>,
    pub(crate) started_at: DateTime<Utc>,
    pub(crate) expected_end_date: DateTime<Utc>,
    pub(crate) ended_at: Option<DateTime<Utc>>,
    pub(crate) draw_enabled: DrawEnabled,
}

#[derive(Enum, Eq, PartialEq, Copy, Clone, Debug)]
pub enum EpochStatus {
    Running,
    Yielding,
    Finalising,
    Ended,
}

#[derive(Enum, Eq, PartialEq, Copy, Clone, Debug)]
pub enum DrawEnabled {
    Waiting,
    Draw,
    NoDraw,
}

impl From<epoch::EpochStatus> for EpochStatus {
    fn from(status: epoch::EpochStatus) -> Self {
        match status {
            epoch::EpochStatus::Running => Self::Running,
            epoch::EpochStatus::Yielding => Self::Yielding,
            epoch::EpochStatus::Finalising => Self::Finalising,
            epoch::EpochStatus::Ended => Self::Ended,
        }
    }
}

#[ComplexObject]
impl Epoch {
    async fn winners<'a>(&self, ctx: &'a Context<'_>) -> FieldResult<Option<EpochWinners>> {
        let epoch_service = ctx.data::<Box<dyn EpochManager>>()?;
        let epoch_winners = epoch_service.read_epoch_prizes(self.index).await?;
        Ok(epoch_winners.map(EpochWinners::from))
    }
}

impl From<service::model::epoch::Epoch> for Epoch {
    fn from(model: service::model::epoch::Epoch) -> Self {
        Self {
            pubkey: WalletAddr(model.pubkey.to_string()),
            index: model.index,
            status: model.status.into(),
            prizes: TieredPrizes {
                tier1: model.yield_split_cfg.jackpot.to_string(),
                tier2_yield_share: model.yield_split_cfg.tier2_prize_share,
                tier3_yield_share: model.yield_split_cfg.tier3_prize_share,
            },
            winning_combination: model.winning_combination.clone(),
            total_value_locked: model.total_invested.map(|amount| amount.to_string()),
            total_returned: model.returns.map(|returns| returns.total.to_string()),
            ended_at: model.ended_at,
            expected_end_date: model.expected_end_at,
            started_at: model.started_at,
            draw_enabled: match model.draw_enabled {
                Some(true) => DrawEnabled::Draw,
                Some(false) => DrawEnabled::NoDraw,
                None => DrawEnabled::Waiting,
            },
        }
    }
}

/// Winner represents a single winner of a prize, the amount they earned. Amount is USDC value * 10^(-decimals)
#[derive(SimpleObject, Debug)]
pub struct Winner {
    pub index: u32,
    pub address: WalletAddr,
    pub tier: u8,
    pub prize: String,
    pub claimed: bool,
}

impl From<winner::Winner> for Winner {
    fn from(model: winner::Winner) -> Self {
        Self {
            index: model.index,
            address: WalletAddr(model.address.to_string()),
            tier: model.tier,
            prize: model.prize.to_string(),
            claimed: model.claimed,
        }
    }
}

#[derive(SimpleObject, Debug)]
pub struct TierWinnersMeta {
    pub total_prize: String,
    pub total_num_winners: u32,
    pub total_num_winning_tickets: u32,
}

impl From<winner::TierWinnersMeta> for TierWinnersMeta {
    fn from(model: winner::TierWinnersMeta) -> Self {
        Self {
            total_prize: model.total_prize.to_string(),
            total_num_winners: model.total_num_winners,
            total_num_winning_tickets: model.total_num_winning_tickets,
        }
    }
}

#[derive(SimpleObject, Debug)]
pub struct EpochWinners {
    pub tier1_meta: TierWinnersMeta,
    pub tier2_meta: TierWinnersMeta,
    pub tier3_meta: TierWinnersMeta,
    pub jackpot_claimable: bool,
    pub winners: Vec<Winner>,
}

impl From<winner::EpochWinners> for EpochWinners {
    fn from(model: winner::EpochWinners) -> Self {
        Self {
            tier1_meta: model.tier1_meta.into(),
            tier2_meta: model.tier2_meta.into(),
            tier3_meta: model.tier3_meta.into(),
            jackpot_claimable: model.jackpot_claimable,
            winners: model.winners.into_iter().map(|w| w.into()).collect(),
        }
    }
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
pub enum Investor {
    Fracium,
    Fake,
}

impl From<Investor> for service::model::epoch::Investor {
    fn from(investor: Investor) -> Self {
        match investor {
            Investor::Fracium => Self::Francium,
            Investor::Fake => Self::Fake,
        }
    }
}
