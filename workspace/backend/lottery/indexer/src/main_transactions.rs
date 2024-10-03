use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::Result;
use diesel::{Connection, PgConnection};
use envconfig::Envconfig;
use indexer::indexer::util::SolanaProgramContext;
use service::{
    prize::PrizeRepository,
    transaction::{TransactionHistoryRepository, UserTransactionRepository},
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::read_keypair};
use store::{
    prizes::PostgresPrizeRepository,
    transactions::{PostgresTransactionHistoryRepository, PostgresUserTransactionRepository},
    DbConfig,
};

#[derive(Envconfig, Clone)]
pub struct AppConfig {
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

    #[envconfig(from = "INDEXER_TRANSACTIONS_RETRY_DELAY_SECONDS", default = "10")]
    pub indexer_transactions_retry_delay_seconds: u64,

    #[envconfig(from = "INDEXER_TRANSACTIONS_BATCH_SIZE", default = "50")]
    pub indexer_transactions_batch_size: usize,

    #[envconfig(from = "TRANSACTION_MAX_QUERY_LIMIT", default = "100")]
    pub transaction_max_query_limit: i64,

    #[envconfig(from = "PRIZE_MAX_QUERY_LIMIT", default = "100")]
    pub prize_max_query_limit: i64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();
    env_logger::init();

    let config: AppConfig = AppConfig::init_from_env()?;
    let db_config: DbConfig = DbConfig::init_from_env()?;

    store::migrations::run(&db_config);
    let db_pool = store::connect(&db_config).await;

    let transaction_history_repository: Box<dyn TransactionHistoryRepository> =
        Box::new(PostgresTransactionHistoryRepository::new(db_pool.clone()));
    let user_transaction_repository: Box<dyn UserTransactionRepository> = Box::new(
        PostgresUserTransactionRepository::new(db_pool.clone(), config.transaction_max_query_limit),
    );
    let prize_repository: Box<dyn PrizeRepository> = Box::new(PostgresPrizeRepository::new(
        db_pool.clone(),
        config.prize_max_query_limit,
    ));
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        config.solana_http_rpc_url,
        CommitmentConfig::finalized(),
    ));
    let staking_program_id = Pubkey::from_str(&config.solana_staking_program_id)?;
    let usdc_mint_pubkey = Pubkey::from_str(&config.usdc_mint)?;

    let mut bytes = &config.admin_keypair.as_bytes()[..];
    let admin_keypair = Arc::new(read_keypair(&mut bytes).unwrap());

    let mut bytes = &config.investor_keypair.as_bytes()[..];
    let investor_keypair = Arc::new(read_keypair(&mut bytes).unwrap());
    let context = Arc::new(SolanaProgramContext::new(
        rpc_client.clone(),
        staking_program_id,
        usdc_mint_pubkey,
        admin_keypair,
        investor_keypair,
    ));
    let transaction_indexer = indexer::indexer::transactions::polling::PollingIndexer::new(
        rpc_client,
        context,
        Duration::from_secs(config.indexer_transactions_retry_delay_seconds),
        config.indexer_transactions_batch_size,
        transaction_history_repository,
        user_transaction_repository,
        prize_repository,
    );

    transaction_indexer.run_loop().await?;

    Ok(())
}
