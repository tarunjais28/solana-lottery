use crate::model::ticket::TicketsWithCount;
use solana_program::pubkey::Pubkey;
use std::collections::HashMap;
use std::collections::{BTreeMap, BTreeSet};

pub mod bonus;
mod price_calculators;
mod repository;
mod sequence;
mod service;
mod service_impl;

pub use self::price_calculators::*;
pub use self::repository::*;
pub use self::sequence::*;
pub use self::service::*;
pub use self::service_impl::*;

#[derive(Debug, Clone)]
pub struct Ticket {
    pub wallet: Pubkey,
    pub epoch_index: u64,
    pub arweave_url: Option<String>,
    pub sequences: Vec<Sequence>,
    pub balance: String,
    pub price: String,
    // The id returned by RISQ after we send the ticket to them
    pub risq_id: Option<String>,
}

impl PartialEq for Ticket {
    fn eq(&self, other: &Self) -> bool {
        self.wallet == other.wallet
            && self.epoch_index == other.epoch_index
            && self.arweave_url == other.arweave_url
            && self.balance == other.balance
            && self.price == other.price
            && self.risq_id == other.risq_id
            && eq_sequences(&self.sequences, &other.sequences)
    }
}

fn eq_sequences(a: &[Sequence], b: &[Sequence]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut a_map = HashMap::new();
    for seq in a.into_iter().cloned() {
        a_map.entry(seq).and_modify(|e| *e += 1).or_insert(1);
    }
    let mut b_map = HashMap::new();
    for seq in b.into_iter().cloned() {
        b_map.entry(seq).and_modify(|e| *e += 1).or_insert(1);
    }

    a_map == b_map
}

impl Ticket {
    pub fn new_for_tests() -> Self {
        Self {
            wallet: Pubkey::new_unique(),
            epoch_index: 0,
            arweave_url: None,
            sequences: Vec::new(),
            balance: "0".to_string(),
            price: "0".to_string(),
            risq_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WalletRisqId {
    pub wallet: Pubkey,
    pub risq_id: String,
}

#[derive(Debug)]
pub struct Winners {
    pub tier1: BTreeSet<Pubkey>,
    pub tier2: BTreeMap<Pubkey, u32>,
    pub tier3: BTreeMap<Pubkey, u32>,
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        sync::{Arc, Mutex},
    };

    use super::{
        bonus::{BonusInfo, BonusInfoService},
        ConstantTicketPriceCalculator, DefaultTicketService, InMemoryTicketRepository, Sequence, Ticket,
        TicketRepository, TicketService,
    };
    use crate::{
        model::epoch::EpochStatus,
        solana::mock::SolanaMock,
        tickets::{bonus::BonusSequenceCount, generate_sequences_with_type, SequenceType},
    };
    use anyhow::Result;
    use async_trait::async_trait;
    use nezha_staking::fixed_point::FPUSDC;
    use pretty_assertions::assert_eq;
    use rand::{rngs::StdRng, Rng, RngCore, SeedableRng};
    use rand_chacha::ChaChaRng;
    use solana_program::pubkey::Pubkey;
    use utils::with_mutex;

    pub struct MockBonusInfoService {
        pub bonus_info: BonusInfo,
    }

    impl MockBonusInfoService {
        pub fn new(bonus_info: BonusInfo) -> Self {
            Self { bonus_info }
        }
    }

    #[async_trait]
    impl BonusInfoService for MockBonusInfoService {
        async fn min_stake_amount(&self) -> FPUSDC {
            self.bonus_info.sub_seq_min_stake
        }

        async fn num_signup_bonus_sequences(&self, _normal_sequence_count: u32) -> Result<u32> {
            Ok(1)
        }
    }

    #[tokio::test]
    async fn test_ticket_service_updates_arweave_url() -> Result<()> {
        let rng = Arc::new(Mutex::new(ChaChaRng::from_entropy()));
        let wallet = Pubkey::new_unique();
        let epoch_index = 0;
        let balance: FPUSDC = "20.0".parse().unwrap();
        let price = "5.0".parse().unwrap();
        let ticket = Ticket {
            wallet: wallet.clone(),
            epoch_index,
            arweave_url: None,
            ..Ticket::new_for_tests()
        };
        let arweave_url = "test".to_owned();

        let mut ticket_repository = InMemoryTicketRepository::new(0);
        ticket_repository.add(ticket.clone());
        let price_calculator = ConstantTicketPriceCalculator::new(price);
        let solana = SolanaMock::new();
        let bonus_info_service = MockBonusInfoService::new(BonusInfo {
            sub_seq_count: BonusSequenceCount::Constant(1),
            sub_seq_min_stake: balance.change_precision(),
        });
        let ticket_service = DefaultTicketService::new(
            rng.clone(),
            Box::new(solana),
            Box::new(ticket_repository),
            Box::new(price_calculator),
            Box::new(bonus_info_service),
        );

        let actual_ticket = ticket_service
            .update_arweave_url(&wallet, epoch_index, arweave_url.clone())
            .await?
            .expect("Ticket reported as not found");

        let expected_ticket = Ticket {
            arweave_url: Some(arweave_url),
            ..ticket
        };

        assert_eq!(expected_ticket, actual_ticket);

        Ok(())
    }

    #[tokio::test]
    async fn test_ticket_service_reads_by_wallet_and_epoch_index() -> Result<()> {
        let rng = Arc::new(Mutex::new(ChaChaRng::from_entropy()));
        let wallet = Pubkey::new_unique();
        let epoch_index = 0;
        let balance: FPUSDC = "20.0".parse().unwrap();
        let price = "5.0".parse().unwrap();
        let expected_ticket = Ticket {
            wallet,
            epoch_index,
            ..Ticket::new_for_tests()
        };

        let mut ticket_repository = InMemoryTicketRepository::new(0);
        ticket_repository.add(expected_ticket.clone());
        let price_calculator = ConstantTicketPriceCalculator::new(price);
        let solana = SolanaMock::new();
        let bonus_info_service = MockBonusInfoService::new(BonusInfo {
            sub_seq_count: BonusSequenceCount::Constant(1),
            sub_seq_min_stake: balance.change_precision(),
        });
        let ticket_service = DefaultTicketService::new(
            rng.clone(),
            Box::new(solana),
            Box::new(ticket_repository),
            Box::new(price_calculator),
            Box::new(bonus_info_service),
        );

        let actual_ticket = ticket_service
            .read_ticket_by_wallet_and_epoch_index(&wallet, epoch_index)
            .await?
            .expect("Ticket reported as not found");

        assert_eq!(expected_ticket, actual_ticket);

        Ok(())
    }

    #[tokio::test]
    async fn test_ticket_service_generates_ticket() -> Result<()> {
        let rng = Arc::new(Mutex::new(StdRng::from_seed([0u8; 32])));
        let wallet = Pubkey::new_unique();
        let epoch_index = 0;
        let balance: FPUSDC = "20.0".parse().unwrap();
        let price: FPUSDC = "4.0".parse().unwrap();
        let sequences_count = balance.checked_div(price).unwrap().as_whole_number();
        assert_eq!(sequences_count, 5);

        let num_airdrop_sequences = 5;
        let ticket_repository = InMemoryTicketRepository::new(num_airdrop_sequences);
        let price_calculator = ConstantTicketPriceCalculator::new(price);
        let mut solana = SolanaMock::new();
        solana.epoch_index = epoch_index;
        solana.epoch_status = EpochStatus::Running;
        solana.stakes.push(crate::solana::Stake {
            owner: wallet,
            amount: balance,
            updated_epoch_index: epoch_index,
        });
        let bonus_info_service = MockBonusInfoService::new(BonusInfo {
            sub_seq_count: BonusSequenceCount::Constant(1),
            sub_seq_min_stake: balance.change_precision(),
        });
        let ticket_service = DefaultTicketService::new(
            rng,
            Box::new(solana),
            Box::new(ticket_repository),
            Box::new(price_calculator),
            Box::new(bonus_info_service),
        );

        let actual_ticket = ticket_service
            .generate_ticket_for_wallet(&wallet, Some(epoch_index))
            .await?;
        let rng = Mutex::new(StdRng::from_seed([0u8; 32]));
        let mut sequences = generate_sequences_with_type(&rng, None, sequences_count as _, SequenceType::Normal)?;
        let aidrop_bonus_sequences = generate_sequences_with_type(
            &rng,
            Some(&sequences.iter().map(|s| s.nums).collect::<HashSet<_>>()),
            num_airdrop_sequences,
            SequenceType::AirdropBonus,
        )?;
        sequences.extend(aidrop_bonus_sequences);
        let sub_sequences = generate_sequences_with_type(
            &rng,
            Some(&sequences.iter().map(|s| s.nums).collect::<HashSet<_>>()),
            1,
            SequenceType::SignUpBonus,
        )?;
        sequences.extend(sub_sequences);
        let expected_ticket = Ticket {
            wallet,
            epoch_index,
            balance: balance.to_string(),
            price: price.to_string(),
            sequences,
            ..Ticket::new_for_tests()
        };

        assert_eq!(expected_ticket, actual_ticket);

        Ok(())
    }

    #[tokio::test]
    async fn test_ticket_service_generates_ticket_correctly_when_not_running() -> Result<()> {
        let rng = Arc::new(Mutex::new(StdRng::from_seed([0u8; 32])));
        let wallet = Pubkey::new_unique();
        let epoch_index = 0;

        let price_calculator = ConstantTicketPriceCalculator::new("1.0".parse().unwrap());
        let mut solana = SolanaMock::new();
        solana.stakes.push(crate::solana::Stake {
            owner: wallet,
            amount: "1.0".parse().unwrap(),
            updated_epoch_index: 0,
        });

        for epoch_status in all_epoch_status().into_iter() {
            let expected_epoch_index = if epoch_status == EpochStatus::Running {
                epoch_index
            } else {
                epoch_index + 1
            };

            let ticket_repository = InMemoryTicketRepository::new(0);
            solana.epoch_index = epoch_index;
            solana.epoch_status = epoch_status;
            let bonus_info_service = MockBonusInfoService::new(BonusInfo {
                sub_seq_count: BonusSequenceCount::Constant(1),
                sub_seq_min_stake: "25.0".parse().unwrap(),
            });
            let ticket_service = DefaultTicketService::new(
                rng.clone(),
                Box::new(solana.clone()),
                Box::new(ticket_repository),
                Box::new(price_calculator.clone()),
                Box::new(bonus_info_service),
            );

            let actual_ticket = ticket_service.generate_ticket_for_wallet(&wallet, None).await?;
            assert_eq!(expected_epoch_index, actual_ticket.epoch_index);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_calculate_optimal_winning_combination() -> Result<()> {
        let mut rng = StdRng::from_seed([0u8; 32]);
        let epoch_index = rng.gen();

        let ticket_repository = InMemoryTicketRepository::new(0);

        let mut prefix: [u8; 6] = Default::default();
        rng.fill_bytes(&mut prefix);

        let rng = Mutex::new(rng);

        for _ in 0..10 {
            let sequences_count = with_mutex(&rng, |rng| rng.gen_range(1..=10));
            let mut ticket = Ticket {
                epoch_index,
                sequences: generate_sequences_with_type(&rng, None, sequences_count, SequenceType::Normal)?,
                ..Ticket::new_for_tests()
            };
            let prefix_len = with_mutex(&rng, |rng| rng.gen_range(4..=6));
            let mut sequence: [u8; 6] = Default::default();
            sequence[..prefix_len].clone_from_slice(&prefix[..prefix_len]);
            for i in prefix_len..6 {
                sequence[i] = with_mutex(&rng, |rng| rng.gen());
            }
            ticket.sequences.push(Sequence {
                nums: sequence,
                sequence_type: SequenceType::Normal,
            });
            ticket_repository.create(&ticket).await?;
        }
        let price_calculator = ConstantTicketPriceCalculator::new("1.0".parse().unwrap());
        let mut solana = SolanaMock::new();
        solana.epoch_index = epoch_index;
        solana.epoch_status = EpochStatus::Running;
        let bonus_info_service = MockBonusInfoService::new(BonusInfo {
            sub_seq_count: BonusSequenceCount::Constant(1),
            sub_seq_min_stake: "25.0".parse().unwrap(),
        });
        let ticket_service = DefaultTicketService::new(
            Arc::new(rng),
            Box::new(solana),
            Box::new(ticket_repository),
            Box::new(price_calculator),
            Box::new(bonus_info_service),
        );

        let sequence = ticket_service
            .calculate_optimal_winning_combination()
            .await?
            .expect("should have found a winning combination");
        assert_eq!(sequence[..4], prefix[..4]);

        Ok(())
    }

    fn all_epoch_status() -> Vec<EpochStatus> {
        let statuses = vec![
            EpochStatus::Running,
            EpochStatus::Yielding,
            EpochStatus::Finalising,
            EpochStatus::Ended,
        ];

        // This match statement requires all arms and so will fail to compile if a new status is added.
        // if that happens then statuses above also needs to be updated.
        statuses
            .iter()
            .map(|st| match st {
                EpochStatus::Running => EpochStatus::Running,
                EpochStatus::Yielding => EpochStatus::Yielding,
                EpochStatus::Finalising => EpochStatus::Finalising,
                EpochStatus::Ended => EpochStatus::Ended,
            })
            .collect()
    }
}
