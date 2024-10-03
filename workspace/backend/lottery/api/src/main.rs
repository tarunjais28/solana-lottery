use anyhow::{anyhow, Result};
use envconfig::Envconfig;
use git_version::git_version;
use log::info;
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use service::epoch::EpochRepository;
use service::epoch::{service::EpochService, EpochManager};
use service::faucet::SolanaFaucetService;
use service::health_check::ServiceHealthCheck;
use service::prize::PrizeServiceImpl;
use service::solana::rpc::SolanaRpcReal;
use service::solana::solana_impl::SolanaImpl;
use service::solana::{SwitchboardConfiguration, VrfConfiguration, FPUSDC};
use service::stake::DefaultStakeService;
use service::stake::StakeService;
use service::tickets::bonus::{BonusInfo, BonusSequenceCount, DefaultBonusInfoService};
use service::tickets::{ConstantTicketPriceCalculator, DefaultTicketService, TicketRepository, TicketService};
use service::transaction::{UserTransactionRepository, UserTransactionService, UserTransactionServiceImpl};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signer::keypair::read_keypair};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{net::TcpListener, str::FromStr};
use store::epochs::PostgresEpochRepository;
use store::faucet::PostgresFaucetRepository;
use store::health_check::DbHealthCheck;
use store::prizes::PostgresPrizeRepository;
use store::transactions::PostgresUserTransactionRepository;
use store::{stake_update::PostgresStakeUpdateRepository, tickets::PostgresTicketRepository, DbConfig};

#[derive(Envconfig, Clone)]
pub struct AppConfig {
    #[envconfig(from = "APP_HOST", default = "127.0.0.1")]
    pub host: String,

    #[envconfig(from = "APP_PORT", default = "8080")]
    pub port: u16,

    #[envconfig(from = "SOLANA_HTTP_RPC_URL", default = "https://api.mainnet-beta.solana.com")]
    pub solana_http_rpc_url: String,

    #[envconfig(from = "SOLANA_STAKING_PROGRAM_ID")]
    pub solana_staking_program_id: String,

    #[envconfig(from = "SOLANA_VRF_PROGRAM_ID")]
    pub solana_vrf_program_id: String,

    #[envconfig(from = "SOLANA_USDC_MINT")]
    pub usdc_mint: String,

    #[envconfig(from = "SOLANA_NEZ_MINT")]
    pub nez_mint: String,

    #[envconfig(from = "SOLANA_ADMIN_KEYPAIR")]
    pub admin_keypair: String,

    #[envconfig(from = "SOLANA_INVESTOR_KEYPAIR")]
    pub investor_keypair: String,

    #[envconfig(from = "FAUCET_RETRY_LIMIT_SECONDS")]
    pub faucet_retry_limit_seconds: i64,

    #[envconfig(from = "FAUCET_MINT_AMOUNT")]
    pub faucet_mint_amount: u64,

    #[envconfig(from = "ENABLE_FAUCET", default = "false")]
    pub enable_faucet: bool,

    #[envconfig(from = "SIGNUP_BONUS_SEQUENCE_COUNT")]
    pub signup_bonus_sequence_count: String,

    #[envconfig(from = "SIGNUP_BONUS_MIN_SEQUENCE_COUNT")]
    pub signup_bonus_min_sequence_count: u32,

    #[envconfig(from = "SIGNUP_BONUS_SEQUENCE_MIN_STAKE")]
    pub signup_bonus_sequence_min_stake: String,

    #[envconfig(from = "TRANSACTION_MAX_QUERY_LIMIT", default = "100")]
    pub transaction_max_query_limit: i64,

    #[envconfig(from = "PRIZE_MAX_QUERY_LIMIT", default = "100")]
    pub prize_max_query_limit: i64,

    #[envconfig(from = "SWITCHBOARD_CONFIG", default = "fake")]
    pub switchboard_configuration: String,
}

impl AppConfig {
    pub fn connection_string(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let db_config: DbConfig = DbConfig::init_from_env()?;
    store::migrations::run(&db_config);

    let config: AppConfig = AppConfig::init_from_env()?;

    let solana = new_solana(&config).await;

    let db_pool = store::connect(&db_config).await;
    let listener = TcpListener::bind(config.connection_string())?;

    let rng = Arc::new(Mutex::new(ChaChaRng::from_entropy()));
    let stake_update_repo = PostgresStakeUpdateRepository::new(db_pool.clone());
    let stake_service: Box<dyn StakeService> = Box::new(DefaultStakeService::new(
        Box::new(solana.clone()),
        Box::new(stake_update_repo.clone()),
    ));
    let user_transaction_repository: Box<dyn UserTransactionRepository> = Box::new(
        PostgresUserTransactionRepository::new(db_pool.clone(), config.transaction_max_query_limit),
    );
    let user_transaction_service: Box<dyn UserTransactionService> =
        Box::new(UserTransactionServiceImpl::new(user_transaction_repository));
    let epoch_repository: Box<dyn EpochRepository> = Box::new(PostgresEpochRepository::new(db_pool.clone()));
    let ticket_repository: Box<dyn TicketRepository> = Box::new(PostgresTicketRepository::new(db_pool.clone()));
    let epoch_service: Box<dyn EpochManager> = Box::new(EpochService::new(
        Box::new(solana.clone()),
        epoch_repository,
        ticket_repository,
    ));

    let ticket_price_calc = ConstantTicketPriceCalculator::new("25.0".parse().unwrap());
    let ticket_repository: Box<dyn TicketRepository> = Box::new(PostgresTicketRepository::new(db_pool.clone()));
    let sub_seq_count = if config.signup_bonus_sequence_count.chars().last() == Some('X') {
        let mul = config.signup_bonus_sequence_count[..config.signup_bonus_sequence_count.len() - 1].parse::<f32>()?;
        let min_count = config.signup_bonus_min_sequence_count;
        BonusSequenceCount::Multiplier { mul, min_count }
    } else {
        let count = config.signup_bonus_sequence_count.parse::<u32>()?;
        BonusSequenceCount::Constant(count)
    };

    let bonus_info_service = Box::new(DefaultBonusInfoService::new(BonusInfo {
        sub_seq_count,
        sub_seq_min_stake: config
            .signup_bonus_sequence_min_stake
            .parse()
            .map_err(|e: String| anyhow!(e))?,
    }));
    let ticket_service: Box<dyn TicketService> = Box::new(DefaultTicketService::new(
        rng,
        Box::new(solana.clone()),
        ticket_repository,
        Box::new(ticket_price_calc),
        bonus_info_service,
    ));
    let faucet_repository = PostgresFaucetRepository::new(db_pool.clone());
    let faucet_retry_time_limit = chrono::Duration::seconds(config.faucet_retry_limit_seconds);
    let faucet_mint_amount = FPUSDC::from(config.faucet_mint_amount);
    let faucet_service = Box::new(SolanaFaucetService::new(
        faucet_retry_time_limit,
        faucet_mint_amount,
        Box::new(faucet_repository),
        Box::new(solana.clone()),
        config.enable_faucet,
    ));

    let prize_repository = PostgresPrizeRepository::new(db_pool.clone(), config.prize_max_query_limit);
    let prize_service = Box::new(PrizeServiceImpl::new(Box::new(prize_repository)));

    let git_version: &str = git_version!(args = ["--abbrev=40", "--always"]);

    let db_health_check = DbHealthCheck::new(db_pool.clone(), Duration::from_secs(20));
    let service_health_check = Arc::new(ServiceHealthCheck::new(Box::new(db_health_check)));

    info!("Starting server");

    api::run(
        listener,
        epoch_service,
        ticket_service,
        stake_service,
        user_transaction_service,
        faucet_service,
        prize_service,
        service_health_check,
        git_version.to_string(),
    )
    .await?
    .await?;

    Ok(())
}

async fn new_solana(config: &AppConfig) -> SolanaImpl {
    let mut bytes = &config.admin_keypair.as_bytes()[..];
    let admin_keypair = Arc::new(read_keypair(&mut bytes).expect("unable to read admin keypair"));

    let mut bytes = &config.investor_keypair.as_bytes()[..];
    let investor_keypair = Arc::new(read_keypair(&mut bytes).expect("unable to read investor keypair"));

    let program_id = Pubkey::from_str(&config.solana_staking_program_id).unwrap();
    let usdc_mint = Pubkey::from_str(&config.usdc_mint).unwrap();
    let nez_mint = Pubkey::from_str(&config.nez_mint).unwrap();
    let vrf_program_id = Pubkey::from_str(&config.solana_vrf_program_id).unwrap();

    let rpc_url = config.solana_http_rpc_url.clone();
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let switchboard_config: SwitchboardConfiguration =
        SwitchboardConfiguration::from_str(&config.switchboard_configuration).unwrap();

    let vrf_configuration = VrfConfiguration::build(vrf_program_id, switchboard_config, &rpc_client)
        .await
        .unwrap();

    let solana_rpc = Arc::new(SolanaRpcReal::new(rpc_client, program_id));

    let solana = SolanaImpl {
        rpc_client: solana_rpc.clone(),
        program_id,
        usdc_mint,
        nez_mint,
        admin_keypair: admin_keypair.clone(),
        investor_keypair: investor_keypair.clone(),
        vrf_configuration,
    };

    solana
}
