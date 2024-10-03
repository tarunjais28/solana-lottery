use crate::{
    model::{
        epoch::{EpochError, EpochStatus},
        ticket::TicketsWithCount,
    },
    solana::{Solana, Stake},
    tickets::generate_sequences_with_type,
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use nezha_staking::fixed_point::FPUSDC;
use nezha_vrf_lib::state::NezhaVrfRequestStatus;
use rand::Rng;
use solana_program::pubkey::Pubkey;
use std::{
    cmp::{self, max},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::{Arc, Mutex},
};

use super::{
    bonus::BonusInfoService, Sequence, SequenceType, Ticket, TicketPriceCalculator, TicketRepository, TicketService,
    WalletRisqId, Winners,
};

pub struct DefaultTicketService<R: Rng> {
    rng: Arc<Mutex<R>>,
    solana: Box<dyn Solana>,
    repository: Box<dyn TicketRepository>,
    calculator: Box<dyn TicketPriceCalculator>,
    bonus_info_service: Box<dyn BonusInfoService>,
}

impl<R: Rng> DefaultTicketService<R> {
    pub fn new(
        rng: Arc<Mutex<R>>,
        solana: Box<dyn Solana>,
        repository: Box<dyn TicketRepository>,
        calculator: Box<dyn TicketPriceCalculator>,
        bonus_info_service: Box<dyn BonusInfoService>,
    ) -> Self {
        Self {
            rng,
            solana,
            repository,
            calculator,
            bonus_info_service,
        }
    }
}

#[async_trait]
impl<R: Rng + Sync + Send> TicketService for DefaultTicketService<R> {
    async fn read_ticket_by_wallet_and_epoch_index(&self, wallet: &Pubkey, index: u64) -> Result<Option<Ticket>> {
        self.repository.by_wallet_and_epoch_index(wallet, index).await
    }

    async fn read_tickets_by_epoch_index_and_prefix(
        &self,
        index: u64,
        limit: u8,
        prefix: &[u8],
    ) -> Result<TicketsWithCount> {
        let limit = cmp::min(limit, 40);
        self.repository
            .by_epoch_index_and_prefix(index, Some(limit), prefix)
            .await
    }

    async fn generate_ticket_for_stake(&self, stake: &Stake, epoch_index: Option<u64>) -> Result<Ticket> {
        let wallet = stake.owner;
        log::info!("generating ticket for stake: {stake:#?}");
        let epoch_index = match epoch_index {
            Some(epoch_index) => epoch_index,
            None => {
                let latest_epoch = self.solana.get_latest_epoch().await?;

                if latest_epoch.status == EpochStatus::Running {
                    latest_epoch.index
                } else {
                    latest_epoch.index + 1
                }
            }
        };
        // if the stake has a newer epoch index, we should use that
        let epoch_index = max(stake.updated_epoch_index, epoch_index);

        let balance = stake.amount.change_precision();
        let ticket_price = self.calculator.calculate(balance).await;

        log::info!("Reading existing ticket");
        if let Some(ticket) = self
            .repository
            .by_wallet_and_epoch_index(&wallet, epoch_index)
            .await
            .with_context(|| "Can't query existing ticket for the wallet")?
        {
            log::info!("Existing ticket found, adjusting sequences");
            let extra = adjust_sequences(
                &self.rng,
                &ticket.sequences,
                ticket_price.sequences_count,
                SequenceType::Normal,
            )?;

            log::info!("Saving sequences");
            let res = self
                .repository
                .add_sequences(&ticket.wallet, ticket.epoch_index, &extra)
                .await
                .with_context(|| "Error updating sequences")?;
            Ok(res)
        } else {
            log::info!("No existing ticket found, creating a new one");
            let mut sequences =
                generate_sequences_with_type(&self.rng, None, ticket_price.sequences_count, SequenceType::Normal)?;
            let mut unique_sequences = HashSet::from_iter(sequences.iter().cloned().map(|s| s.nums));

            // Airdrop bonus sequence generation
            let num_airdrop_bonus_sequences = self
                .repository
                .num_airdrop_sequences_by_wallet_and_epoch_index(&stake.owner, epoch_index)
                .await?;
            let airdrop_bonus_sequences = generate_sequences_with_type(
                &self.rng,
                Some(&unique_sequences),
                num_airdrop_bonus_sequences,
                SequenceType::AirdropBonus,
            )?;
            unique_sequences.extend(airdrop_bonus_sequences.iter().cloned().map(|s| s.nums));
            sequences.extend(airdrop_bonus_sequences);

            // Signup bonus sequence generation
            let num_signup_bonus_sequences = self.num_signup_bonus_sequences(&stake.owner, stake.amount).await?;
            let signup_bonus_sequences = generate_sequences_with_type(
                &self.rng,
                Some(&unique_sequences),
                num_signup_bonus_sequences,
                SequenceType::SignUpBonus,
            )?;
            sequences.extend(signup_bonus_sequences);

            let ticket = Ticket {
                wallet: wallet.clone(),
                epoch_index,
                sequences,
                price: ticket_price.price_per_ticket.to_string(),
                balance: balance.to_string(),
                risq_id: None,
                arweave_url: None,
            };

            log::info!("Saving the new ticket");
            let res = self
                .repository
                .create(&ticket)
                .await
                .with_context(|| "Error saving tickets")?;
            Ok(res)
        }
    }

    async fn generate_ticket_for_wallet(&self, wallet: &Pubkey, epoch_index: Option<u64>) -> Result<Ticket> {
        let stake = self.solana.get_stake_by_wallet(*wallet).await?;
        self.generate_ticket_for_stake(&stake, epoch_index).await
    }

    async fn generate_tickets_for_all(&self) -> Result<Vec<Result<Ticket>>> {
        let stakes = self.solana.get_all_stakes().await?;
        let latest_epoch = self.solana.get_latest_epoch().await?;
        let epoch_index = if latest_epoch.status == EpochStatus::Running {
            latest_epoch.index
        } else {
            latest_epoch.index + 1
        };

        let mut tickets = vec![];

        for stake in stakes {
            let ticket = self.generate_ticket_for_stake(&stake, Some(epoch_index)).await;

            tickets.push(ticket);
        }

        Ok(tickets)
    }

    async fn update_arweave_url(
        &self,
        wallet: &Pubkey,
        epoch_index: u64,
        arweave_url: String,
    ) -> Result<Option<Ticket>> {
        match self
            .repository
            .update_arweave_url(wallet, epoch_index, arweave_url)
            .await?
        {
            Some(_) => self.repository.by_wallet_and_epoch_index(wallet, epoch_index).await,
            None => Ok(None),
        }
    }

    async fn get_unsubmitted_tickets_in_epoch(&self, epoch_index: u64) -> Result<Vec<Ticket>> {
        self.repository.get_unsubmitted_tickets_in_epoch(epoch_index).await
    }

    async fn update_risq_ids(&self, epoch_index: u64, risq_ids: &[WalletRisqId]) -> Result<Vec<Ticket>> {
        self.repository.update_risq_ids(epoch_index, risq_ids).await
    }

    async fn calculate_winners(&self) -> Result<Winners> {
        let latest_epoch = self.solana.get_latest_epoch().await?;
        let epoch = self.solana.get_epoch_by_index(latest_epoch.index).await?;
        let vrf_req = self.solana.get_epoch_vrf_request(latest_epoch.index).await?;
        if !matches!(vrf_req.status, NezhaVrfRequestStatus::Success) {
            return Err(anyhow::anyhow!(
                "unable to calculate winners before vrf requests is successful"
            ));
        }

        let winning_combination = vrf_req
            .winning_combination
            .ok_or(EpochError::WinningCombinationNotSet)?;

        let mut winning_prefix = vec![0; 4];
        winning_prefix.clone_from_slice(&winning_combination[..4]);
        let tickets = self
            .repository
            .by_epoch_index_and_prefix(epoch.index, None, &winning_prefix)
            .await?
            .tickets;

        let mut winners = Winners {
            tier1: BTreeSet::new(),
            tier2: BTreeMap::new(),
            tier3: BTreeMap::new(),
        };
        for ticket in tickets {
            for sequence in ticket.sequences {
                let count = sequence
                    .nums
                    .into_iter()
                    .zip(winning_combination)
                    .take_while(|&(actual, expected)| actual == expected)
                    .count();

                match count {
                    6 => {
                        winners.tier1.insert(ticket.wallet);
                    }
                    5 => {
                        *winners.tier2.entry(ticket.wallet).or_insert(0) += 1;
                    }
                    4 => {
                        *winners.tier3.entry(ticket.wallet).or_insert(0) += 1;
                    }
                    _ => {}
                }
            }
        }
        Ok(winners)
    }

    async fn calculate_optimal_winning_combination(&self) -> Result<Option<[u8; 6]>> {
        let latest_epoch = self.solana.get_latest_epoch().await?;
        let epoch_index = latest_epoch.index;

        log::info!("getting distinct sequences for epoch {}", epoch_index);
        let sequences = self.repository.distinct_sequences_by_epoch_index(epoch_index).await?;

        log::info!("constructing prefix hash map");
        let mut prefix_map: HashMap<[u8; 4], ([u8; 6], usize)> = HashMap::new();
        for sequence in sequences {
            let mut prefix: [u8; 4] = Default::default();
            prefix.clone_from_slice(&sequence[..4]);
            prefix_map.entry(prefix).or_insert((sequence, 0)).1 += 1;
        }

        log::info!("calculating optimal winning combination from sequences");
        let optimal_sequence = prefix_map.into_values().max_by_key(|v| v.1).map(|v| v.0);
        Ok(optimal_sequence)
    }

    async fn random_winning_combination(&self) -> Result<Option<[u8; 6]>> {
        let latest_epoch = self.solana.get_latest_epoch().await?;
        let epoch_index = latest_epoch.index;

        let sequence = self.repository.random_sequence_by_epoch_index(epoch_index).await?;
        Ok(sequence)
    }

    async fn ticket_price(&self) -> Result<FPUSDC> {
        Ok(self.calculator.price().await)
    }

    async fn draws_played_by_wallet(&self, wallet: &Pubkey) -> Result<u64> {
        Ok(self.repository.draws_played_by_wallet(wallet).await?)
    }

    async fn num_signup_bonus_sequences(&self, wallet: &Pubkey, amount: FPUSDC) -> Result<u32> {
        let staked_min_amount = amount >= self.bonus_info_service.min_stake_amount().await;
        let is_first_time_user = !self.repository.prior_sequences_exist_by_wallet(wallet).await?;
        let normal_sequences_count = self.calculator.calculate(amount).await.sequences_count;

        // let normal_sequence_count = ;
        if staked_min_amount && is_first_time_user {
            self.bonus_info_service
                .num_signup_bonus_sequences(normal_sequences_count)
                .await
        } else {
            Ok(0)
        }
    }
}

fn adjust_sequences<R: Rng>(
    rng: &Mutex<R>,
    sequences: &[Sequence],
    sequences_count: u32,
    sequence_type: SequenceType,
) -> Result<Vec<Sequence>> {
    if sequences_count > sequences.len() as u32 {
        let num_sequences_extra = sequences_count - sequences.len() as u32;
        let prior = HashSet::from_iter(sequences.iter().map(|sequence| sequence.nums));
        let sequences_extra = generate_sequences_with_type(rng, Some(&prior), num_sequences_extra, sequence_type)?;
        Ok(sequences_extra)
    } else {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    mod adjust_tickets {
        use std::collections::BTreeSet;

        use rand::{prelude::StdRng, SeedableRng};
        use std::sync::Mutex;

        use crate::tickets::{generate_sequences, service_impl::adjust_sequences, Sequence, SequenceType};

        #[test]
        fn works() {
            let rng = Mutex::new(StdRng::from_seed([0; 32]));
            let mut sequences = generate_sequences(&rng, None, 10)
                .unwrap()
                .into_iter()
                .map(|sequence| Sequence {
                    nums: sequence,
                    sequence_type: SequenceType::Normal,
                })
                .collect::<Vec<_>>();
            assert_eq!(sequences.len(), 10); // sanity check
            assert_no_duplicates(sequences.iter()); // sanity check

            let new_sequences = adjust_sequences(&rng, &sequences, 20, SequenceType::Normal).unwrap();
            sequences.extend_from_slice(&new_sequences);
            assert_eq!(sequences.len(), 20);
            assert_no_duplicates(sequences.iter()); // test no duplicates are introduced by
                                                    // enlargement
        }

        fn assert_no_duplicates<T: Ord>(iter: impl Iterator<Item = T>) {
            let mut set = BTreeSet::new();
            let mut count = 0;

            for i in iter {
                set.insert(i);
                count += 1;
            }

            assert_eq!(set.len(), count);
        }
    }
}
