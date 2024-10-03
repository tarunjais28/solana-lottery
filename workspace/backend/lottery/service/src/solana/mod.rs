pub mod error;
pub mod rpc;
pub mod solana_impl;

pub mod with_pubkey;
use anyhow::bail;
use nezha_staking::instruction::{CreateEpochWinnersMetaArgs, WinnerInput};
use nezha_vrf_lib::{
    state::NezhaVrfRequest,
    switchboard::{self, SWITCHBOARD_LABS_DEVNET_PERMISSIONLESS_QUEUE, SWITCHBOARD_LABS_MAINNET_PERMISSIONLESS_QUEUE},
};
use solana_client::nonblocking::rpc_client::RpcClient;
pub use with_pubkey::*;

pub mod mock;

pub mod models;
pub use models::*;

use async_trait::async_trait;
pub use error::*;

use std::{str::FromStr, sync::Arc};

use chrono::{DateTime, Utc};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};

pub use nezha_staking::{
    fixed_point::FPUSDC,
    state::{
        Epoch, InsuranceCfg, LatestEpoch, Returns, Stake as SolanaStake, StakeUpdateRequest, StakeUpdateState,
        YieldSplitCfg,
    },
};

const ALLOWED_SWITCHBOARD_CONF_VALUES: [&str; 3] = ["fake", "devnet", "mainnet"];

#[derive(Debug, Clone)]
pub enum SwitchboardConfiguration {
    Fake,
    Devnet,
    Mainnet,
}

impl FromStr for SwitchboardConfiguration {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let r = match s.to_lowercase().as_str() {
            "fake" => Self::Fake,
            "devnet" => Self::Devnet,
            "mainnet" => Self::Mainnet,

            _ => bail!("must be one of {}", ALLOWED_SWITCHBOARD_CONF_VALUES.join(",")),
        };

        Ok(r)
    }
}

#[derive(Debug, Clone)]
pub enum VrfConfiguration {
    Fake {
        program_id: Pubkey,
    },
    Switchboard {
        program_id: Pubkey,
        details: SwitchboardDetails,
    },
}

#[derive(Debug, Clone)]
pub struct SwitchboardDetails {
    switchboard_program_id: Pubkey,
    switchboard_oracle_queue: Pubkey,
    switchboard_oracle_queue_authority: Pubkey,
    switchboard_oracle_queue_mint: Pubkey,
    switchboard_oracle_queue_data_buffer: Pubkey,
}

impl VrfConfiguration {
    pub fn is_fake(&self) -> bool {
        match self {
            VrfConfiguration::Fake { .. } => true,
            _ => false,
        }
    }

    pub fn details(&self) -> Option<&SwitchboardDetails> {
        match self {
            VrfConfiguration::Switchboard { details, .. } => Some(details),
            _ => None,
        }
    }

    pub async fn build(
        vrf_program_id: Pubkey,
        switchboard_cfg: SwitchboardConfiguration,
        rpc: &RpcClient,
    ) -> anyhow::Result<Self> {
        let switchboard_oracle_queue = match switchboard_cfg {
            SwitchboardConfiguration::Fake => {
                return Ok(Self::Fake {
                    program_id: vrf_program_id,
                })
            }
            SwitchboardConfiguration::Devnet => SWITCHBOARD_LABS_DEVNET_PERMISSIONLESS_QUEUE,
            SwitchboardConfiguration::Mainnet => SWITCHBOARD_LABS_MAINNET_PERMISSIONLESS_QUEUE,
        };

        let switchboard_program_id = switchboard::SWITCHBOARD_PROGRAM_ID;

        let switchboard_queue_data = rpc.get_account_data(&switchboard_oracle_queue).await.unwrap();
        let switchboard_queue_account =
            switchboard::deserialize_oracle_queue_account_data(&switchboard_queue_data).unwrap();

        let (sb_state, _bump) = switchboard::get_program_state_pda(&switchboard_program_id);
        let sb_state_data = rpc.get_account_data(&sb_state).await.unwrap();
        let sb_state_account = switchboard::deserialize_sb_state(&sb_state_data).unwrap();

        let switchboard_oracle_queue_mint = sb_state_account.dao_mint;
        let switchboard_oracle_queue_authority = switchboard_queue_account.authority;
        let switchboard_oracle_queue_data_buffer = switchboard_queue_account.data_buffer;

        Ok(VrfConfiguration::Switchboard {
            program_id: vrf_program_id,
            details: SwitchboardDetails {
                switchboard_program_id,
                switchboard_oracle_queue,
                switchboard_oracle_queue_authority,
                switchboard_oracle_queue_mint,
                switchboard_oracle_queue_data_buffer,
            },
        })
    }
}

use crate::model::winner::EpochWinners;

/// Interface to all On-Chain stuff
/// This is a trait for two reasons
///   1. Make the list of available functions easily scannable
///   2. Allow mocking for tests those need not hit the chain
#[async_trait]
pub trait Solana: Send + Sync {
    fn program_id(&self) -> Pubkey;
    fn admin_keypair(&self) -> Arc<Keypair>;
    fn investor_keypair(&self) -> Arc<Keypair>;
    fn usdc_mint(&self) -> Pubkey;
    fn nez_mint(&self) -> Pubkey;
    fn nezha_vrf_program_id(&self) -> Pubkey;
    fn vrf_configuration(&self) -> VrfConfiguration;

    // Query Epoch Data

    /// Get LatestEpoch account that holds index/pubkey of the current Epoch account
    async fn get_latest_epoch(&self) -> Result<WithPubkey<LatestEpoch>, SolanaError>;
    async fn get_epoch_vrf_request(&self, epoch_index: u64) -> Result<WithPubkey<NezhaVrfRequest>, SolanaError>;
    async fn get_recent_epochs(&self, n: u64) -> Result<Vec<WithPubkey<Epoch>>, SolanaError>;
    async fn get_epoch_by_index(&self, epoch_index: u64) -> Result<WithPubkey<Epoch>, SolanaError>;
    async fn get_epoch_by_pubkey(&self, epoch_pubkey: Pubkey) -> Result<WithPubkey<Epoch>, SolanaError>;
    async fn get_epoch_winners(&self, epoch_index: u64) -> Result<EpochWinners, SolanaError>;

    // Query User Data
    async fn get_stake_by_wallet(&self, wallet: Pubkey) -> Result<Stake, SolanaError>;
    async fn get_prizes_by_wallet(&self, wallet: Pubkey) -> Result<Vec<WalletPrize>, SolanaError>;
    async fn get_stake_update_request_by_wallet(
        &self,
        wallet: Pubkey,
    ) -> Result<Option<StakeUpdateRequest>, SolanaError>;
    async fn get_all_stakes(&self) -> Result<Vec<Stake>, SolanaError>;
    async fn get_all_stake_update_requests(&self) -> Result<Vec<StakeUpdateRequest>, SolanaError>;

    // Epoch state progression

    async fn create_epoch(
        &self,
        epoch_index: u64,
        expected_end_date: DateTime<Utc>,
        yield_split_cfg: YieldSplitCfg,
    ) -> Result<Signature, SolanaError>;
    async fn approve_stake_update(&self, wallet: Pubkey, amount: i64) -> Result<Signature, SolanaError>;
    async fn complete_stake_update(&self, wallet: Pubkey) -> Result<Signature, SolanaError>;
    async fn enter_investment_fake(
        &self,
        epoch_index: u64,
        num_sequences_issued: u64,
    ) -> Result<Signature, SolanaError>;
    async fn exit_investment_fake(&self, epoch_index: u64, amount: FPUSDC) -> Result<Signature, SolanaError>;
    async fn enter_investment_francium(
        &self,
        epoch_index: u64,
        num_sequences_issued: u64,
    ) -> Result<Signature, SolanaError>;
    async fn exit_investment_francium(&self, epoch_index: u64) -> Result<Signature, SolanaError>;
    async fn publish_winners(
        &self,
        epoch_index: u64,
        draw_enabled: bool,
        meta_args: &CreateEpochWinnersMetaArgs,
        winners_input: &[WinnerInput],
    ) -> Result<Signature, SolanaError>;
    // Sets the winning combination on the mock contract.
    async fn set_winning_combination_fake(
        &self,
        epoch_index: u64,
        winning_combination: &[u8; 6],
    ) -> Result<Signature, SolanaError>;
    async fn request_winning_combination(&self) -> Result<Signature, SolanaError>;
    async fn fund_jackpot(&self, epoch_index: u64, amount: FPUSDC) -> Result<Signature, SolanaError>;

    // Custom USDC
    async fn get_usdc_balance_by_wallet(&self, wallet: Pubkey) -> Result<FPUSDC, SolanaError>;
    async fn create_usdc_ata(&self, wallet: Pubkey) -> Result<Signature, SolanaError>;
    async fn mint_usdc(&self, wallet: Pubkey, amount: FPUSDC) -> Result<Signature, SolanaError>;

    // NEZ
    async fn get_nez_balance_by_wallet(&self, wallet: Pubkey) -> Result<FPUSDC, SolanaError>;
}
