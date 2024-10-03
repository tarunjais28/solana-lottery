use crate::WalletAddr;
use async_graphql::{Enum, InputObject, SimpleObject};
use itertools::Itertools;
use service::tickets;

#[derive(Enum, Eq, PartialEq, Copy, Clone, Debug)]
pub enum SequenceType {
    Normal,
    SignUpBonus,
    AirdropBonus,
}

impl From<tickets::SequenceType> for SequenceType {
    fn from(sequence_type: tickets::SequenceType) -> Self {
        match sequence_type {
            tickets::SequenceType::Normal => Self::Normal,
            tickets::SequenceType::SignUpBonus => Self::SignUpBonus,
            tickets::SequenceType::AirdropBonus => Self::AirdropBonus,
        }
    }
}

#[derive(SimpleObject, Debug)]
pub struct Sequence {
    pub nums: [u8; 6],
    pub sequence_type: SequenceType,
}

impl From<tickets::Sequence> for Sequence {
    fn from(sequence: tickets::Sequence) -> Self {
        Self {
            nums: sequence.nums,
            sequence_type: sequence.sequence_type.into(),
        }
    }
}

#[derive(SimpleObject, Debug)]
pub struct Ticket {
    pub(crate) wallet: WalletAddr,
    pub(crate) epoch_index: u64,
    pub(crate) arweave_url: Option<String>,
    pub(crate) sequences: Vec<Sequence>,
    pub(crate) balance: String,
    pub(crate) price: String,
}

impl From<tickets::Ticket> for Ticket {
    fn from(ticket: tickets::Ticket) -> Self {
        Self {
            wallet: WalletAddr(ticket.wallet.to_string()),
            epoch_index: ticket.epoch_index,
            arweave_url: ticket.arweave_url,
            sequences: ticket.sequences.into_iter().map(Sequence::from).collect_vec(),
            balance: ticket.balance,
            price: ticket.price,
        }
    }
}

#[derive(InputObject, Debug)]
pub struct WalletRisqId {
    pub(crate) wallet: WalletAddr,
    pub(crate) risq_id: String,
}

impl TryInto<tickets::WalletRisqId> for WalletRisqId {
    type Error = anyhow::Error;
    fn try_into(self) -> anyhow::Result<tickets::WalletRisqId> {
        Ok(tickets::WalletRisqId {
            wallet: self.wallet.try_into()?,
            risq_id: self.risq_id,
        })
    }
}

#[derive(SimpleObject, Debug)]
pub struct TicketsWithCount {
    pub(crate) tickets: Vec<Ticket>,
    pub(crate) count: usize,
}

impl From<service::model::ticket::TicketsWithCount> for TicketsWithCount {
    fn from(tickets: service::model::ticket::TicketsWithCount) -> Self {
        Self {
            tickets: tickets.tickets.into_iter().map(Ticket::from).collect_vec(),
            count: tickets.count,
        }
    }
}
