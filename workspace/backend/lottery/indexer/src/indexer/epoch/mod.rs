use std::{ops::Range, str::FromStr, sync::Arc};

use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use cron::Schedule;
use log::info;
use nezha_staking::{
    fixed_point::{self, test_utils::fp},
    instruction,
};
use rand::Rng;
use service::solana::SwitchboardConfiguration;
use solana_sdk::signer::Signer;
use spl_associated_token_account::get_associated_token_address;
use thiserror::Error;

use crate::{
    indexer::{
        epoch::artkai::ArtkaiUpdater,
        util::{send_and_confirm_transaction, SolanaProgramContext},
    },
    nezha_api::{DrawEnabled, Epoch, EpochStatus, EpochWinners, Investor, NezhaAPI, TieredPrizes, YieldSplitCfg},
};

use self::rng::SequenceGenerator;

pub mod artkai;
pub mod rng;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EpochCommand {
    CreateEpoch,
    EnterInvestment,
    ExitInvestment,
    PublishWinningCombination,
    PublishWinners,
    FundJackpot,
}

pub trait NextCommand {
    fn next_command(&self) -> EpochCommand;
}

impl NextCommand for Option<Epoch> {
    fn next_command(&self) -> EpochCommand {
        match self {
            None => EpochCommand::CreateEpoch,
            Some(epoch) => match epoch.status {
                EpochStatus::Running => EpochCommand::EnterInvestment,
                EpochStatus::Yielding => EpochCommand::ExitInvestment,
                EpochStatus::Finalising => match epoch.winning_combination {
                    None => EpochCommand::PublishWinningCombination,
                    Some(_) => EpochCommand::PublishWinners,
                },
                EpochStatus::Ended => {
                    if matches!(epoch.draw_enabled, DrawEnabled::NoDraw) || jackpot_funded(&epoch.winners) {
                        EpochCommand::CreateEpoch
                    } else {
                        EpochCommand::FundJackpot
                    }
                }
            },
        }
    }
}

pub fn jackpot_funded(epoch_winners: &Option<EpochWinners>) -> bool {
    match &epoch_winners {
        Some(winners) => winners.jackpot_claimable || winners.tier1_meta.total_num_winners == 0,
        None => false,
    }
}

#[derive(Debug)]
pub struct EpochJob {
    pub command: EpochCommand,
    pub due_time: DateTime<Utc>,
}

pub struct EpochJobSchedulerConfig {
    pub start_schedule_string: String,
    pub enter_investment_offset_seconds: i64,
    pub exit_investment_offset_seconds: i64,
    pub publish_winning_combination_offset_seconds: i64,
    pub publish_winners_offset_seconds: i64,
}

pub struct EpochJobScheduler {
    /// the schedule for starting a new epoch
    pub start_schedule: Schedule,
    /// the duration offset from epoch start to withdrawing the yield to the investor
    pub enter_investment_offset: Duration,
    /// the duration offset from epoch start to finalising the epoch
    pub exit_investment_offset: Duration,
    /// the duration offset from epoch start to publishing the winning combination
    pub publish_winning_combination_offset: Duration,
    /// the duration offset from epoch start to publishing the winners
    pub publish_winners_offset: Duration,
    /// the duration offset from epoch start to funding the prizes
    pub fund_prizes_offset: Duration,
}

impl EpochJobScheduler {
    pub fn due_time(&self, command: &EpochCommand) -> Result<DateTime<Utc>> {
        let mut epoch_start_iterator = self.start_schedule.upcoming(Utc);
        let next_epoch_start_time = epoch_start_iterator.next().ok_or(EpochIndexerError::NoUpcomingEpoch)?;
        let this_epoch_start_time = epoch_start_iterator.next_back().ok_or(EpochIndexerError::NoPastEpoch)?;

        // if Utc::now() is later than fund prizes time, and we need to create a new epoch,
        // then we create an epoch in the next cycle
        // otherwise, everything else is done in the current cycle
        // the reason for this is that if an epoch creation is due when the current epoch is supposed to have ended,
        // then we may as well create a new epoch in the next cycle
        let due_time = match command {
            EpochCommand::CreateEpoch => {
                if Utc::now() > this_epoch_start_time + self.fund_prizes_offset {
                    next_epoch_start_time
                } else {
                    this_epoch_start_time
                }
            }
            EpochCommand::EnterInvestment => this_epoch_start_time + self.enter_investment_offset,
            EpochCommand::ExitInvestment => this_epoch_start_time + self.exit_investment_offset,
            EpochCommand::PublishWinningCombination => this_epoch_start_time + self.publish_winning_combination_offset,
            EpochCommand::PublishWinners => this_epoch_start_time + self.publish_winners_offset,
            EpochCommand::FundJackpot => this_epoch_start_time + self.fund_prizes_offset,
        };

        Ok(due_time)
    }
}

impl TryFrom<EpochJobSchedulerConfig> for EpochJobScheduler {
    type Error = anyhow::Error;

    fn try_from(config: EpochJobSchedulerConfig) -> Result<Self> {
        let start_schedule = Schedule::from_str(&config.start_schedule_string)?;
        if !(config.enter_investment_offset_seconds <= config.exit_investment_offset_seconds
            && config.exit_investment_offset_seconds <= config.publish_winning_combination_offset_seconds
            && config.publish_winning_combination_offset_seconds <= config.publish_winners_offset_seconds)
        {
            return Err(EpochIndexerError::OrderOfOperationsIncorrect.into());
        }
        let enter_investment_offset = Duration::seconds(config.enter_investment_offset_seconds);
        let exit_investment_offset = Duration::seconds(config.exit_investment_offset_seconds);
        let publish_winning_combination_offset = Duration::seconds(config.publish_winning_combination_offset_seconds);
        let publish_winners_offset = Duration::seconds(config.publish_winners_offset_seconds);
        let fund_prizes_offset = Duration::seconds(config.publish_winners_offset_seconds);
        Ok(EpochJobScheduler {
            start_schedule,
            enter_investment_offset,
            exit_investment_offset,
            publish_winning_combination_offset,
            publish_winners_offset,
            fund_prizes_offset,
        })
    }
}

#[derive(Error, Debug)]
pub enum EpochIndexerError {
    #[error("No upcoming epochs")]
    NoUpcomingEpoch,

    #[error("No past epochs")]
    NoPastEpoch,

    #[error("Epoch length is too short. Expected at least {expected} minutes, actual length {actual} minutes")]
    EpochLengthTooShort { expected: i64, actual: i64 },

    #[error("Order of operations is not correct")]
    OrderOfOperationsIncorrect,

    #[error("Could not read latest epoch")]
    CouldNotReadLatestEpoch,

    #[error("Total value locked not set, withdraw yield has to be run first")]
    TotalValueLockedNotSet,

    #[error("Winners not set, publish winners has to be run first")]
    WinnersNotSet,

    #[error("Numerical Overflow")]
    Overflow,
}

#[derive(Debug)]
pub enum WinningCombinationSource {
    GuaranteedJackpot,
    Optimal,
    Random,
}

pub struct EpochIndexer<T: Rng> {
    pub scheduler: EpochJobScheduler,
    pub nezha_api: Box<dyn NezhaAPI + Send + Sync>,
    pub context: Arc<SolanaProgramContext>,
    pub prizes: TieredPrizes,
    pub yield_split_cfg: YieldSplitCfg,
    pub yield_range: Range<f64>,
    pub artkai_client: Box<dyn ArtkaiUpdater + Send + Sync>,
    pub sequence_generator: SequenceGenerator<T>,
    pub winning_combination_source: WinningCombinationSource,
    pub investor: Investor,
    pub switchboard_cfg: SwitchboardConfiguration,
}

impl<T: Rng> EpochIndexer<T> {
    pub fn new(
        scheduler: EpochJobScheduler,
        nezha_api: Box<dyn NezhaAPI + Send + Sync>,
        context: Arc<SolanaProgramContext>,
        prizes: TieredPrizes,
        yield_split_cfg: YieldSplitCfg,
        yield_range: Range<f64>,
        artkai_client: Box<dyn ArtkaiUpdater + Send + Sync>,
        sequence_generator: SequenceGenerator<T>,
        winning_combination_source: WinningCombinationSource,
        investor: Investor,
        switchboard_cfg: SwitchboardConfiguration,
    ) -> Self {
        Self {
            scheduler,
            nezha_api,
            context,
            prizes,
            yield_split_cfg,
            yield_range,
            artkai_client,
            sequence_generator,
            winning_combination_source,
            investor,
            switchboard_cfg,
        }
    }

    pub async fn run_loop(&mut self) -> Result<()> {
        loop {
            let job = self.next_job().await?;
            self.wait_for_job(&job).await?;
            if let Err(e) = self.run_job(&job).await {
                log::error!("Error running job ({:?}): {}", job, e);
            }
        }
    }

    pub async fn next_job(&self) -> Result<EpochJob> {
        let latest_epoch = self.nezha_api.get_latest_epoch().await?;
        match &latest_epoch {
            None => log::info!("No epochs yet"),
            Some(epoch) => log::info!("Latest epoch: {epoch:?}"),
        }
        let command = latest_epoch.next_command();
        log::info!("Next command: {command:?}");
        let due_time = self.scheduler.due_time(&command)?;
        Ok(EpochJob { command, due_time })
    }

    pub async fn wait_for_job(&self, job: &EpochJob) -> Result<()> {
        let now = Utc::now();
        // sleep until the job is due
        if job.due_time > now {
            let sleep_duration = job.due_time.signed_duration_since(now);
            log::info!("Sleeping for {} seconds", sleep_duration.num_seconds());
            tokio::time::sleep(sleep_duration.to_std()?).await;
        }
        Ok(())
    }

    pub async fn run_job(&mut self, job: &EpochJob) -> Result<()> {
        match job.command {
            EpochCommand::CreateEpoch => {
                // create epoch
                log::info!("Creating epoch");
                self.create_epoch().await?;
            }
            EpochCommand::EnterInvestment => {
                // withdraw yield
                log::info!("Entering investment");
                self.nezha_api.enter_investment(self.investor).await?;
            }
            EpochCommand::ExitInvestment => {
                // deposit yield
                log::info!("Exiting investment");
                self.exit_investment().await?;
            }
            EpochCommand::PublishWinningCombination => {
                // publish winning combination
                log::info!("Publishing winning combination");
                self.publish_winning_combination().await?;
            }
            EpochCommand::PublishWinners => {
                // publish winners
                log::info!("Publishing winners");
                self.nezha_api.publish_winners().await?;

                // update Artkai
                log::info!("Updating Artkai");
                self.artkai_finish_epoch().await?;
            }
            EpochCommand::FundJackpot => {
                // fund epoch winner prizes
                log::info!("Funding epoch winner prizes");
                self.fund_jackpot().await?;
            }
        }
        Ok(())
    }

    pub async fn create_epoch(&self) -> Result<()> {
        // TODO: check for minimum epoch length
        // validate epoch length
        let mut start_schedule_iter = self.scheduler.start_schedule.upcoming(Utc);
        let next_epoch_start = start_schedule_iter.next().ok_or(EpochIndexerError::NoUpcomingEpoch)?;
        let this_epoch_start = start_schedule_iter.next_back().ok_or(EpochIndexerError::NoPastEpoch)?;
        let epoch_length = next_epoch_start - this_epoch_start;
        if epoch_length < self.scheduler.publish_winners_offset {
            return Err(EpochIndexerError::EpochLengthTooShort {
                expected: self.scheduler.publish_winners_offset.num_minutes(),
                actual: epoch_length.num_minutes(),
            }
            .into());
        }
        // if we have lost time in this epoch, the epoch duration should be adjusted
        let expected_duration_minutes = (self.scheduler.publish_winners_offset
            - Utc::now().signed_duration_since(this_epoch_start))
        .num_minutes()
        .max(0) as u32;

        let expected_duration_minutes = expected_duration_minutes + 3; // Add 3 min delay to
                                                                       // account for the time it takes to confirm the final transaction
        self.nezha_api
            .create_epoch(
                self.prizes.clone(),
                expected_duration_minutes,
                self.yield_split_cfg.clone(),
            )
            .await?;
        Ok(())
    }

    pub async fn exit_investment(&self) -> Result<()> {
        if let Investor::Fake = self.investor {
            let mut rng = rand::thread_rng();
            let yield_percent = rng.gen_range(self.yield_range.clone());

            let total_invested = self
                .nezha_api
                .get_latest_epoch()
                .await?
                .ok_or_else(|| EpochIndexerError::CouldNotReadLatestEpoch)?
                .total_value_locked
                .ok_or_else(|| EpochIndexerError::TotalValueLockedNotSet)?;

            let return_amount = total_invested
                .checked_mul(
                    // using test_utils is fine because this is fake investor used for testing
                    fixed_point::test_utils::fp(1.0 + yield_percent / 100.0),
                )
                .ok_or_else(|| EpochIndexerError::Overflow)?;

            log::info!("Yield percent: {}, Return amount: {}", yield_percent, return_amount);
            self.nezha_api
                .exit_investment(self.investor, Some(return_amount))
                .await?;
        } else {
            self.nezha_api.exit_investment(self.investor, None).await?;
        }

        Ok(())
    }

    pub async fn publish_winning_combination(&mut self) -> Result<()> {
        match self.switchboard_cfg {
            SwitchboardConfiguration::Fake => {
                let winning_combination = match self.winning_combination_source {
                    // These two need to read from the GraphQL API because they need to be calculated
                    WinningCombinationSource::GuaranteedJackpot => self.nezha_api.random_winning_combination().await?,
                    WinningCombinationSource::Optimal => self.nezha_api.calculate_optimal_winning_combination().await?,

                    // This one can be generated from within the indexer
                    WinningCombinationSource::Random => Some(self.sequence_generator.generate_sequence()),
                };
                let winning_combination = match winning_combination {
                    Some(winning_combination) => winning_combination,
                    None => self.sequence_generator.generate_sequence(),
                };
                self.nezha_api.publish_winning_combination(winning_combination).await?;
            }
            SwitchboardConfiguration::Devnet | SwitchboardConfiguration::Mainnet => {
                info!("Requesting combination from VRF");
                self.nezha_api.publish_winning_combination([0; 6]).await?;
            }
        }

        Ok(())
    }

    pub async fn artkai_finish_epoch(&self) -> Result<()> {
        let latest_epoch = self
            .nezha_api
            .get_latest_epoch()
            .await?
            .ok_or(EpochIndexerError::CouldNotReadLatestEpoch)?;

        let epoch_index = latest_epoch.index;

        self.artkai_client.finish_epoch(epoch_index).await?;

        Ok(())
    }

    pub async fn fund_jackpot(&self) -> Result<()> {
        let latest_epoch = self
            .nezha_api
            .get_latest_epoch()
            .await?
            .ok_or(EpochIndexerError::CouldNotReadLatestEpoch)?;
        if jackpot_funded(&latest_epoch.winners) {
            return Ok(());
        }
        let epoch_index = latest_epoch.index;
        let epoch_winners = latest_epoch.winners.clone().ok_or(EpochIndexerError::WinnersNotSet)?;
        let amount = epoch_winners.tier1_meta.total_prize;
        let admin_pubkey = self.context.admin_keypair.pubkey();
        let admin_ata_pubkey = get_associated_token_address(&admin_pubkey, &self.context.usdc_mint_pubkey);
        let mint_instruction = spl_token::instruction::mint_to(
            &spl_token::id(),
            &self.context.usdc_mint_pubkey,
            &admin_ata_pubkey,
            &admin_pubkey,
            &[&admin_pubkey],
            amount.as_usdc(),
        )?;
        send_and_confirm_transaction(
            &self.context.rpc_client,
            mint_instruction,
            &self.context.admin_keypair,
            &admin_pubkey,
        )
        .await?;

        let funding_instruction = instruction::fund_jackpot(
            &self.context.staking_program_id,
            &admin_pubkey,
            &admin_ata_pubkey,
            epoch_index,
        );
        send_and_confirm_transaction(
            &self.context.rpc_client,
            funding_instruction,
            &self.context.admin_keypair,
            &admin_pubkey,
        )
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use rand::thread_rng;
    use solana_client::nonblocking::rpc_client::RpcClient;
    use solana_sdk::{pubkey::Pubkey, signature::Keypair};

    use crate::mocks::NezhaApiImpl;

    use super::*;

    struct MockArtkaiClient;

    #[async_trait]
    impl ArtkaiUpdater for MockArtkaiClient {
        async fn finish_epoch(&self, _epoch_index: u64) -> Result<()> {
            todo!()
        }
    }

    #[tokio::test]
    async fn test_scheduler() -> Result<()> {
        let epoch_length_seconds = 6;
        let config = EpochJobSchedulerConfig {
            start_schedule_string: format!("0/{epoch_length_seconds} * * * * *"),
            enter_investment_offset_seconds: 1,
            exit_investment_offset_seconds: 2,
            publish_winning_combination_offset_seconds: 3,
            publish_winners_offset_seconds: 4,
        };

        let scheduler = EpochJobScheduler::try_from(config)?;

        // sleep until the middle of the next epoch cycle
        let mut iter = scheduler.start_schedule.upcoming(Utc);
        let next_epoch_start = iter.next().unwrap();
        let epoch_mid = next_epoch_start + scheduler.enter_investment_offset;
        tokio::time::sleep(epoch_mid.signed_duration_since(Utc::now()).to_std()?).await;

        // check for epoch starts in the middle of a cycle
        let start_time = scheduler.due_time(&EpochCommand::CreateEpoch)?;
        let now = Utc::now();
        assert!(start_time < now);

        // jobs should be due in the current epoch
        let withdraw_yield_time = scheduler.due_time(&EpochCommand::EnterInvestment)?;
        let deposit_yield_time = scheduler.due_time(&EpochCommand::ExitInvestment)?;
        let publish_winning_combination_time = scheduler.due_time(&EpochCommand::PublishWinningCombination)?;
        let publish_winners_time = scheduler.due_time(&EpochCommand::PublishWinners)?;
        let fund_jackpot_time = scheduler.due_time(&EpochCommand::FundJackpot)?;

        assert_eq!(withdraw_yield_time, start_time + scheduler.enter_investment_offset);
        assert_eq!(deposit_yield_time, start_time + scheduler.exit_investment_offset);
        assert_eq!(
            publish_winning_combination_time,
            start_time + scheduler.publish_winning_combination_offset
        );
        assert_eq!(publish_winners_time, start_time + scheduler.publish_winners_offset);
        assert_eq!(fund_jackpot_time, start_time + scheduler.fund_prizes_offset);

        // sleep until the end of the epoch cycle
        let epoch_end = next_epoch_start + scheduler.publish_winners_offset;
        tokio::time::sleep(epoch_end.signed_duration_since(Utc::now()).to_std()?).await;

        // epoch start at the end of a cycle should be due in the next epoch
        let start_time = scheduler.due_time(&EpochCommand::CreateEpoch)?;
        assert!(start_time > now);

        Ok(())
    }

    #[tokio::test]
    async fn test_wait_for_next_job() -> Result<()> {
        let epoch_length_seconds = 6;
        let start_schedule = Schedule::from_str(&format!("0/{epoch_length_seconds} * * * * *"))?;
        let enter_investment_offset = Duration::seconds(1);
        let exit_investment_offset = Duration::seconds(2);
        let publish_winning_combination_offset = Duration::seconds(3);
        let publish_winners_offset = Duration::seconds(4);
        let fund_prizes_offset = Duration::seconds(5);
        let scheduler = EpochJobScheduler {
            start_schedule: start_schedule.clone(),
            enter_investment_offset: enter_investment_offset.clone(),
            exit_investment_offset: exit_investment_offset.clone(),
            publish_winning_combination_offset: publish_winning_combination_offset.clone(),
            publish_winners_offset: publish_winners_offset.clone(),
            fund_prizes_offset: fund_prizes_offset.clone(),
        };

        let nezha_api = Box::new(NezhaApiImpl::new());

        let context = Arc::new(SolanaProgramContext::new(
            Arc::new(RpcClient::new("".to_string())),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Arc::new(Keypair::new()),
            Arc::new(Keypair::new()),
        ));

        let prizes = TieredPrizes {
            tier1: "1000".into(),
            tier2_yield_share: 7,
            tier3_yield_share: 3,
        };

        let yield_split_cfg = YieldSplitCfg {
            insurance_premium: "10".into(),
            insurance_jackpot: "1000".into(),
            insurance_probability: "0.1".into(),
            treasury_ratio: "0.5".into(),
        };

        let yield_range = 0.0..5.0;
        let artkai_client = Box::new(MockArtkaiClient);
        let sequence_generator = SequenceGenerator::new(thread_rng());
        let indexer = EpochIndexer::new(
            scheduler,
            nezha_api,
            context,
            prizes,
            yield_split_cfg,
            yield_range,
            artkai_client,
            sequence_generator,
            WinningCombinationSource::Random,
            Investor::Fake,
            SwitchboardConfiguration::Fake,
        );

        let start_time = start_schedule.upcoming(Utc).next().unwrap();
        let jobs = vec![
            EpochJob {
                command: EpochCommand::CreateEpoch,
                due_time: start_time,
            },
            EpochJob {
                command: EpochCommand::EnterInvestment,
                due_time: start_time + enter_investment_offset,
            },
            EpochJob {
                command: EpochCommand::ExitInvestment,
                due_time: start_time + exit_investment_offset,
            },
            EpochJob {
                command: EpochCommand::PublishWinningCombination,
                due_time: start_time + publish_winning_combination_offset,
            },
            EpochJob {
                command: EpochCommand::PublishWinners,
                due_time: start_time + publish_winners_offset,
            },
            EpochJob {
                command: EpochCommand::FundJackpot,
                due_time: start_time + fund_prizes_offset,
            },
        ];
        for job in jobs {
            let res = tokio::time::timeout(
                Duration::seconds(epoch_length_seconds).to_std()?,
                indexer.wait_for_job(&job),
            )
            .await?;
            assert!(res.is_ok(), "{res:?}");

            let now = Utc::now();
            let due_time = job.due_time;
            assert!(now >= due_time, "{now:?} {due_time:?}");
        }
        Ok(())
    }
}
