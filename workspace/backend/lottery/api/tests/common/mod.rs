//! This module is special in Rust, as it can contain non-test code used by the tests.
//!
//! Common mocks, stubs, fakes and dummy data belong here.
//!
use std::{fs, str::FromStr, sync::Arc};

use service::stake::SolanaProgramContext;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signature::read_keypair_file};

pub mod epochs;
pub mod stake;
pub mod tickets;

const SOLANA_DEVNET_URL: &str = "https://api.devnet.solana.com";
const STAKE_PROGRAM_ID: &str = "545Nd9jw9DPiYZRQgzKqfKSEUb1djMCEbrPeSTG7mCFE";
const USDC_MINT: &str = "7tWUTDppUCLm482XrHqZK5mqChepjVdWw6xAkGXRBLeC";
const SPL_TOKEN_PROGRAM_ID: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
const ATA_PROGRAM_ID: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

pub fn setup() -> Arc<SolanaProgramContext> {
    let keypair_path = fs::canonicalize("./tests/keys/nzhq61BXwvKc2dSRKWH9M7danj1GUG3fVUunhkyBYJh.json")
        .expect("unable to find admin key path");

    let rpc_client = RpcClient::new(SOLANA_DEVNET_URL.into());
    let stake_program_id = Pubkey::from_str(STAKE_PROGRAM_ID).expect("unable to parse stake program id");
    let admin_keypair = read_keypair_file(keypair_path).expect("failed to read keypair file");

    Arc::new(SolanaProgramContext::new(
        Arc::new(rpc_client),
        stake_program_id,
        Pubkey::from_str(SPL_TOKEN_PROGRAM_ID).unwrap(),
        Pubkey::from_str(ATA_PROGRAM_ID).unwrap(),
        Pubkey::from_str(USDC_MINT).unwrap(),
        Arc::new(admin_keypair),
    ))
}
