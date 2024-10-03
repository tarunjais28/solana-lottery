#![allow(dead_code)]
use std::{str::FromStr, sync::Arc};

use borsh::BorshSerialize;
use deadpool_postgres::Pool;
use envconfig::Envconfig;
use indexer::{
    db::{connect, DbConfig},
    indexer::util::SolanaProgramContext,
    nezha_api::{self, NezhaAPI},
};
use nezha_staking::state::AccountType;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    pubsub_client::{ProgramSubscription, PubsubClient},
    rpc_config::RpcProgramAccountsConfig,
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_sdk::{
    commitment_config::{CommitmentConfig, CommitmentLevel},
    pubkey::Pubkey,
    signer::keypair::read_keypair_file,
};

pub mod util;

const INDEXER_NEZHA_GRAPHQL_URL: &str = "http://localhost:8080/";
const ADMIN_KEYPAIR_PATH: &str = "./tests/keys/admin.json";
const INVESTOR_KEYPAIR_PATH: &str = "./tests/keys/investor.json";
const STAKING_PROGRAM_ID: &str = "stkt5YJMm5gVBRaFER6QNhkfteSZFU64MeR4BaiH8cL";
const USDC_MINT_PUBKEY: &str = "5dKXSr4Yyhn48r8ERRtE9hBoNr2kWQL8EUpBNCRjQHKa";
const RPC_URL: &str = "http://localhost:8899";

pub async fn setup_store() -> Pool {
    let _ = dotenv::dotenv();

    let db_config = DbConfig::init_from_env().expect("Failed to indexer DbConfig from env vars");
    let pool = connect(db_config).await;
    pool
}

pub fn setup_context() -> Arc<SolanaProgramContext> {
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        RPC_URL.to_string(),
        CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        },
    ));
    let admin_keypair = Arc::new(read_keypair_file(ADMIN_KEYPAIR_PATH).unwrap());
    let investor_keypair = Arc::new(read_keypair_file(INVESTOR_KEYPAIR_PATH).unwrap());
    let staking_program_id = Pubkey::from_str(STAKING_PROGRAM_ID).unwrap();
    let usdc_mint_pubkey = Pubkey::from_str(USDC_MINT_PUBKEY).unwrap();
    let context = SolanaProgramContext::new(
        rpc_client,
        staking_program_id,
        usdc_mint_pubkey,
        admin_keypair,
        investor_keypair,
    );
    Arc::new(context)
}

pub fn setup_pubsub() -> ProgramSubscription {
    let staking_program_id = Pubkey::from_str(STAKING_PROGRAM_ID).unwrap();
    let solana_pubsub = PubsubClient::program_subscribe(
        RPC_URL,
        &staking_program_id,
        Some(RpcProgramAccountsConfig {
            filters: Some(vec![RpcFilterType::Memcmp(Memcmp {
                offset: 0,
                bytes: MemcmpEncodedBytes::Bytes(AccountType::StakeUpdateRequest.try_to_vec().unwrap()),
                encoding: None,
            })]),
            ..Default::default()
        }),
    )
    .expect("Can't create program subscription");
    solana_pubsub
}

pub fn setup_api() -> Box<impl NezhaAPI> {
    Box::new(nezha_api::new(INDEXER_NEZHA_GRAPHQL_URL))
}
