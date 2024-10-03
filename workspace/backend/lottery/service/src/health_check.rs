use std::collections::HashMap;

use async_trait::async_trait;
use serde::Serialize;

#[derive(Serialize, PartialEq, Clone, Debug)]
pub struct HealthReport {
    #[serde(serialize_with = "serialize_result")]
    pub status: Result<(), String>,
    #[serde(flatten)]
    pub inner: HashMap<String, HealthReport>,
}

fn serialize_result<S>(result: &Result<(), String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match result {
        Ok(_) => serializer.serialize_str("Ok"),
        Err(e) => serializer.serialize_str(e),
    }
}

#[async_trait]
pub trait HealthCheck: Sync + Send {
    async fn health_check(&self) -> HealthReport;
}

pub struct ServiceHealthCheck {
    db_health_check: Box<dyn HealthCheck>,
}

impl ServiceHealthCheck {
    pub fn new(db_health_check: Box<dyn HealthCheck>) -> Self {
        Self { db_health_check }
    }
}

#[async_trait]
impl HealthCheck for ServiceHealthCheck {
    async fn health_check(&self) -> HealthReport {
        let mut report = HealthReport {
            status: Ok(()),
            inner: HashMap::new(),
        };
        let db_health_status = self.db_health_check.health_check().await;
        report.inner.insert("db".to_string(), db_health_status);

        for (_, status) in report.inner.iter() {
            if status.status.is_err() {
                report.status = Err("Service is not healthy".to_string());
                break;
            }
        }
        report
    }
}
