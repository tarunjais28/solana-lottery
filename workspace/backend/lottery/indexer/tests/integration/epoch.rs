use anyhow::Result;
use async_trait::async_trait;
use indexer::{
    indexer::epoch::{
        artkai::ArtkaiUpdater, jackpot_funded, rng::SequenceGenerator, EpochCommand, EpochIndexer, EpochJobScheduler,
        EpochJobSchedulerConfig, WinningCombinationSource,
    },
    nezha_api::{DrawEnabled, EpochStatus, Investor, TieredPrizes, YieldSplitCfg},
};
use rand::thread_rng;
use solana_sdk::signer::{keypair::Keypair, Signer};
use spl_token::ui_amount_to_amount;

use crate::common::setup_api;
use crate::common::{
    self,
    util::{airdrop_sol, airdrop_usdc, attempt_deposit},
};

struct MockArtkaiClient;

#[async_trait]
impl ArtkaiUpdater for MockArtkaiClient {
    async fn finish_epoch(&self, _epoch_id: u64) -> Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_epoch_indexer() -> Result<()> {
    let epoch_length_seconds = 60;
    let config = EpochJobSchedulerConfig {
        start_schedule_string: format!("0/{epoch_length_seconds} * * * * *"),
        enter_investment_offset_seconds: 10,
        exit_investment_offset_seconds: 20,
        publish_winning_combination_offset_seconds: 30,
        publish_winners_offset_seconds: 40,
    };
    let scheduler = EpochJobScheduler::try_from(config).unwrap();
    let nezha_api = setup_api();
    let context = common::setup_context();
    let prizes = TieredPrizes {
        tier1: "1000".into(),
        tier2_yield_share: 7,
        tier3_yield_share: 3,
    };
    let yield_split_cfg = YieldSplitCfg {
        insurance_premium: "2.0".into(),
        insurance_jackpot: "1000".into(),
        insurance_probability: "0.0001".into(),
        treasury_ratio: "0.5".into(),
    };

    let yield_range = 0.0..5.0;

    let artkai_client = Box::new(MockArtkaiClient);

    let sequence_generator = SequenceGenerator::new(thread_rng());

    let mut indexer = EpochIndexer::new(
        scheduler,
        nezha_api,
        context.clone(),
        prizes,
        yield_split_cfg,
        yield_range,
        artkai_client,
        sequence_generator,
        WinningCombinationSource::Random,
        Investor::Fake,
        service::solana::SwitchboardConfiguration::Fake,
    );

    let previous_epoch = indexer.nezha_api.get_latest_epoch().await.unwrap();

    // run a whole epoch cycle
    loop {
        let job = indexer.next_job().await.unwrap();

        indexer.run_job(&job).await.unwrap();

        if let EpochCommand::CreateEpoch = job.command {
            let user_keypair = Keypair::new();
            let user_pubkey = user_keypair.pubkey();
            airdrop_sol(&context.rpc_client, &user_pubkey).await?;
            airdrop_usdc(&context, &user_pubkey, ui_amount_to_amount(25.0, 6)).await?;
            attempt_deposit(&context.clone(), &user_keypair, ui_amount_to_amount(25.0, 6)).await?;
            indexer.nezha_api.approve_stake_update(&user_pubkey).await?;
        }

        let latest_epoch = indexer
            .nezha_api
            .get_latest_epoch()
            .await
            .unwrap()
            .expect("there should be at least 1 epoch");
        match job.command {
            EpochCommand::CreateEpoch => assert_eq!(latest_epoch.status, EpochStatus::Running),
            EpochCommand::EnterInvestment => assert_eq!(latest_epoch.status, EpochStatus::Yielding),
            EpochCommand::ExitInvestment => assert_eq!(latest_epoch.status, EpochStatus::Finalising),
            EpochCommand::PublishWinningCombination => {
                assert_eq!(latest_epoch.status, EpochStatus::Finalising);
                assert!(latest_epoch.winning_combination.is_some());
            }
            EpochCommand::PublishWinners => assert_eq!(latest_epoch.status, EpochStatus::Ended),
            EpochCommand::FundJackpot => {
                assert_eq!(latest_epoch.status, EpochStatus::Ended);
                assert!(jackpot_funded(&latest_epoch.winners));
            }
        }
        match &previous_epoch {
            Some(previous_epoch) => {
                if previous_epoch.status == latest_epoch.status {
                    break;
                }
            }
            None => {
                if latest_epoch.status == EpochStatus::Ended {
                    break;
                }
            }
        }
    }

    let latest_epoch = indexer
        .nezha_api
        .get_latest_epoch()
        .await?
        .expect("there should be at least 1 epoch");
    if let Some(epoch) = previous_epoch {
        assert_eq!(latest_epoch.index, epoch.index + 1);
    } else {
        assert_eq!(latest_epoch.index, 1);
    }

    Ok(())
}

#[tokio::test]
async fn test_fund_prize() -> Result<()> {
    let config = EpochJobSchedulerConfig {
        start_schedule_string: format!("0 0/2 * * * *"),
        enter_investment_offset_seconds: 10,
        exit_investment_offset_seconds: 20,
        publish_winning_combination_offset_seconds: 30,
        publish_winners_offset_seconds: 40,
    };
    let scheduler = EpochJobScheduler::try_from(config)?;
    let nezha_api = setup_api();
    let context = common::setup_context();
    let prizes = TieredPrizes {
        tier1: "1000".into(),
        tier2_yield_share: 7,
        tier3_yield_share: 3,
    };
    let yield_split_cfg = YieldSplitCfg {
        insurance_premium: "2.0".into(),
        insurance_jackpot: "1000".into(),
        insurance_probability: "0.0001".into(),
        treasury_ratio: "0.5".into(),
    };

    let yield_range = 1000.0..1100.0;

    let artkai_client = Box::new(MockArtkaiClient);

    let sequence_generator = SequenceGenerator::new(thread_rng());

    let mut indexer = EpochIndexer::new(
        scheduler,
        nezha_api,
        context.clone(),
        prizes,
        yield_split_cfg,
        yield_range,
        artkai_client,
        sequence_generator,
        WinningCombinationSource::Random,
        Investor::Fake,
        service::solana::SwitchboardConfiguration::Fake,
    );

    let user_keypair = Keypair::new();
    let user_pubkey = user_keypair.pubkey();

    loop {
        let job = indexer.next_job().await?;
        if let EpochCommand::EnterInvestment = job.command {
            break;
        } else {
            indexer.run_job(&job).await?;
        }
    }

    airdrop_sol(&context.rpc_client, &user_pubkey).await?;
    airdrop_usdc(&context, &user_pubkey, ui_amount_to_amount(25.0, 6)).await?;
    attempt_deposit(&context.clone(), &user_keypair, ui_amount_to_amount(25.0, 6)).await?;

    indexer.nezha_api.approve_stake_update(&user_pubkey).await?;
    indexer.nezha_api.complete_stake_update(&user_pubkey).await?;

    let sequences = indexer.nezha_api.generate_ticket(&user_pubkey).await?;
    let winning_combination = sequences.first().unwrap();

    indexer.nezha_api.enter_investment(indexer.investor).await?;
    indexer.exit_investment().await?;

    indexer
        .nezha_api
        .publish_winning_combination(winning_combination.nums)
        .await?;
    indexer.nezha_api.publish_winners().await?;

    indexer.fund_jackpot().await?;

    let epoch = indexer
        .nezha_api
        .get_latest_epoch()
        .await?
        .expect("there should be at least 1 epoch");
    assert_eq!(epoch.draw_enabled, DrawEnabled::Draw);
    let winners = epoch.winners.expect("there should be winners");
    assert!(winners.jackpot_claimable);
    assert_eq!(winners.winners, vec![user_pubkey]);

    Ok(())
}
