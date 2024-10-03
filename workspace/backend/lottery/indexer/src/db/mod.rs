pub mod migrations;
pub mod risq_epochs;

use anyhow::Result;
pub use deadpool_postgres::Pool;
use deadpool_postgres::{Client, Config, ManagerConfig, PoolError, RecyclingMethod, Runtime};
use envconfig::Envconfig;
use tokio_postgres::NoTls;

#[derive(Envconfig, Clone)]
pub struct DbConfig {
    #[envconfig(from = "INDEXER_DB_HOST", default = "127.0.0.1")]
    pub host: String,

    #[envconfig(from = "INDEXER_DB_PORT", default = "5432")]
    pub port: u16,

    #[envconfig(from = "INDEXER_DB_USER", default = "postgres")]
    pub username: String,

    #[envconfig(from = "INDEXER_DB_PASSWORD")]
    pub password: String,

    #[envconfig(from = "INDEXER_DB_NAME", default = "indexer")]
    pub database: String,
}

impl DbConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database
        )
    }
}

pub async fn connect(db_config: DbConfig) -> Pool {
    let mut cfg = Config::new();
    cfg.host = Some(db_config.host);
    cfg.port = Some(db_config.port);
    cfg.user = Some(db_config.username);
    cfg.password = Some(db_config.password);
    cfg.dbname = Some(db_config.database);
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    cfg.create_pool(Some(Runtime::Tokio1), NoTls).unwrap()
}

pub async fn get_client(pool: &Pool) -> Result<Client, PoolError> {
    pool.get().await
}

