use std::{collections::HashMap, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::Pool;
use service::health_check::{HealthCheck, HealthReport};

pub struct DbHealthCheck {
    pool: Pool,
    timeout: Duration,
}

impl DbHealthCheck {
    pub fn new(pool: Pool, timeout: Duration) -> Self {
        Self { pool, timeout }
    }

    async fn tables(&self) -> Result<Vec<String>> {
        let client = self.pool.get().await?;

        let rows = tokio::time::timeout(
            self.timeout,
            client.query(
                "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname=$1",
                &[&"public"],
            ),
        )
        .await??;

        let tables = rows
            .iter()
            .map(|row| row.get::<_, String>("tablename"))
            .filter(|table_name| table_name != "__diesel_schema_migrations")
            .collect::<Vec<_>>();

        Ok(tables)
    }

    async fn table_health_check(&self, table_name: &str) -> HealthReport {
        let mut table_health_report = HealthReport {
            status: Ok(()),
            inner: HashMap::new(),
        };
        let client = match self.pool.get().await {
            Ok(client) => client,
            Err(err) => {
                table_health_report.status = Err(err.to_string());
                return table_health_report;
            }
        };

        let query = "SELECT * FROM ".to_string() + table_name + " LIMIT 1";
        match tokio::time::timeout(self.timeout, client.execute(&query, &[])).await {
            Ok(Ok(_)) => {}
            Ok(Err(err)) => {
                table_health_report.status = Err(err.to_string());
            }
            Err(err) => {
                table_health_report.status = Err(err.to_string());
            }
        }
        table_health_report
    }
}

#[async_trait]
impl HealthCheck for DbHealthCheck {
    async fn health_check(&self) -> HealthReport {
        let mut db_health_report = HealthReport {
            status: Ok(()),
            inner: HashMap::new(),
        };

        let tables = match self.tables().await {
            Ok(tables) => tables,
            Err(e) => {
                db_health_report.status = Err(e.to_string());
                return db_health_report;
            }
        };

        for table in tables.iter().to_owned() {
            let table_health_report = self.table_health_check(table).await;
            if table_health_report.status.is_err() {
                db_health_report.status = Err("Database is not healthy".to_string());
            }
            db_health_report.inner.insert(table.to_string(), table_health_report);
        }

        db_health_report
    }
}
