use std::collections::HashSet;

use anyhow::Result;
use pretty_assertions::{assert_eq, assert_ne};
use rand::{distributions::Standard, prelude::SliceRandom, thread_rng, Rng};
use rust_decimal::Decimal;
use service::{
    model::ticket::TicketError,
    tickets::{Sequence, SequenceType, Ticket, TicketRepository, WalletRisqId},
};
use solana_sdk::pubkey::{self, Pubkey};
use store::{get_client, tickets::PostgresTicketRepository};

use crate::common;

fn create_ticket() -> Ticket {
    let mut rng = thread_rng();

    let mut sequences = Vec::new();
    let sequences_count: usize = rng.gen_range(0..=5);
    for _ in 0..sequences_count {
        let sequence_type = match rng.gen_range(0..3) {
            0 => SequenceType::Normal,
            1 => SequenceType::SignUpBonus,
            2 => SequenceType::AirdropBonus,
            _ => unreachable!(),
        };
        let mut sequence = Sequence {
            nums: [0; 6],
            sequence_type,
        };
        for i in 0..6 {
            let x = rng.gen::<u8>();
            sequence.nums[i] = x;
        }
        sequences.push(sequence);
    }

    // We need this to be random so that we don't have to reset the DB between re-runs of the
    // tests.
    let wallet = pubkey::new_rand();

    // Randomly set risq_id to non-null value approx 50% of the time
    let risq_id = {
        let x: u64 = rng.gen();
        if x % 2 == 0 {
            Some(x.to_string())
        } else {
            None
        }
    };

    Ticket {
        arweave_url: None,
        balance: rng.gen::<u64>().to_string(),
        epoch_index: rng.gen(),
        price: rng.gen::<u64>().to_string(),
        sequences,
        wallet,
        risq_id,
    }
}

async fn get_repo() -> PostgresTicketRepository {
    let pool = common::setup().await;
    PostgresTicketRepository::new(pool)
}

#[tokio::test]
async fn test_create() -> Result<()> {
    let repo = get_repo().await;
    let ticket = create_ticket();
    let stored = repo.create(&ticket).await?;
    assert_eq!(stored, ticket);
    Ok(())
}

#[tokio::test]
async fn test_add_sequences() -> Result<()> {
    let repo = get_repo().await;

    // Create a second ticket to make sure we don't update the wrong entry
    let unchanged_ticket = create_ticket();
    repo.create(&unchanged_ticket).await?;
    let assert_unchanged = || async {
        let res = repo
            .by_wallet_and_epoch_index(&unchanged_ticket.wallet, unchanged_ticket.epoch_index)
            .await?;
        assert_eq!(res.as_ref(), Some(&unchanged_ticket));
        let res: Result<()> = Ok(());
        res
    };
    assert_unchanged().await?; // sanity check - assert_unchanged works

    let mut ticket = create_ticket();
    repo.create(&ticket).await?;

    let random_ticket = create_ticket();

    let new_sequences = random_ticket.sequences;
    ticket.sequences.extend_from_slice(&new_sequences);

    let res = repo
        .add_sequences(&ticket.wallet, ticket.epoch_index, &new_sequences)
        .await?;
    assert_eq!(res, ticket);

    let res = repo
        .by_wallet_and_epoch_index(&ticket.wallet, ticket.epoch_index)
        .await?;
    assert_eq!(res, Some(ticket));

    assert_unchanged().await?; // check the second ticket was unchanged

    Ok(())
}

#[tokio::test]
async fn test_update_arweave_url() -> Result<()> {
    let repo = get_repo().await;

    let ticket_unchanged = create_ticket();
    let _stored = repo.create(&ticket_unchanged).await?;

    let ticket = create_ticket();
    let _stored = repo.create(&ticket).await?;

    // Set to some

    let arweave_url = "NEW".to_string();
    repo.update_arweave_url(&ticket.wallet, ticket.epoch_index, arweave_url)
        .await?;

    let ticket_found = repo
        .by_wallet_and_epoch_index(&ticket.wallet, ticket.epoch_index)
        .await?
        .unwrap();

    assert_eq!(ticket_found.arweave_url, Some("NEW".to_string()));

    // Shouldn't change unchanged's arweave_url

    let ticket_found = repo
        .by_wallet_and_epoch_index(&ticket_unchanged.wallet, ticket_unchanged.epoch_index)
        .await?
        .unwrap();

    assert_eq!(ticket_found.arweave_url, ticket_unchanged.arweave_url);

    Ok(())
}

#[tokio::test]
async fn test_by_wallet_and_epoch_index() -> Result<()> {
    let repo = get_repo().await;

    // The ticket we are going to store and search for

    let ticket1 = create_ticket();
    let _stored = repo.create(&ticket1).await?;

    // A second ticket to ensure that the filters are working

    let ticket2 = create_ticket();
    let _stored = repo.create(&ticket2).await?;

    let ticket_found = repo
        .by_wallet_and_epoch_index(&ticket1.wallet, ticket1.epoch_index)
        .await?;

    assert_eq!(ticket_found, Some(ticket1.clone()));

    // Nonexistent combos

    let ticket_found = repo
        .by_wallet_and_epoch_index(&ticket2.wallet, ticket1.epoch_index)
        .await?;

    assert_eq!(ticket_found, None);

    let ticket_found = repo
        .by_wallet_and_epoch_index(&ticket1.wallet, ticket2.epoch_index)
        .await?;

    assert_eq!(ticket_found, None);

    Ok(())
}

#[tokio::test]
async fn test_by_wallets_and_epoch_index() -> Result<()> {
    let repo = get_repo().await;

    let wallet1 = Pubkey::new_unique();
    let wallet2 = Pubkey::new_unique();

    let epoch_index1 = thread_rng().gen();
    let epoch_index2 = thread_rng().gen();

    let ticket_wallet1_epoch1 = {
        let mut ticket = create_ticket();
        ticket.wallet = wallet1;
        ticket.epoch_index = epoch_index1;
        let _stored = repo.create(&ticket).await?;
        ticket
    };

    let _ticket_wallet1_epoch2 = {
        let mut ticket = create_ticket();
        ticket.wallet = wallet1;
        ticket.epoch_index = epoch_index2;
        let _stored = repo.create(&ticket).await?;
        ticket
    };

    let ticket_wallet2_epoch1 = {
        let mut ticket = create_ticket();
        ticket.wallet = wallet2;
        ticket.epoch_index = epoch_index1;
        let _stored = repo.create(&ticket).await?;
        ticket
    };

    let _ticket_wallet2_epoch2 = {
        let mut ticket = create_ticket();
        ticket.wallet = wallet2;
        ticket.epoch_index = epoch_index2;
        let _stored = repo.create(&ticket).await?;
        ticket
    };

    let mut expected = Vec::new();
    expected.push(ticket_wallet1_epoch1.clone());
    expected.push(ticket_wallet2_epoch1.clone());
    expected.sort_by_key(|t| (t.wallet, t.epoch_index));

    let mut tickets_found = repo
        .by_wallets_and_epoch_index(&[wallet1, wallet2], epoch_index1)
        .await?;
    tickets_found.sort_by_key(|t| (t.wallet, t.epoch_index));

    assert_eq!(tickets_found, expected);

    Ok(())
}

#[tokio::test]
async fn test_by_epoch_index_and_prefix() -> Result<()> {
    let mut rng = thread_rng();
    let repo = get_repo().await;
    for _ in 0..10 {
        let ticket = create_ticket();
        let _ = repo.create(&ticket).await?;
    }

    let epoch_index: u64 = rng.gen();
    for _ in 0..10 {
        let ticket = Ticket {
            epoch_index: epoch_index,
            ..create_ticket()
        };
        let _ = repo.create(&ticket).await?;
    }

    let mut expected = Vec::new();
    let prefix_len: usize = rng.gen_range(1..=6);
    let prefix: Vec<u8> = (&mut rng).sample_iter(Standard).take(prefix_len).collect();
    for _ in 0..40 {
        let mut sequences = vec![];
        for _ in 0..rng.gen_range(0..=5usize) {
            let mut sequence: [u8; 6] = Default::default();
            sequence[..prefix_len].clone_from_slice(&prefix);
            for i in prefix_len..6 {
                sequence[i] = rng.gen();
            }
            sequences.push(Sequence {
                nums: sequence,
                sequence_type: SequenceType::Normal,
            });
        }
        let ticket = Ticket {
            epoch_index,
            sequences,
            ..create_ticket()
        };
        let _stored = repo.create(&ticket).await?;
        expected.push(ticket);
    }
    assert_ne!(expected.len(), 0);
    let limit = 10;
    let actual = repo
        .by_epoch_index_and_prefix(epoch_index, Some(limit), &prefix)
        .await?;
    assert!(actual.count >= limit as usize);
    assert_ne!(actual.tickets.len(), 0);
    assert!(actual.tickets.len() <= limit as usize);
    assert!(actual
        .tickets
        .iter()
        .all(|t| t.epoch_index == epoch_index && t.sequences.iter().all(|s| s.nums.starts_with(&prefix))));

    Ok(())
}

#[tokio::test]
async fn test_by_epoch_index_and_prefix_length_exceeded() -> Result<()> {
    let mut rng = thread_rng();
    let repo = get_repo().await;
    let prefix = vec![0; 7];
    let epoch_index = rng.gen();
    let res = repo.by_epoch_index_and_prefix(epoch_index, None, &prefix).await;
    assert!(res.is_err(), "{res:?}");
    let error = res.expect_err("could not get error");
    let error = error
        .downcast_ref::<TicketError>()
        .expect("could not downcast error to TicketError");
    assert!(matches!(error, TicketError::PrefixLengthExceeded(7)));
    Ok(())
}

#[tokio::test]
async fn test_by_epoch_index_and_prefix_empty_prefix() -> Result<()> {
    let mut rng = thread_rng();
    let repo = get_repo().await;
    let epoch_index = rng.gen();
    let res = repo.by_epoch_index_and_prefix(epoch_index, None, &[]).await;
    assert!(res.is_err(), "{res:?}");
    let error = res.expect_err("could not get error");
    let error = error
        .downcast_ref::<TicketError>()
        .expect("could not downcast error to TicketError");
    assert!(matches!(error, TicketError::EmptyPrefix));
    Ok(())
}

#[tokio::test]
async fn test_all() -> Result<()> {
    let repo = get_repo().await;
    let mut tickets = Vec::new();

    let mut keys = HashSet::new();

    for _ in 0..10 {
        let ticket = create_ticket();
        let _stored = repo.create(&ticket).await?;
        keys.insert(ticket.wallet);
        tickets.push(ticket);
    }
    tickets.sort_by_key(|t| t.wallet);
    assert_ne!(tickets.len(), 0);
    assert_ne!(keys.len(), 0);

    let all_tickets = repo.all().await?;
    assert_ne!(all_tickets.len(), 0);

    // filter out tickets which may have been inserted by previous runs of the test or other
    // parallely running tests.
    // This is doable because we are using a random pubkey which has sufficient entropy.
    let mut tickets_of_this_test: Vec<Ticket> = all_tickets.into_iter().filter(|t| keys.contains(&t.wallet)).collect();
    tickets_of_this_test.sort_by_key(|t| t.wallet);

    assert_ne!(tickets_of_this_test.len(), 0);
    assert_ne!(tickets.len(), 0);

    assert_eq!(tickets_of_this_test, tickets);

    Ok(())
}

#[tokio::test]
async fn test_distinct_sequences_by_epoch_index() -> Result<()> {
    let pool = common::setup().await;
    let client = get_client(&pool).await?;
    let mut rng = thread_rng();
    let epoch_index = rng.gen();
    // make sure we don't have any tickets with the same epoch_index
    client
        .execute(
            "DELETE FROM sequences WHERE epoch_index = $1",
            &[&Decimal::from(epoch_index)],
        )
        .await?;

    let repo = get_repo().await;
    let mut sequences = vec![];
    for _ in 0..10 {
        let mut sequence: [u8; 6] = Default::default();
        for i in 0..6 {
            sequence[i] = rng.gen();
        }
        sequences.push(Sequence {
            nums: sequence,
            sequence_type: SequenceType::Normal,
        });
    }

    for _ in 0..10 {
        let amount = rng.gen();
        let sequences = sequences
            .choose_multiple(&mut rng, amount)
            .cloned()
            .collect::<Vec<Sequence>>();
        let ticket = Ticket {
            epoch_index,
            sequences,
            ..create_ticket()
        };
        let _stored = repo.create(&ticket).await?;
    }

    let expected_sequences: HashSet<[u8; 6]> =
        HashSet::from_iter(sequences.into_iter().map(|sequence| sequence.into()));
    let random_sequence = repo
        .random_sequence_by_epoch_index(epoch_index)
        .await?
        .expect("could not get random sequence");
    assert!(expected_sequences.contains(&random_sequence));

    Ok(())
}

#[tokio::test]
async fn test_random_sequence_by_epoch_index() -> Result<()> {
    let pool = common::setup().await;
    let client = get_client(&pool).await?;
    let mut rng = thread_rng();
    let epoch_index = rng.gen();
    // make sure we don't have any tickets with the same epoch_index
    client
        .execute(
            "DELETE FROM sequences WHERE epoch_index = $1",
            &[&Decimal::from(epoch_index)],
        )
        .await?;

    let repo = get_repo().await;
    let mut sequences = vec![];
    for _ in 0..10 {
        let mut sequence: [u8; 6] = Default::default();
        for i in 0..6 {
            sequence[i] = rng.gen();
        }
        sequences.push(Sequence {
            nums: sequence,
            sequence_type: SequenceType::Normal,
        });
    }

    for _ in 0..10 {
        let amount = rng.gen();
        let sequences = sequences
            .choose_multiple(&mut rng, amount)
            .cloned()
            .collect::<Vec<Sequence>>();
        let ticket = Ticket {
            epoch_index,
            sequences,
            ..create_ticket()
        };
        let _stored = repo.create(&ticket).await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_get_unsubmitted_tickets_in_epoch() -> Result<()> {
    let repo = get_repo().await;
    let mut rng = thread_rng();

    let this_epoch = rng.gen();

    let good_ticket = || {
        // - In this epoch
        // - risq_id is None
        let mut t = create_ticket();
        t.epoch_index = this_epoch;
        t.risq_id = None;
        t
    };

    let good_1 = {
        let t = good_ticket();
        repo.create(&t).await?;
        t
    };

    let good_2 = {
        let t = good_ticket();
        repo.create(&t).await?;
        t
    };

    let not_in_this_epoch = {
        let mut t = good_ticket();
        t.epoch_index = rng.gen();
        repo.create(&t).await?;
        t
    };

    let with_risq_id = {
        let mut t = good_ticket();
        t.risq_id = Some("risq_id".to_string());
        repo.create(&t).await?;
        t
    };

    let tickets = repo.get_unsubmitted_tickets_in_epoch(this_epoch).await?;
    assert_ne!(tickets.len(), 0);

    let mut good_1_found = false;
    let mut good_2_found = false;
    let mut not_in_this_epoch_found = false;
    let mut with_risq_id_found = false;

    for ticket in tickets {
        if ticket.wallet == good_1.wallet {
            good_1_found = true;
        } else if ticket.wallet == good_2.wallet {
            good_2_found = true;
        } else if ticket.wallet == not_in_this_epoch.wallet {
            not_in_this_epoch_found = true;
        } else if ticket.wallet == with_risq_id.wallet {
            with_risq_id_found = true;
        } else {
            // ignore tickets not created by this test
        }
    }

    assert!(good_1_found);
    assert!(good_2_found);
    assert!(!not_in_this_epoch_found);
    assert!(!with_risq_id_found);

    Ok(())
}

#[tokio::test]
async fn test_update_risq_ids() -> Result<()> {
    let repo = get_repo().await;
    let mut rng = thread_rng();

    let this_epoch = rng.gen();

    let mut wallet_risq_ids = Vec::new();

    let good_ticket = || {
        // - In this epoch
        // - Doesn't have an existing risq_id
        let mut t = create_ticket();
        t.epoch_index = this_epoch;
        t.risq_id = None;
        t
    };

    // tickets

    let good_1 = {
        let t = good_ticket();
        repo.create(&t).await?;
        wallet_risq_ids.push(WalletRisqId {
            wallet: t.wallet,
            risq_id: "good_1".to_string(),
        });
        t
    };

    let good_2 = {
        let t = good_ticket();
        repo.create(&t).await?;
        wallet_risq_ids.push(WalletRisqId {
            wallet: t.wallet,
            risq_id: "good_2".to_string(),
        });
        t
    };

    let not_asked_to_update = {
        let t = good_ticket();
        repo.create(&t).await?;
        t
    };

    let not_in_this_epoch = {
        let mut t = good_ticket();
        t.epoch_index = rng.gen();
        repo.create(&t).await?;
        wallet_risq_ids.push(WalletRisqId {
            wallet: t.wallet,
            risq_id: "not_in_this_epoch".to_string(),
        });
        t
    };

    let with_existing_risq_id = {
        let mut t = good_ticket();
        t.risq_id = Some("existing_not_updated".to_string());
        repo.create(&t).await?;
        wallet_risq_ids.push(WalletRisqId {
            wallet: t.wallet,
            risq_id: "existing_updated".to_string(),
        });
        t
    };

    // Update and test results

    let tickets_updated = repo.update_risq_ids(this_epoch, &wallet_risq_ids).await?;
    assert_eq!(tickets_updated.len(), 2);

    let mut good_1_found = false;
    let mut good_2_found = false;

    for ticket in tickets_updated {
        if ticket.wallet == good_1.wallet {
            assert_eq!(ticket.risq_id, Some("good_1".to_string()));
            good_1_found = true;
        } else if ticket.wallet == good_2.wallet {
            assert_eq!(ticket.risq_id, Some("good_2".to_string()));
            good_2_found = true;
        } else {
            unreachable!()
        }
    }

    assert!(good_1_found);
    assert!(good_2_found);

    // Make sure it's reflected in `all()` also

    let tickets_all = repo.all().await?;
    assert_ne!(tickets_all.len(), 0);

    let mut good_1_found = false;
    let mut good_2_found = false;
    let mut not_asked_to_update_found = false;
    let mut not_in_this_epoch_found = false;
    let mut with_existing_risq_id_found = false;

    for ticket in tickets_all {
        if ticket.wallet == good_1.wallet {
            assert_eq!(ticket.risq_id, Some("good_1".to_string()));
            good_1_found = true;
        } else if ticket.wallet == good_2.wallet {
            assert_eq!(ticket.risq_id, Some("good_2".to_string()));
            good_2_found = true;
        } else if ticket.wallet == not_asked_to_update.wallet {
            assert_eq!(ticket.risq_id, None);
            not_asked_to_update_found = true;
        } else if ticket.wallet == not_in_this_epoch.wallet {
            assert_eq!(ticket.risq_id, None);
            not_in_this_epoch_found = true;
        } else if ticket.wallet == with_existing_risq_id.wallet {
            assert_eq!(ticket.risq_id, Some("existing_not_updated".to_string()));
            with_existing_risq_id_found = true;
        } else {
            // ignore tickets not created by this test
        }
    }

    assert!(good_1_found);
    assert!(good_2_found);
    assert!(not_asked_to_update_found);
    assert!(not_in_this_epoch_found);
    assert!(with_existing_risq_id_found);

    Ok(())
}

#[tokio::test]
async fn test_num_sequences_by_epoch_index() -> Result<()> {
    let pool = common::setup().await;
    let client = get_client(&pool).await?;
    let mut rng = thread_rng();
    let epoch_index = rng.gen::<u64>();
    // make sure we don't have any tickets with the same epoch_index
    client
        .execute(
            "DELETE FROM sequences WHERE epoch_index = $1",
            &[&Decimal::from(epoch_index)],
        )
        .await?;
    let repo = get_repo().await;
    let num_tickets = rng.gen_range(10..=100u64);
    let mut expected_num_sequences = 0;
    for _ in 0..num_tickets {
        let mut ticket = create_ticket();
        ticket.epoch_index = epoch_index;
        let _ = repo.create(&ticket).await?;
        expected_num_sequences += ticket.sequences.len() as u64;
    }
    let actual_num_sequences = repo.num_sequences_by_epoch_index(epoch_index).await?;
    assert_eq!(expected_num_sequences, actual_num_sequences);
    Ok(())
}

#[tokio::test]
async fn test_num_airdrop_sequences_by_wallet_and_epoch_index() -> Result<()> {
    let pool = common::setup().await;
    let client = get_client(&pool).await?;
    let mut rng = thread_rng();
    let epoch_index = rng.gen::<u64>();
    let wallet = Pubkey::new_unique();
    let mut total_ad_seq_count = 0u32;
    for i in 1..rng.gen_range(2..=10usize) {
        let num_sequences = rng.gen_range(10..=100u32);
        total_ad_seq_count += num_sequences;
        let airdrop_id = i.to_string();
        client
            .execute(
                "INSERT INTO ticket_airdrop (wallet, epoch_index, num_sequences, airdrop_id) VALUES ($1, $2, $3, $4)",
                &[
                    &wallet.to_string(),
                    &Decimal::from(epoch_index),
                    &(num_sequences as i64),
                    &airdrop_id,
                ],
            )
            .await?;
    }
    let repo = get_repo().await;
    let actual_num_sequences = repo
        .num_airdrop_sequences_by_wallet_and_epoch_index(&wallet, epoch_index)
        .await?;
    assert_eq!(total_ad_seq_count, actual_num_sequences);
    Ok(())
}

#[tokio::test]
async fn test_prior_sequences_exist_by_wallet_and_epoch_index() -> Result<()> {
    let pool = common::setup().await;
    let client = get_client(&pool).await?;
    let repo = get_repo().await;

    let mut rng = thread_rng();
    let epoch_index = rng.gen_range(0..u64::max_value());
    let wallet = Pubkey::new_unique();

    client
        .execute("DELETE FROM sequences WHERE wallet = $1", &[&wallet.to_string()])
        .await?;

    let prior_sequences_exist = repo.prior_sequences_exist_by_wallet(&wallet).await?;
    // no sequences exist for this wallet, so prior sequences should not exist
    assert!(!prior_sequences_exist);
    let sequences = vec![Sequence {
        nums: [1, 2, 3, 4, 5, 6],
        sequence_type: SequenceType::Normal,
    }];
    let ticket = Ticket {
        wallet,
        epoch_index,
        sequences,
        ..create_ticket()
    };
    repo.create(&ticket).await?;
    let prior_sequences_exist = repo.prior_sequences_exist_by_wallet(&wallet).await?;
    assert!(prior_sequences_exist);

    Ok(())
}

#[tokio::test]
async fn test_draws_played_by_wallet() -> Result<()> {
    let pool = common::setup().await;
    let client = get_client(&pool).await?;
    let repo = get_repo().await;

    let wallet = Pubkey::new_unique();
    client
        .execute("DELETE FROM sequences WHERE wallet = $1", &[&wallet.to_string()])
        .await?;
    let draws_played = repo.draws_played_by_wallet(&wallet).await?;
    assert_eq!(draws_played, 0);

    let expected_draws_played = 10;
    for _ in 1..=expected_draws_played {
        let ticket = Ticket {
            wallet,
            sequences: vec![Sequence {
                nums: [1, 2, 3, 4, 5, 6],
                sequence_type: SequenceType::Normal,
            }],
            ..create_ticket()
        };
        repo.create(&ticket).await?;
    }
    let draws_played = repo.draws_played_by_wallet(&wallet).await?;
    assert_eq!(draws_played, expected_draws_played);

    Ok(())
}
