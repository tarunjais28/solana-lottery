use deadpool_postgres::Pool;
use envconfig::Envconfig;
use store::{connect, DbConfig};

pub async fn setup() -> Pool {
    let _ = dotenv::dotenv();

    let db_config = DbConfig::init_from_env().expect("Failed to store DbConfig from env vars");
    let pool = connect(&db_config).await;
    pool
}
