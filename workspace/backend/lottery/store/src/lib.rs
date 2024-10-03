#[macro_use]
extern crate diesel_migrations;

pub mod epochs;
pub mod faucet;
pub mod health_check;
pub mod migrations;
pub mod prizes;
pub mod stake_update;
pub mod tickets;
pub mod transactions;

use std::str::FromStr;

use anyhow::Result;
pub use deadpool_postgres::Pool;
use deadpool_postgres::{Client, Config, Manager, ManagerConfig, PoolError, RecyclingMethod, Runtime, SslMode};
use envconfig::Envconfig;
use tokio_postgres::NoTls;

#[derive(Envconfig, Clone)]
pub struct DbConfig {
    #[envconfig(from = "DB_HOST", default = "127.0.0.1")]
    pub host: String,

    #[envconfig(from = "DB_PORT", default = "5432")]
    pub port: u16,

    #[envconfig(from = "DB_USER", default = "postgres")]
    pub username: String,

    #[envconfig(from = "DB_PASSWORD", default = "postgres")]
    pub password: String,

    #[envconfig(from = "DB_NAME", default = "lottery")]
    pub database: String,
}

impl DbConfig {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}?sslmode=disable",
            self.username, self.password, self.host, self.port, self.database
        )
    }
}

pub async fn connect(db_config: &DbConfig) -> Pool {
    let cfg = tokio_postgres::Config::from_str(&db_config.connection_string()).unwrap();
    let mgr = Manager::new(cfg, NoTls);

    Pool::builder(mgr).build().unwrap()
}

pub async fn get_client(pool: &Pool) -> Result<Client, PoolError> {
    pool.get().await
}
