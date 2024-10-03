use service::health_check::HealthCheck;
use std::time::Duration;
use store::health_check::DbHealthCheck;

use crate::common;

#[tokio::test]
async fn test_db_health_check() {
    let pool = common::setup().await;
    let db_health_check = DbHealthCheck::new(pool, Duration::from_secs(20));
    let db_health_report = db_health_check.health_check().await;
    assert!(db_health_report.status.is_ok(), "{db_health_report:?}");

    let expected_tables = ["stake_update", "faucet", "epoch", "ticket", "sequences"];
    let tables = db_health_report.inner.keys().collect::<Vec<_>>();
    for table in expected_tables {
        assert!(tables.contains(&&table.to_string()));
    }
}
