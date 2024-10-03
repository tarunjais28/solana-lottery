use actix_web::{web::Data, HttpRequest, HttpResponse, Result};
use serde::Serialize;
use service::health_check::{HealthCheck, HealthReport, ServiceHealthCheck};

#[derive(Serialize)]
pub struct Info {
    git_commit: String,
}

#[derive(Serialize)]
pub struct HealthCheckResponse {
    pub health_report: HealthReport,
    pub info: Info,
}

pub async fn health_check_handler(req: HttpRequest) -> Result<HttpResponse> {
    let service_health_check = req
        .app_data::<Data<ServiceHealthCheck>>()
        .expect("Could not get service health check");
    let github_ref = req.app_data::<Data<String>>().expect("Github ref not found");
    let health_report = service_health_check.health_check().await;
    Ok(HttpResponse::Ok().json(HealthCheckResponse {
        health_report,
        info: Info {
            git_commit: github_ref.to_string(),
        },
    }))
}
