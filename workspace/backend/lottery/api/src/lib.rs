mod epochs;
mod health_check;
pub mod schema;
mod tickets;
mod types;
mod users;

use crate::{health_check::health_check_handler, schema::*};
use actix_web::{dev::Server, guard, middleware, web, App, HttpResponse, HttpServer};
use anyhow::Result;
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptySubscription, Schema,
};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use log::info;
use service::{
    epoch::EpochManager, faucet::FaucetService, health_check::ServiceHealthCheck, prize::PrizeService,
    stake::StakeService, tickets::TicketService, transaction::UserTransactionService,
};
use std::{net::TcpListener, sync::Arc};
pub use types::*;

async fn index(schema: web::Data<NezhaSchema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn index_playground() -> actix_web::Result<HttpResponse> {
    let source = playground_source(GraphQLPlaygroundConfig::new("/").subscription_endpoint("/"));
    Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(source))
}

pub fn load_schema(
    epoch_service: Box<dyn EpochManager>,
    ticket_service: Box<dyn TicketService>,
    stake_service: Box<dyn StakeService>,
    user_transaction_service: Box<dyn UserTransactionService>,
    faucet_service: Box<dyn FaucetService>,
    prize_service: Box<dyn PrizeService>,
) -> Schema<Query, Mutation, EmptySubscription> {
    Schema::build(Query::default(), Mutation::default(), EmptySubscription)
        .data(epoch_service)
        .data(stake_service)
        .data(user_transaction_service)
        .data(ticket_service)
        .data(faucet_service)
        .data(prize_service)
        .finish()
}

pub fn sdl_export() -> String {
    Schema::build(Query::default(), Mutation::default(), EmptySubscription)
        .finish()
        .sdl()
}

pub async fn run(
    listener: TcpListener,
    epoch_service: Box<dyn EpochManager>,
    ticket_service: Box<dyn TicketService>,
    stake_service: Box<dyn StakeService>,
    user_transaction_service: Box<dyn UserTransactionService>,
    faucet_service: Box<dyn FaucetService>,
    prize_service: Box<dyn PrizeService>,
    service_health_check: Arc<ServiceHealthCheck>,
    git_version: String,
) -> Result<Server, std::io::Error> {
    let schema = load_schema(
        epoch_service,
        ticket_service,
        stake_service,
        user_transaction_service,
        faucet_service,
        prize_service,
    );
    let health_check_data = web::Data::from(service_health_check);
    let git_version = web::Data::from(Arc::new(git_version));
    info!("{}", schema.sdl());

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(schema.clone()))
            .wrap(middleware::Logger::default())
            .service(web::resource("/").guard(guard::Post()).to(index))
            .service(web::resource("/").guard(guard::Get()).to(index_playground))
            .service(
                web::scope("/health_check")
                    .app_data(health_check_data.clone())
                    .app_data(git_version.clone())
                    .route("", web::get().to(health_check_handler)),
            )
    })
    .listen(listener)?
    .run();

    Ok(server)
}
