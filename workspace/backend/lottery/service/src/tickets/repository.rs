use anyhow::Result;
use async_trait::async_trait;
use solana_program::pubkey::Pubkey;
use std::{collections::HashSet, sync::RwLock};

use super::{Sequence, Ticket, TicketsWithCount, WalletRisqId};
use crate::model::ticket::{TicketError, SEQUENCE_LENGTH};

#[async_trait]
pub trait TicketRepository: Sync + Send {
    async fn by_wallet_and_epoch_index(&self, wallet: &Pubkey, index: u64) -> Result<Option<Ticket>>;
    async fn by_wallets_and_epoch_index(&self, wallets: &[Pubkey], index: u64) -> Result<Vec<Ticket>>;
    async fn by_epoch_index_and_prefix(&self, index: u64, limit: Option<u8>, prefix: &[u8])
        -> Result<TicketsWithCount>;
    async fn all(&self) -> Result<Vec<Ticket>>;
    async fn distinct_sequences_by_epoch_index(&self, index: u64) -> Result<Vec<[u8; 6]>>;
    async fn random_sequence_by_epoch_index(&self, index: u64) -> Result<Option<[u8; 6]>>;
    async fn create(&self, ticket: &Ticket) -> Result<Ticket>;
    async fn add_sequences(&self, wallet: &Pubkey, index: u64, sequences: &[Sequence]) -> Result<Ticket>;
    async fn update_arweave_url(&self, wallet: &Pubkey, index: u64, arweave_url: String) -> Result<Option<()>>;
    async fn get_unsubmitted_tickets_in_epoch(&self, epoch_index: u64) -> Result<Vec<Ticket>>;
    async fn update_risq_ids(&self, epoch_index: u64, risq_ids: &[WalletRisqId]) -> Result<Vec<Ticket>>;
    async fn num_sequences_by_epoch_index(&self, epoch_index: u64) -> Result<u64>;
    async fn num_airdrop_sequences_by_wallet_and_epoch_index(&self, wallet: &Pubkey, epoch_index: u64) -> Result<u32>;
    async fn prior_sequences_exist_by_wallet(&self, wallet: &Pubkey) -> Result<bool>;
    async fn draws_played_by_wallet(&self, wallet: &Pubkey) -> Result<u64>;
}

/// In memory implementation of TicketRepository for testing.
pub struct InMemoryTicketRepository {
    mem: RwLock<Vec<Ticket>>,
    num_airdrop_sequences: u32,
}

impl InMemoryTicketRepository {
    pub fn new(num_airdrop_sequences: u32) -> Self {
        Self {
            mem: RwLock::new(vec![]),
            num_airdrop_sequences,
        }
    }

    pub fn add(&mut self, entry: Ticket) {
        self.mem.write().unwrap().push(entry)
    }
}

#[async_trait]
impl TicketRepository for InMemoryTicketRepository {
    async fn by_wallet_and_epoch_index(&self, wallet: &Pubkey, index: u64) -> Result<Option<Ticket>> {
        Ok(self
            .mem
            .read()
            .unwrap()
            .iter()
            .find(|v| &v.wallet == wallet && v.epoch_index == index)
            .map(|v| v.clone()))
    }

    async fn by_wallets_and_epoch_index(&self, wallets: &[Pubkey], index: u64) -> Result<Vec<Ticket>> {
        Ok(self
            .mem
            .read()
            .unwrap()
            .iter()
            .filter(|v| wallets.contains(&v.wallet) && v.epoch_index == index)
            .map(|v| v.clone())
            .collect())
    }

    async fn by_epoch_index_and_prefix(
        &self,
        index: u64,
        limit: Option<u8>,
        prefix: &[u8],
    ) -> Result<TicketsWithCount> {
        let prefix_len = prefix.len();
        if prefix_len > SEQUENCE_LENGTH {
            return Err(TicketError::PrefixLengthExceeded(prefix_len).into());
        } else if prefix_len == 0 {
            return Err(TicketError::EmptyPrefix.into());
        }
        let mut tickets: Vec<Ticket> = self
            .mem
            .read()
            .unwrap()
            .iter()
            .filter(|v| v.epoch_index == index)
            .map(|ticket| {
                let sequences = ticket
                    .sequences
                    .iter()
                    .filter(|sequence| sequence.nums.iter().take(prefix_len).eq(prefix.iter()))
                    .map(|sequence| sequence.clone())
                    .collect();
                Ticket {
                    sequences,
                    ..ticket.clone()
                }
            })
            .collect();
        tickets.sort_by(|a, b| b.sequences.len().cmp(&a.sequences.len()));
        let count = tickets.len();
        let tickets: Vec<Ticket> = match limit {
            Some(limit) => tickets.into_iter().take(limit as usize).collect(),
            None => tickets,
        };
        Ok(TicketsWithCount { tickets, count })
    }

    async fn all(&self) -> Result<Vec<Ticket>> {
        Ok(self.mem.read().unwrap().clone())
    }

    async fn distinct_sequences_by_epoch_index(&self, index: u64) -> Result<Vec<[u8; 6]>> {
        let mut sequences = HashSet::new();
        for ticket in self.mem.read().unwrap().iter().filter(|v| v.epoch_index == index) {
            for sequence in &ticket.sequences {
                sequences.insert(sequence.nums.clone());
            }
        }
        Ok(sequences.into_iter().collect())
    }

    async fn random_sequence_by_epoch_index(&self, index: u64) -> Result<Option<[u8; 6]>> {
        Ok(self
            .mem
            .read()
            .unwrap()
            .iter()
            .filter(|v| v.epoch_index == index)
            .flat_map(|ticket| ticket.sequences.iter().map(|sequence| sequence.nums.clone()))
            .next())
    }

    async fn create(&self, ticket: &Ticket) -> Result<Ticket> {
        self.mem.write().unwrap().push(ticket.clone());
        Ok(ticket.clone())
    }

    async fn add_sequences(&self, wallet: &Pubkey, epoch_index: u64, sequences: &[Sequence]) -> Result<Ticket> {
        for t in &mut *self.mem.write().unwrap() {
            if t.wallet == *wallet && t.epoch_index == epoch_index {
                t.sequences.extend_from_slice(&sequences);
                return Ok(t.clone());
            }
        }
        anyhow::bail!("Ticket not found")
    }

    async fn update_arweave_url(&self, wallet: &Pubkey, index: u64, arweave_url: String) -> Result<Option<()>> {
        Ok(self
            .mem
            .write()
            .unwrap()
            .iter_mut()
            .find(|v| &v.wallet == wallet && v.epoch_index == index)
            .map(|v| v.arweave_url = Some(arweave_url)))
    }

    async fn get_unsubmitted_tickets_in_epoch(&self, epoch_index: u64) -> Result<Vec<Ticket>> {
        Ok(self
            .mem
            .read()
            .unwrap()
            .iter()
            .filter(|v| v.epoch_index == epoch_index && v.risq_id.is_none())
            .map(|v| v.clone())
            .collect())
    }

    async fn update_risq_ids(&self, epoch_index: u64, risq_ids: &[WalletRisqId]) -> Result<Vec<Ticket>> {
        Ok(self
            .mem
            .write()
            .unwrap()
            .iter_mut()
            .filter(|v| v.epoch_index == epoch_index)
            .map(|v| {
                if let Some(wr) = risq_ids.iter().find(|wr| wr.wallet == v.wallet) {
                    v.risq_id = Some(wr.risq_id.clone());
                }
                v
            })
            .map(|v| v.clone())
            .collect())
    }

    async fn num_sequences_by_epoch_index(&self, epoch_index: u64) -> Result<u64> {
        Ok(self
            .mem
            .read()
            .unwrap()
            .iter()
            .filter(|v| v.epoch_index == epoch_index)
            .map(|v| v.sequences.len() as u64)
            .sum())
    }

    async fn num_airdrop_sequences_by_wallet_and_epoch_index(
        &self,
        _wallet: &Pubkey,
        _epoch_index: u64,
    ) -> Result<u32> {
        Ok(self.num_airdrop_sequences)
    }

    async fn prior_sequences_exist_by_wallet(&self, wallet: &Pubkey) -> Result<bool> {
        Ok(self
            .mem
            .read()
            .unwrap()
            .iter()
            .any(|v| v.wallet == *wallet && !v.sequences.is_empty()))
    }

    async fn draws_played_by_wallet(&self, wallet: &Pubkey) -> Result<u64> {
        Ok(self
            .mem
            .read()
            .unwrap()
            .iter()
            .filter(|v| v.wallet == *wallet)
            .filter(|v| !v.sequences.is_empty())
            .count() as u64)
    }
}
