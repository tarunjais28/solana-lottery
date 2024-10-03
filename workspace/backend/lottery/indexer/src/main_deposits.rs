use anyhow::Result;
use envconfig::Envconfig;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use indexer::indexer::deposits::polling::{AcceptDeposits, AcceptDepositsConfig, GenerateTickets};
use indexer::indexer::deposits::pubsub::{SolanaPubsub, SolanaPubsubConfig};
use indexer::nezha_api;

#[derive(Envconfig, Clone)]
pub struct AppConfig {
    #[envconfig(from = "INDEXER_NEZHA_GRAPHQL_URL")]
    pub nezha_graphql_url: String,

    #[envconfig(from = "SOLANA_WS_RPC_URL")]
    pub solana_ws_rpc_url: String,

    #[envconfig(from = "SOLANA_HTTP_RPC_URL")]
    pub solana_http_rpc_url: String,

    #[envconfig(from = "INDEXER_DEPOSITS_SOLANA_PROGRAM_ID")]
    pub solana_program_id: String,

    #[envconfig(from = "INDEXER_PUBSUB_RETRY_DELAY_MS")]
    pub pubsub_retry_interval_ms: u64,

    #[envconfig(from = "INDEXER_DEPOSITS_POLL_FREQ_MS")]
    pub approve_deposits_poll_freq_ms: u64,

    #[envconfig(from = "INDEXER_DEPOSITS_BATCH_GAP_MS")]
    pub approve_deposits_batch_gap_ms: u64,

    #[envconfig(from = "INDEXER_DEPOSITS_BATCH_SIZE")]
    pub approve_deposits_batch_size: usize,

    #[envconfig(from = "INDEXER_TICKETS_POLL_FREQ_MS")]
    pub generate_tickets_poll_freq_ms: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();

    env_logger::init();

    let config: AppConfig = AppConfig::init_from_env()?;

    let cancelled = Arc::new(AtomicBool::new(false));

    let pubsub_loop = {
        let program_id = Pubkey::from_str(&config.solana_program_id).unwrap();
        let nezha_graphql_url = config.nezha_graphql_url.clone();
        let pubsub_retry_interval = Duration::from_millis(config.pubsub_retry_interval_ms);
        let cancelled = cancelled.clone();
        let pubsub_cfg = SolanaPubsubConfig {
            program_id,
            rpc_ws_url: config.solana_ws_rpc_url,
        };
        let rpc_client = Arc::new(RpcClient::new(config.solana_http_rpc_url));
        let pubsub_loop = tokio::spawn(async move {
            let mut last_connected: Option<SystemTime> = None;
            while !cancelled.load(Ordering::Relaxed) {
                if let Some(last_connected) = last_connected {
                    let elapsed = last_connected.elapsed().unwrap_or_default();
                    if elapsed < pubsub_retry_interval {
                        let sleep_duration = elapsed - pubsub_retry_interval;
                        log::info!("Sleeping {}s before next retry", sleep_duration.as_secs());
                        tokio::time::sleep(sleep_duration).await;
                    }
                }

                let nezha_api = Arc::new(nezha_api::new(&nezha_graphql_url));
                let mut pubsub = SolanaPubsub::new(program_id, rpc_client.clone(), nezha_api);
                last_connected = Some(SystemTime::now());
                match pubsub.run(cancelled.clone(), &pubsub_cfg).await {
                    Err(e) => log::error!("Pubsub loop exited with error: {e}"),
                    Ok(()) => log::info!("Pubsub loop exited"),
                }
            }
        });
        pubsub_loop
    };

    let accept_deposits_loop = {
        let cancelled = cancelled.clone();
        let nezha_api = Arc::new(nezha_api::new(&config.nezha_graphql_url));
        let poll_frequency = Duration::from_millis(config.approve_deposits_poll_freq_ms);
        let accept_deposits_cfg = AcceptDepositsConfig {
            batch_gap: Duration::from_millis(config.approve_deposits_batch_gap_ms),
            batch_size: config.approve_deposits_batch_size,
        };
        let accept_deposits = AcceptDeposits {
            nezha_api,
            config: accept_deposits_cfg,
        };
        let accept_deposits_loop = tokio::spawn(async move {
            while !cancelled.load(Ordering::Relaxed) {
                if let Err(e) = accept_deposits.run(cancelled.clone()).await {
                    log::error!("Accept deposits exited with error: {e}")
                }
                log::info!("Poll Accept deposits sleeping for {}s", poll_frequency.as_secs_f32());
                tokio::time::sleep(poll_frequency).await;
            }
        });
        accept_deposits_loop
    };

    let gen_tickets_loop = {
        let cancelled = cancelled.clone();
        let nezha_api = Box::new(nezha_api::new(&config.nezha_graphql_url));
        let poll_frequency = Duration::from_millis(config.generate_tickets_poll_freq_ms);

        let gen_tickets = GenerateTickets { nezha_api };
        let gen_tickets_loop = tokio::spawn(async move {
            while !cancelled.load(Ordering::Relaxed) {
                if let Err(e) = gen_tickets.run().await {
                    log::error!("Generate tickets exited with error: {e}")
                }
                log::info!("Poll Generate tickets sleeping for {}s", poll_frequency.as_secs_f32());
                tokio::time::sleep(poll_frequency).await;
            }
        });
        gen_tickets_loop
    };

    let _ = tokio::join!(pubsub_loop, accept_deposits_loop, gen_tickets_loop);

    Ok(())
}
