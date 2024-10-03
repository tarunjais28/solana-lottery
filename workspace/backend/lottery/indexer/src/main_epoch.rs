use std::{str::FromStr, sync::Arc};

use anyhow::{anyhow, Result};
use envconfig::Envconfig;
use indexer::{
    indexer::{
        epoch::{
            artkai::{ArtkaiClient, ArtkaiUpdater, FakeArtkaiClient},
            rng::SequenceGenerator,
            EpochIndexer, EpochJobScheduler, EpochJobSchedulerConfig, WinningCombinationSource,
        },
        util::SolanaProgramContext,
    },
    nezha_api::{self, Investor, TieredPrizes, YieldSplitCfg},
};
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use service::solana::SwitchboardConfiguration;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::keypair::read_keypair};

#[derive(Envconfig, Clone)]
pub struct AppConfig {
    #[envconfig(from = "INDEXER_NEZHA_GRAPHQL_URL")]
    pub nezha_graphql_url: String,

    #[envconfig(from = "INDEXER_EPOCH_START_SCHEDULE")]
    pub epoch_start_schedule: String,

    #[envconfig(from = "INDEXER_EPOCH_ENTER_INVESTMENT_OFFSET_SECONDS")]
    pub epoch_enter_investment_offset_seconds: i64,

    #[envconfig(from = "INDEXER_EPOCH_EXIT_INVESTMENT_OFFSET_SECONDS")]
    pub epoch_exit_investment_offset_seconds: i64,

    #[envconfig(from = "INDEXER_EPOCH_PUBLISH_WINNING_COMBINATION_OFFSET_SECONDS")]
    pub epoch_publish_winning_combination_offset_seconds: i64,

    #[envconfig(from = "INDEXER_EPOCH_PUBLISH_WINNERS_OFFSET_SECONDS")]
    pub epoch_publish_winners_offset_seconds: i64,

    #[envconfig(from = "SOLANA_HTTP_RPC_URL", default = "https://api.mainnet-beta.solana.com")]
    pub solana_http_rpc_url: String,

    #[envconfig(from = "SOLANA_ADMIN_KEYPAIR")]
    pub admin_keypair: String,

    #[envconfig(from = "SOLANA_STAKING_PROGRAM_ID")]
    pub solana_staking_program_id: String,

    #[envconfig(from = "SOLANA_USDC_MINT")]
    pub usdc_mint: String,

    #[envconfig(from = "SOLANA_INVESTOR_KEYPAIR")]
    pub investor_keypair: String,

    #[envconfig(from = "INDEXER_TIER1_PRIZE")]
    pub tier1_prize: String,

    #[envconfig(from = "INDEXER_TIER2_PRIZE_YIELD_SHARE")]
    pub tier2_prize_yield_share: u8,

    #[envconfig(from = "INDEXER_TIER3_PRIZE_YIELD_SHARE")]
    pub tier3_prize_yield_share: u8,

    #[envconfig(from = "INDEXER_YIELD_SPLIT_INSURANCE_JACKPOT")]
    pub yield_split_insurance_jackpot: String,

    #[envconfig(from = "INDEXER_YIELD_SPLIT_INSURANCE_PREMIUM")]
    pub yield_split_insurance_premium: String,

    #[envconfig(from = "INDEXER_YIELD_SPLIT_INSURANCE_PROBABILITY")]
    pub yield_split_insurance_probability: String,

    #[envconfig(from = "INDEXER_YIELD_SPLIT_TREASURY_RATIO")]
    pub yield_split_treasury_ratio: String,

    #[envconfig(from = "FAKE_ARTKAI")]
    pub fake_artkai: bool,

    #[envconfig(from = "ARTKAI_WEBHOOK_URL")]
    pub artkai_webhook_url: Option<String>,

    #[envconfig(from = "ARTKAI_WEBHOOK_TOKEN")]
    pub artkai_webhook_token: Option<String>,

    #[envconfig(from = "YIELD_RANGE_LOW")]
    pub yield_range_low: f64,

    #[envconfig(from = "YIELD_RANGE_HIGH")]
    pub yield_range_high: f64,

    #[envconfig(from = "WINNING_COMBINATION_SOURCE")]
    pub winning_combination_source: String,

    #[envconfig(from = "INVESTOR", default = "fake")]
    pub investor: String,

    #[envconfig(from = "SWITCHBOARD_CONFIG")]
    pub switchboard_config: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();

    env_logger::init();

    let config: AppConfig = AppConfig::init_from_env()?;

    let nezha_api = Box::new(nezha_api::new(&config.nezha_graphql_url));

    let rpc_client = Arc::new(RpcClient::new(config.solana_http_rpc_url.clone()));
    let staking_program_id = Pubkey::from_str(&config.solana_staking_program_id)?;
    let usdc_mint_pubkey = Pubkey::from_str(&config.usdc_mint)?;

    let mut bytes = &config.admin_keypair.as_bytes()[..];
    let admin_keypair = Arc::new(read_keypair(&mut bytes).expect("unable to read keypair"));

    let mut bytes = &config.investor_keypair.as_bytes()[..];
    let investor_keypair = Arc::new(read_keypair(&mut bytes).expect("unable to read keypair"));

    let context = Arc::new(SolanaProgramContext::new(
        rpc_client,
        staking_program_id,
        usdc_mint_pubkey,
        admin_keypair,
        investor_keypair,
    ));

    let prizes = TieredPrizes {
        tier1: config.tier1_prize,
        tier2_yield_share: config.tier2_prize_yield_share,
        tier3_yield_share: config.tier3_prize_yield_share,
    };
    let yield_split_cfg = YieldSplitCfg {
        insurance_premium: config.yield_split_insurance_premium,
        insurance_jackpot: config.yield_split_insurance_jackpot,
        insurance_probability: config.yield_split_insurance_probability,
        treasury_ratio: config.yield_split_treasury_ratio,
    };

    let yield_range = config.yield_range_low..config.yield_range_high;

    let scheduler_config = EpochJobSchedulerConfig {
        start_schedule_string: config.epoch_start_schedule.replace("_", " "),
        enter_investment_offset_seconds: config.epoch_enter_investment_offset_seconds,
        exit_investment_offset_seconds: config.epoch_exit_investment_offset_seconds,
        publish_winning_combination_offset_seconds: config.epoch_publish_winning_combination_offset_seconds,
        publish_winners_offset_seconds: config.epoch_publish_winners_offset_seconds,
    };

    let scheduler = EpochJobScheduler::try_from(scheduler_config)?;

    let artkai_client: Box<dyn ArtkaiUpdater + Send + Sync> = if config.fake_artkai {
        Box::new(FakeArtkaiClient)
    } else {
        Box::new(ArtkaiClient::new(
            reqwest::Client::new(),
            config
                .artkai_webhook_url
                .ok_or(anyhow!("Artkai webhook URL not configured"))?
                .clone(),
            config
                .artkai_webhook_token
                .ok_or(anyhow!("Artkai webhook token not configured"))?
                .clone(),
        ))
    };

    let rng = ChaCha20Rng::from_entropy();

    let sequence_generator = SequenceGenerator::new(rng);

    let winning_combination_source = match config.winning_combination_source.as_str() {
        "guaranteed_jackpot" => WinningCombinationSource::GuaranteedJackpot,
        "optimal" => WinningCombinationSource::Optimal,
        "random" => WinningCombinationSource::Random,
        _ => return Err(anyhow!("Invalid winning combination source")),
    };
    let investor = config.investor.parse::<Investor>()?;

    let switchboard = SwitchboardConfiguration::from_str(&config.switchboard_config).unwrap();

    let mut indexer = EpochIndexer::new(
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
        switchboard,
    );

    indexer.run_loop().await
}
