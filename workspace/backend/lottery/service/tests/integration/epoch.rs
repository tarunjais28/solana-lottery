use crate::common::{
    self,
    util::{deposit_yield, progress_latest_epoch_to_status, publish_winning_combination},
    SolanaContext,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use pretty_assertions::assert_eq;
use rand::{
    prelude::{IteratorRandom, StdRng},
    thread_rng, Rng, SeedableRng,
};
use service::{
    epoch::{service::EpochService, EpochManager, EpochRepository, FPUSDC},
    model::epoch::{Epoch, EpochStatus, Investor, UseCache},
    solana::{
        rpc::{SolanaRpc, SolanaRpcExt},
        Solana, ToSolanaError,
    },
    tickets::{
        bonus::BonusInfoService, DefaultTicketService, Sequence, SequenceType, Ticket, TicketPrice,
        TicketPriceCalculator, TicketRepository, TicketService,
    },
};
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_sdk::signer::Signer;
use std::sync::{Arc, Mutex, RwLock};
use utils::with_mutex;

use nezha_staking::accounts as ac;
use nezha_staking::fixed_point::test_utils::fp;
use service::tickets::InMemoryTicketRepository;

fn new_epoch_svc(solana: Box<dyn Solana>) -> EpochService {
    EpochService::new(
        solana,
        Box::new(InMemoryEpochRepository::new()),
        Box::new(InMemoryTicketRepository::new(0)),
    )
}

#[tokio::test]
async fn test_non_existing_index() -> Result<()> {
    let ctx = common::setup_solana().await;
    let solana = ctx.solana;

    let epoch = solana.get_epoch_by_index(9999999).await;

    match epoch {
        Err(err) => {
            assert!(err.is_account_not_found(nezha_staking::state::AccountType::Epoch));
        }
        Ok(epoch) => {
            assert!(false, "Epoch was not supposed to be found: {}", epoch.index);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_first_and_latest() -> Result<()> {
    let ctx = common::setup_solana().await;
    let solana = &ctx.solana;

    // make sure there's an epoch with some winners
    progress_latest_epoch_to_status(&ctx, EpochStatus::Running).await?;
    progress_latest_epoch_to_status(&ctx, EpochStatus::Ended).await?;

    let latest_epoch = solana.get_latest_epoch().await.expect("expected LatestEpoch to exist");

    assert!(latest_epoch.index >= 1);
    let epoch = solana
        .get_epoch_by_index(1)
        .await
        .expect("unable to find the first epoch");
    assert_eq!(1, epoch.index);

    Ok(())
}

#[tokio::test]
async fn test_latest_epoch_from_service() -> Result<()> {
    let ctx = common::setup_solana().await;
    let solana = &ctx.solana;

    // make sure there's at least 1 epoch
    progress_latest_epoch_to_status(&ctx, EpochStatus::Running).await?;

    let svc = new_epoch_svc(Box::new(solana.clone()));

    let latest_epoch = svc.latest_epoch(UseCache::No).await?.unwrap();
    assert!(latest_epoch.index >= 1);
    let epoch = solana
        .get_epoch_by_index(latest_epoch.index)
        .await
        .expect("unable to find the first epoch");
    assert_eq!(latest_epoch.index, epoch.index);

    Ok(())
}

#[tokio::test]
async fn test_prize_decoding() -> Result<()> {
    let ctx = common::setup_solana().await;
    let solana = &ctx.solana;

    // make sure there's an epoch with the required winner
    progress_latest_epoch_to_status(&ctx, EpochStatus::Running).await?;
    progress_latest_epoch_to_status(&ctx, EpochStatus::Ended).await?;

    let svc = new_epoch_svc(Box::new(solana.clone()));

    let prizes = svc.wallet_prizes(&ctx.user_keypair.pubkey()).await?;
    assert_eq!(false, prizes.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_withdraw_yield() -> Result<()> {
    let ctx = common::setup_solana().await;
    let solana = &ctx.solana;

    let investor_pubkey = ctx.investor_keypair.pubkey();

    let svc = new_epoch_svc(Box::new(solana.clone()));

    progress_latest_epoch_to_status(&ctx, EpochStatus::Running).await?;

    let latest_epoch = svc.latest_epoch(UseCache::No).await?.unwrap();
    assert_eq!(latest_epoch.status, EpochStatus::Running);

    let old_investor_balance = solana.get_usdc_balance_by_wallet(investor_pubkey).await?;

    let deposit_vault = ac::deposit_vault(&solana.program_id()).pubkey;
    let old_vault_balance = get_usdc_token_account_balance(solana.rpc_client.as_ref(), deposit_vault).await?;

    let epoch = svc.enter_investment(Investor::Fake).await?;
    assert_eq!(epoch.status, EpochStatus::Yielding);

    let new_investor_balance = solana.get_usdc_balance_by_wallet(investor_pubkey).await?;
    let new_vault_balance = get_usdc_token_account_balance(solana.rpc_client.as_ref(), deposit_vault).await?;
    assert_eq!(new_vault_balance, 0u8.into());
    assert_eq!(
        new_investor_balance,
        old_investor_balance.checked_add(old_vault_balance).unwrap()
    );

    let total_invested = epoch.total_invested;
    assert!(total_invested.is_some());
    let total_invested = total_invested.expect("total invested not set");
    assert_eq!(total_invested, old_vault_balance);
    assert_eq!(svc.latest_epoch(UseCache::Yes).await?, Some(epoch));

    Ok(())
}

#[tokio::test]
async fn test_deposit_yield() -> Result<()> {
    let ctx = common::setup_solana().await;
    let solana = &ctx.solana;

    let svc = new_epoch_svc(Box::new(solana.clone()));

    progress_latest_epoch_to_status(&ctx, EpochStatus::Running).await?;

    let mut rng = thread_rng();

    progress_latest_epoch_to_status(&ctx, EpochStatus::Yielding).await?;
    let latest_epoch = svc.latest_epoch(UseCache::No).await?.unwrap();
    assert_eq!(latest_epoch.status, EpochStatus::Yielding);

    let yield_range = 20.0..25.0;
    let yield_percent: f64 = rng.gen_range(yield_range);

    let return_amount = latest_epoch
        .total_invested
        .unwrap()
        .checked_mul(fp(1.0 + yield_percent / 100.0))
        .unwrap();

    svc.exit_investment(Investor::Fake, Some(return_amount)).await?;
    let latest_epoch_chain = {
        let latest_epoch = solana.get_latest_epoch().await?;
        let epoch = solana.get_epoch_by_index(latest_epoch.index).await?;
        Epoch::from_solana(&epoch, None)
    };
    let latest_epoch = svc.latest_epoch(UseCache::Yes).await?.unwrap();
    assert_eq!(latest_epoch, latest_epoch_chain);
    assert_eq!(latest_epoch.status, EpochStatus::Finalising);
    assert_eq!(latest_epoch.returns.unwrap().total, return_amount);

    Ok(())
}

// #[tokio::test]
// async fn test_publish_winning_combination() -> Result<()> {
//     let ctx = common::setup_solana().await;
//     let solana = &ctx.solana;

//     let svc = new_epoch_svc(Box::new(solana.clone()));

//     // start with a fresh epoch
//     progress_latest_epoch_to_status(&ctx, EpochStatus::Ended).await?;
//     progress_latest_epoch_to_status(&ctx, EpochStatus::Running).await?;
//     // epoch needs to be in Finalising for publishing winning combination
//     progress_latest_epoch_to_status(&ctx, EpochStatus::Finalising).await?;

//     svc.latest_epoch(UseCache::No).await?;

//     let winning_combination = [1, 2, 3, 4, 5, 6];
//     let epoch = svc.publish_winning_combination(winning_combination.clone()).await?;

//     assert!(epoch.winning_combination.is_some());
//     assert_eq!(epoch.winning_combination.unwrap(), winning_combination);
//     let latest_epoch_chain = {
//         let latest_epoch = solana.get_latest_epoch().await?;
//         let epoch = solana.get_epoch_by_index(latest_epoch.index).await?;
//         Epoch::from_solana(&epoch)
//     };
//     assert_eq!(svc.latest_epoch(UseCache::Yes).await?, Some(latest_epoch_chain));

//     Ok(())
// }

async fn setup_service_with_tier(tier: u8) -> (SolanaContext, Box<dyn TicketService>, Box<EpochService>) {
    let ctx = common::setup_solana().await;
    let solana = &ctx.solana;

    let user_pubkey = ctx.user_keypair.pubkey();
    let epoch_service = new_epoch_svc(Box::new(solana.clone()));

    let ticket_repo = Box::new(InMemoryTicketRepository::new(0));
    let rng = Arc::new(Mutex::new(StdRng::from_entropy()));

    // start with a fresh epoch
    progress_latest_epoch_to_status(&ctx, EpochStatus::Ended)
        .await
        .expect("could not progress epoch to ended");
    progress_latest_epoch_to_status(&ctx, EpochStatus::Running)
        .await
        .expect("could not progress epoch to running");
    let latest_epoch = epoch_service
        .latest_epoch(UseCache::No)
        .await
        .expect("could not read latest epoch")
        .unwrap();

    // generate only 1 ticket for the user for the latest epoch
    // this ensures we have control on the winners
    let ticket = Ticket {
        wallet: user_pubkey.clone(),
        epoch_index: latest_epoch.index,
        sequences: vec![Sequence {
            nums: [1, 2, 3, 4, 5, 6],
            sequence_type: SequenceType::Normal,
        }],
        ..Ticket::new_for_tests()
    };
    ticket_repo.create(&ticket).await.expect("could not create ticket");
    let ticket_service = DefaultTicketService::new(
        rng.clone(),
        Box::new(solana.clone()),
        ticket_repo,
        Box::new(MockTicketPriceCalculator {}),
        Box::new(MockBonusInfoService {}),
    );

    let mut winning_combination: [u8; 6] = ticket.sequences[0].nums;
    // change winning combination for tiers 2 & 3
    let mut rng = rng.lock().expect("Can't lock the rng");
    match tier {
        3 => {
            winning_combination[4] = (1..=56)
                .filter(|n| !winning_combination.contains(n))
                .choose(&mut *rng)
                .unwrap();
        }
        2 => {
            winning_combination[5] = (1..=10)
                .filter(|n| !winning_combination.contains(n))
                .choose(&mut *rng)
                .unwrap();
        }
        _ => {}
    };
    drop(rng);

    progress_latest_epoch_to_status(&ctx, EpochStatus::Yielding)
        .await
        .expect("could not progress epoch to yielding");
    let latest_epoch = {
        let latest_epoch = solana.get_latest_epoch().await.unwrap();
        let epoch = solana.get_epoch_by_index(latest_epoch.index).await.unwrap();
        Epoch::from_solana(&epoch, None)
    };
    let total_invested = latest_epoch.total_invested.unwrap();
    let yield_amount = "1.0".parse().unwrap();
    let return_amount = total_invested.checked_add(yield_amount).unwrap();
    deposit_yield(solana, latest_epoch.index, Some(return_amount))
        .await
        .expect("could not deposit yield");
    publish_winning_combination(solana, latest_epoch.index, winning_combination)
        .await
        .expect("could not publish winning combination");

    epoch_service.latest_epoch(UseCache::No).await.unwrap();
    (ctx, Box::new(ticket_service), Box::new(epoch_service))
}

#[tokio::test]
async fn test_publish_winners_tier_1() -> Result<()> {
    let (ctx, ticket_service, epoch_service) = setup_service_with_tier(1).await;
    let solana = ctx.solana;
    let user_pubkey = ctx.user_keypair.pubkey();

    let winners = ticket_service.calculate_winners().await?;
    let _epoch = epoch_service.publish_winners(winners).await?;

    let latest_epoch = {
        let latest_epoch = solana.get_latest_epoch().await?;
        let epoch = solana.get_epoch_by_index(latest_epoch.index).await?;
        Epoch::from_solana(&epoch, None)
    };
    let epoch_winners = epoch_service.read_epoch_prizes(latest_epoch.index).await?.unwrap();

    assert!(epoch_winners
        .winners
        .iter()
        .any(|winner| winner.tier == 1 && winner.address == user_pubkey));

    Ok(())
}

#[tokio::test]
async fn test_publish_winners_tier_2() -> Result<()> {
    let (ctx, ticket_service, epoch_service) = setup_service_with_tier(2).await;
    let solana = ctx.solana;
    let user_pubkey = ctx.user_keypair.pubkey();

    let winners = ticket_service.calculate_winners().await?;
    let _epoch = epoch_service.publish_winners(winners).await?;

    let latest_epoch = {
        let latest_epoch = solana.get_latest_epoch().await?;
        let epoch = solana.get_epoch_by_index(latest_epoch.index).await?;
        Epoch::from_solana(&epoch, None)
    };
    let epoch_winners = epoch_service.read_epoch_prizes(latest_epoch.index).await?.unwrap();

    assert!(epoch_winners
        .winners
        .iter()
        .any(|winner| winner.tier == 2 && winner.address == user_pubkey));

    Ok(())
}

#[tokio::test]
async fn test_multiple_winning_tickets() -> Result<()> {
    let ctx = common::setup_solana().await;
    let solana = &ctx.solana;

    let user_pubkey = ctx.user_keypair.pubkey();
    let epoch_service = new_epoch_svc(Box::new(solana.clone()));

    let ticket_repo = Box::new(InMemoryTicketRepository::new(0));
    let rng = Arc::new(Mutex::new(StdRng::from_entropy()));

    // start with a fresh epoch
    progress_latest_epoch_to_status(&ctx, EpochStatus::Ended).await?;
    progress_latest_epoch_to_status(&ctx, EpochStatus::Running).await?;
    let latest_epoch = {
        let latest_epoch = solana.get_latest_epoch().await?;
        let epoch = solana.get_epoch_by_index(latest_epoch.index).await?;
        Epoch::from_solana(&epoch, None)
    };

    let sequences = vec![
        // tier 2 winning sequences
        Sequence {
            nums: [1, 2, 3, 4, 5, 7],
            sequence_type: SequenceType::Normal,
        },
        Sequence {
            nums: [1, 2, 3, 4, 5, 8],
            sequence_type: SequenceType::Normal,
        },
        // tier 3 winning sequences
        Sequence {
            nums: [1, 2, 3, 4, 6, 7],
            sequence_type: SequenceType::Normal,
        },
        Sequence {
            nums: [1, 2, 3, 4, 6, 8],
            sequence_type: SequenceType::Normal,
        },
    ];
    let ticket = Ticket {
        wallet: user_pubkey.clone(),
        epoch_index: latest_epoch.index,
        arweave_url: None,
        sequences,
        balance: "1".to_string(),
        price: "1".to_string(),
        risq_id: None,
    };
    ticket_repo.create(&ticket).await?;

    // add more tier 2 winning tickets
    let n_winners: usize = with_mutex(&rng, |rng| rng.gen_range(1..=5));
    for _ in 0..n_winners {
        let ticket = Ticket {
            wallet: Pubkey::new_unique(),
            epoch_index: latest_epoch.index,
            arweave_url: None,
            sequences: vec![Sequence {
                nums: [1, 2, 3, 4, 5, 7],
                sequence_type: SequenceType::Normal,
            }],
            balance: "1".to_string(),
            price: "1".to_string(),
            risq_id: None,
        };
        ticket_repo.create(&ticket).await?;
    }
    let tier2_winning_tickets_count = (n_winners + 2) as u64;

    // add more tier 3 winning tickets
    let n_winners: usize = with_mutex(&rng, |rng| rng.gen_range(1..=5));
    for _ in 0..n_winners {
        let ticket = Ticket {
            wallet: Pubkey::new_unique(),
            epoch_index: latest_epoch.index,
            arweave_url: None,
            sequences: vec![Sequence {
                nums: [1, 2, 3, 4, 6, 7],
                sequence_type: SequenceType::Normal,
            }],
            balance: "1".to_string(),
            price: "1".to_string(),
            risq_id: None,
        };
        ticket_repo.create(&ticket).await?;
    }
    let tier3_winning_tickets_count = (n_winners + 2) as u64;

    let ticket_service = DefaultTicketService::new(
        rng.clone(),
        Box::new(solana.clone()),
        ticket_repo,
        Box::new(MockTicketPriceCalculator {}),
        Box::new(MockBonusInfoService {}),
    );

    let winning_combination: [u8; 6] = [1, 2, 3, 4, 5, 6];

    progress_latest_epoch_to_status(&ctx, EpochStatus::Yielding).await?;
    let latest_epoch = epoch_service.latest_epoch(UseCache::No).await?.unwrap();
    let total_invested = latest_epoch.total_invested.unwrap();
    let yield_amount = "1.0".parse().unwrap();
    let return_amount = total_invested.checked_add(yield_amount).unwrap();
    deposit_yield(solana, latest_epoch.index, Some(return_amount)).await?;
    publish_winning_combination(solana, latest_epoch.index, winning_combination).await?;

    let winners = ticket_service.calculate_winners().await?;
    let _epoch = epoch_service.publish_winners(winners).await?;
    let epoch_winners = epoch_service.read_epoch_prizes(latest_epoch.index).await?.unwrap();

    assert!(
        epoch_winners
            .winners
            .iter()
            .any(|winner| winner.tier == 2 && winner.address == user_pubkey),
        "User won tier2"
    );
    let expected_user_prize_amount = epoch_winners
        .tier2_meta
        .total_prize
        .checked_mul(2u8.into())
        .unwrap()
        .checked_div(tier2_winning_tickets_count.into())
        .unwrap();
    let actual_user_prize_amount = epoch_winners
        .winners
        .iter()
        .find(|winner| winner.address == user_pubkey && winner.tier == 2)
        .unwrap()
        .prize;
    assert_eq!(
        actual_user_prize_amount, expected_user_prize_amount,
        "Tier2 prize amount is correct"
    );

    assert!(
        epoch_winners.winners.iter().any(|winner| winner.address == user_pubkey),
        "User won tier3"
    );
    let expected_user_prize_amount = epoch_winners
        .tier3_meta
        .total_prize
        .checked_mul(2u8.into())
        .unwrap()
        .checked_div(tier3_winning_tickets_count.into())
        .unwrap();
    let actual_user_prize_amount = epoch_winners
        .winners
        .iter()
        .find(|winner| winner.address == user_pubkey && winner.tier == 3)
        .unwrap()
        .prize;
    assert_eq!(
        actual_user_prize_amount, expected_user_prize_amount,
        "Tier3 prize amount is correct"
    );
    Ok(())
}

struct MockTicketPriceCalculator {}

#[async_trait]
impl TicketPriceCalculator for MockTicketPriceCalculator {
    async fn calculate(&self, _balance: FPUSDC) -> TicketPrice {
        todo!()
    }
    async fn price(&self) -> FPUSDC {
        todo!()
    }
}

#[derive(Clone)]
struct InMemoryEpochRepository {
    epochs: Arc<RwLock<Vec<Epoch>>>,
}

impl InMemoryEpochRepository {
    pub fn new() -> Self {
        Self {
            epochs: Arc::new(RwLock::new(vec![])),
        }
    }
}

#[async_trait]
impl EpochRepository for InMemoryEpochRepository {
    async fn by_index(&self, index: u64) -> Result<Option<Epoch>> {
        let epochs = self.epochs.read().map_err(|_| anyhow!("cannot read epochs"))?;
        Ok(epochs.iter().find(|epoch| epoch.index == index).cloned())
    }

    async fn by_pubkey(&self, pubkey: &Pubkey) -> Result<Option<Epoch>> {
        let epochs = self.epochs.read().map_err(|_| anyhow!("cannot read epochs"))?;
        Ok(epochs.iter().find(|epoch| epoch.pubkey == *pubkey).cloned())
    }

    async fn all(&self) -> Result<Vec<Epoch>> {
        let epochs = self.epochs.read().map_err(|_| anyhow!("cannot read epochs"))?;
        Ok(epochs.clone())
    }

    async fn latest_epoch(&self) -> Result<Option<Epoch>> {
        let epochs = self.epochs.read().map_err(|_| anyhow!("cannot read epochs"))?;
        Ok(epochs.iter().max_by_key(|epoch| epoch.index).cloned())
    }

    async fn create_or_update_epoch(&self, epoch: &Epoch) -> Result<Epoch> {
        let mut epochs = self.epochs.write().map_err(|_| anyhow!("cannot write epochs"))?;
        let e = epochs.iter_mut().find(|e| e.index == epoch.index);
        match e {
            Some(e) => *e = epoch.clone(),
            None => epochs.push(epoch.clone()),
        };
        Ok(epoch.clone())
    }
}

struct MockBonusInfoService {}

#[async_trait]
impl BonusInfoService for MockBonusInfoService {
    async fn min_stake_amount(&self) -> FPUSDC {
        todo!()
    }

    async fn num_signup_bonus_sequences(&self, _normal_sequence_count: u32) -> Result<u32> {
        todo!()
    }
}

async fn get_usdc_token_account_balance(rpc: &dyn SolanaRpc, token_account: Pubkey) -> Result<FPUSDC> {
    let account = rpc.get_account(&token_account).await?;
    match account {
        Some(account) => {
            let account = spl_token::state::Account::unpack(&account.data)
                .with_context(|| format!("Failed to decode USDC Token Account {}", account.pubkey))?;
            Ok(FPUSDC::from_usdc(account.amount))
        }
        None => Ok(FPUSDC::from_usdc(0)),
    }
}
