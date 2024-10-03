use anyhow::{Context, Result};
use envconfig::Envconfig;

use indexer::dummy_draw_config::DummyDrawConfig;
use risq_api_client::resources::{configs, DrawRoutes, EntryRoutes};
use rust_decimal::Decimal;
use std::env;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use indexer::db;
use indexer::db::risq_epochs::{EpochStore, PostgresEpochStore};
use indexer::db::DbConfig;
use indexer::indexer::risq::{fake_risq, IndexerRisq, IndexerRisqConfig};
use indexer::nezha_api;

#[derive(Envconfig, Clone)]
pub struct AppConfig {
    #[envconfig(from = "INDEXER_NEZHA_GRAPHQL_URL")]
    pub nezha_graphql_url: String,

    #[envconfig(from = "INDEXER_RISQ_FAKE", default = "false")]
    pub risq_fake: bool,

    #[envconfig(from = "INDEXER_RISQ_BASE_URL")]
    pub risq_base_url: Option<String>,

    #[envconfig(from = "INDEXER_RISQ_API_PARTNER_ID")]
    pub risq_partner_id: Option<String>,

    #[envconfig(from = "INDEXER_RISQ_API_KEY")]
    pub risq_api_key: Option<String>,

    #[envconfig(from = "INDEXER_RISQ_PRODUCT_ID")]
    pub risq_product_id: Option<String>,

    #[envconfig(from = "INDEXER_RISQ_LICENSEE_ID")]
    pub risq_licensee_id: Option<String>,

    #[envconfig(from = "INDEXER_RISQ_PRIZE_AMOUNT")]
    pub risq_prize_amount: Option<Decimal>,

    #[envconfig(from = "INDEXER_RISQ_TICKET_BATCH_SIZE")]
    pub risq_ticket_batch_size: usize,

    #[envconfig(from = "INDEXER_RISQ_SLEEP_BETWEEN_BATCHES_MS")]
    pub risq_sleep_between_batches_ms: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();

    env_logger::init();

    let db_config: DbConfig = DbConfig::init_from_env()?;
    db::migrations::run(&db_config);

    let config: AppConfig = AppConfig::init_from_env()?;
    let _ = dbg!(env::var("INDEXER_RISQ_FAKE"), config.risq_fake);
    dbg!(config.risq_fake);

    let db_pool = db::connect(db_config).await;

    let (risq_draw_routes, risq_entry_routes, risq_config) = make_risq_config(&config)?;
    let epoch_store: Box<dyn EpochStore + Send + Sync> = Box::new(PostgresEpochStore::new(db_pool.clone()));

    let nezha_api = Box::new(nezha_api::new(&config.nezha_graphql_url));

    let indexer = IndexerRisq {
        epoch_store,
        nezha_api,
        risq_draw_routes,
        risq_entry_routes,
        config: risq_config,
    };

    let cancelled = Arc::new(AtomicBool::new(false));
    indexer.run_loop(cancelled).await
}

fn make_risq_config(
    config: &AppConfig,
) -> Result<(
    Box<dyn DrawRoutes + Send + Sync>,
    Box<dyn EntryRoutes + Send + Sync>,
    IndexerRisqConfig,
)> {
    if !config.risq_fake {
        let risq_client = risq_api_client::new_client(
            config
                .risq_base_url
                .clone()
                .with_context(|| "Expected risq base url to be provided")?,
            config
                .risq_partner_id
                .clone()
                .with_context(|| "Expected risq partner id to be provided")?,
            config
                .risq_api_key
                .clone()
                .with_context(|| "Expected risq api key to be provided")?,
        );
        let config = IndexerRisqConfig {
            risq_licensee_id: config
                .risq_licensee_id
                .clone()
                .with_context(|| "Expected risq licensee id to be provided")?,
            risq_product_id: config
                .risq_product_id
                .clone()
                .with_context(|| "Expected risq product id to be provided")?,
            nezha_prize_amount: config
                .risq_prize_amount
                .clone()
                .with_context(|| "Expected nezha prize amount to be provided")?,
            ticket_batch_size: config.risq_ticket_batch_size,
            sleep_between_batches: Duration::from_millis(config.risq_sleep_between_batches_ms),
            draw_config: Box::new(configs::nezha::Nezha),
        };
        Ok((risq_client.draw, risq_client.entry, config))
    } else {
        let draw: Box<dyn DrawRoutes + Send + Sync> = Box::new(fake_risq::DrawRoutesImpl);
        let entry: Box<dyn EntryRoutes + Send + Sync> = Box::new(fake_risq::EntryRoutesImpl);
        let config = IndexerRisqConfig {
            risq_licensee_id: String::new(),
            risq_product_id: String::new(),
            nezha_prize_amount: Decimal::ZERO,
            ticket_batch_size: config.risq_ticket_batch_size,
            sleep_between_batches: Duration::from_millis(config.risq_sleep_between_batches_ms),
            draw_config: Box::new(DummyDrawConfig),
        };
        Ok((draw, entry, config))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    static INDEXER_ENV_BASE: [(&str, &str); 3] = [
        ("INDEXER_NEZHA_GRAPHQL_URL", ""),
        ("INDEXER_RISQ_TICKET_BATCH_SIZE", "0"),
        ("INDEXER_RISQ_SLEEP_BETWEEN_BATCHES_MS", "0"),
    ];

    static INDEXER_ENV_RISQ: [(&str, &str); 6] = [
        ("INDEXER_RISQ_BASE_URL", ""),
        ("INDEXER_RISQ_API_PARTNER_ID", ""),
        ("INDEXER_RISQ_API_KEY", ""),
        ("INDEXER_RISQ_PRODUCT_ID", ""),
        ("INDEXER_RISQ_LICENSEE_ID", ""),
        ("INDEXER_RISQ_PRIZE_AMOUNT", "0"),
    ];

    fn make_app_config<'a>(envs: &mut dyn Iterator<Item = (&'a str, &'a str)>) -> Result<AppConfig> {
        let base_iter = INDEXER_ENV_BASE.into_iter().map(|(a, b)| (a.to_owned(), b.to_owned()));
        let envs_iter = envs.map(|(a, b)| (a.to_owned(), b.to_owned()));
        AppConfig::init_from_hashmap(&HashMap::from_iter(base_iter.chain(envs_iter)))
            .with_context(|| "failed to make appconfig")
    }

    #[test]
    fn test_risq_fake_env_parsing() -> Result<()> {
        let app_config = make_app_config(&mut [("INDEXER_RISQ_FAKE", "true")].into_iter())
            .with_context(|| "Should be able to parse true")?;
        assert_eq!(app_config.risq_fake, true);

        let app_config = make_app_config(&mut [("INDEXER_RISQ_FAKE", "false")].into_iter())
            .with_context(|| "Should be able to parse false")?;
        assert_eq!(app_config.risq_fake, false);

        let app_config = make_app_config(&mut [].into_iter()).with_context(|| "Should default to false if omitted")?;
        assert_eq!(app_config.risq_fake, false);

        let app_config = make_app_config(&mut [("INDEXER_RISQ_FAKE", "")].into_iter());
        assert!(app_config.is_err());

        let app_config = make_app_config(&mut [("INDEXER_RISQ_FAKE", "1")].into_iter());
        assert!(app_config.is_err());

        let app_config = make_app_config(&mut [("INDEXER_RISQ_FAKE", "0")].into_iter());
        assert!(app_config.is_err());

        Ok(())
    }

    #[test]
    fn test_make_risq_config() -> Result<()> {
        let app_config = make_app_config(&mut [("INDEXER_RISQ_FAKE", "true")].into_iter())?;
        let _ = make_risq_config(&app_config)?; // Should return Ok()

        let app_config = make_app_config(&mut [("INDEXER_RISQ_FAKE", "false")].into_iter())?;
        let res = make_risq_config(&app_config);
        // Should provide other non optional env vars for it to succeed
        assert!(res.is_err());

        // INDEXER_RISQ_FAKE should default to false if not provided
        let app_config = make_app_config(&mut [].into_iter())?;
        let res = make_risq_config(&app_config);
        // Should provide other non optional env vars for it to succeed
        assert!(res.is_err());

        let app_config = make_app_config(&mut INDEXER_ENV_RISQ.into_iter().chain([("INDEXER_RISQ_FAKE", "false")]))?;
        let _ = make_risq_config(&app_config)?; // Should return Ok()

        Ok(())
    }
}
