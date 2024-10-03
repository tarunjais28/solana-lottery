#![allow(unused)]

use std::collections::BTreeMap;

use async_trait::async_trait;
use nezha_staking::fixed_point::FPInternal;
use nezha_staking::state::{CumulativeReturnRate, EpochStatus, InsuranceCfg, LatestEpoch, PendingFunds, Pubkeys};
use solana_program::pubkey::Pubkey;

use super::*;
use super::{with_pubkey::WithPubkey, Solana, SolanaError, Stake, WalletPrize};

#[derive(Clone)]
pub struct SolanaMock {
    pub stakes: Vec<Stake>,
    pub epoch_index: u64,
    pub epoch_status: EpochStatus,
}

impl SolanaMock {
    pub fn new() -> Self {
        Self {
            stakes: Vec::new(),
            epoch_index: 0,
            epoch_status: EpochStatus::Ended,
        }
    }
}

#[async_trait]
impl Solana for SolanaMock {
    async fn request_winning_combination(&self) -> Result<Signature, SolanaError> {
        unimplemented!()
    }
    async fn get_epoch_vrf_request(&self, epoch_index: u64) -> Result<WithPubkey<NezhaVrfRequest>, SolanaError> {
        unimplemented!()
    }
    fn vrf_configuration(&self) -> VrfConfiguration {
        unimplemented!()
    }
    async fn set_winning_combination_fake(
        &self,
        epoch_index: u64,
        winning_combination: &[u8; 6],
    ) -> Result<Signature, SolanaError> {
        unimplemented!()
    }
    fn nezha_vrf_program_id(&self) -> Pubkey {
        unimplemented!()
    }

    fn program_id(&self) -> Pubkey {
        todo!()
    }

    fn admin_keypair(&self) -> Arc<Keypair> {
        todo!()
    }

    fn investor_keypair(&self) -> Arc<Keypair> {
        todo!()
    }

    fn usdc_mint(&self) -> Pubkey {
        todo!()
    }

    fn nez_mint(&self) -> Pubkey {
        todo!()
    }

    // Query Epoch Data
    /// Get LatestEpoch account that holds index/pubkey of the current Epoch account
    async fn get_latest_epoch(&self) -> Result<WithPubkey<LatestEpoch>, SolanaError> {
        Ok(WithPubkey {
            pubkey: Pubkey::new_unique(),
            inner: LatestEpoch {
                account_type: nezha_staking::state::AccountType::LatestEpoch,
                contract_version: nezha_staking::state::ContractVersion::V1,
                is_initialized: true,
                index: self.epoch_index,
                status: self.epoch_status,
                epoch: Pubkey::new_unique(),
                cumulative_return_rate: CumulativeReturnRate::unity(),
                pending_funds: PendingFunds {
                    insurance: 0u8.into(),
                    tier2_prize: 0u8.into(),
                    tier3_prize: 0u8.into(),
                },
                pubkeys: Pubkeys {
                    super_admin: Pubkey::new_unique(),
                    admin: Pubkey::new_unique(),
                    investor: Pubkey::new_unique(),
                    nezha_vrf_program_id: Pubkey::new_unique(),
                },
            },
        })
    }

    async fn get_recent_epochs(&self, n: u64) -> Result<Vec<WithPubkey<Epoch>>, SolanaError> {
        todo!()
    }

    async fn get_epoch_by_index(&self, epoch_index: u64) -> Result<WithPubkey<Epoch>, SolanaError> {
        Ok(WithPubkey {
            pubkey: Pubkey::new_unique(),
            inner: Epoch {
                account_type: nezha_staking::state::AccountType::Epoch,
                contract_version: nezha_staking::state::ContractVersion::V1,
                is_initialized: true,
                index: self.epoch_index,
                status: self.epoch_status,
                yield_split_cfg: YieldSplitCfg {
                    jackpot: "1".parse().unwrap(),
                    insurance: InsuranceCfg {
                        premium: "1".parse().unwrap(),
                        probability: "1".parse().unwrap(),
                    },
                    treasury_ratio: "0.5".parse().unwrap(),
                    tier2_prize_share: 1,
                    tier3_prize_share: 1,
                },
                start_at: 0,
                expected_end_at: 0,
                tickets_info: None,
                total_invested: None,
                returns: None,
                draw_enabled: None,
                end_at: None,
            },
        })
    }

    async fn get_epoch_by_pubkey(&self, epoch_pubkey: Pubkey) -> Result<WithPubkey<Epoch>, SolanaError> {
        todo!()
    }

    async fn get_epoch_winners(&self, epoch_index: u64) -> Result<EpochWinners, SolanaError> {
        todo!()
    }

    // Query User Data
    async fn get_stake_by_wallet(&self, wallet: Pubkey) -> Result<Stake, SolanaError> {
        self.stakes
            .iter()
            .find(|s| s.owner == wallet)
            .cloned()
            .context("No stake for wallet")
    }

    async fn get_prizes_by_wallet(&self, wallet: Pubkey) -> Result<Vec<WalletPrize>, SolanaError> {
        todo!()
    }
    async fn get_stake_update_request_by_wallet(
        &self,
        wallet: Pubkey,
    ) -> Result<Option<StakeUpdateRequest>, SolanaError> {
        todo!()
    }
    async fn get_all_stakes(&self) -> Result<Vec<Stake>, SolanaError> {
        todo!()
    }
    async fn get_all_stake_update_requests(&self) -> Result<Vec<StakeUpdateRequest>, SolanaError> {
        todo!()
    }
    // Epoch state progression
    async fn create_epoch(
        &self,
        epoch_index: u64,
        expected_end_date: DateTime<Utc>,
        yield_split_cfg: YieldSplitCfg,
    ) -> Result<Signature, SolanaError> {
        todo!()
    }
    async fn approve_stake_update(&self, wallet: Pubkey, amount: i64) -> Result<Signature, SolanaError> {
        todo!()
    }
    async fn complete_stake_update(&self, wallet: Pubkey) -> Result<Signature, SolanaError> {
        todo!()
    }
    async fn enter_investment_fake(
        &self,
        epoch_index: u64,
        num_sequences_issued: u64,
    ) -> Result<Signature, SolanaError> {
        todo!()
    }
    async fn exit_investment_fake(&self, epoch_index: u64, amount: FPUSDC) -> Result<Signature, SolanaError> {
        todo!()
    }
    async fn enter_investment_francium(
        &self,
        epoch_index: u64,
        num_sequences_issued: u64,
    ) -> Result<Signature, SolanaError> {
        todo!()
    }
    async fn exit_investment_francium(&self, epoch_index: u64) -> Result<Signature, SolanaError> {
        todo!()
    }
    async fn publish_winners(
        &self,
        epoch_index: u64,
        draw_enabled: bool,
        meta_args: &CreateEpochWinnersMetaArgs,
        winners_input: &[WinnerInput],
    ) -> Result<Signature, SolanaError> {
        todo!()
    }
    async fn fund_jackpot(&self, epoch_index: u64, amount: FPUSDC) -> Result<Signature, SolanaError> {
        todo!()
    }

    async fn get_usdc_balance_by_wallet(&self, wallet: Pubkey) -> Result<FPUSDC, SolanaError> {
        todo!()
    }
    async fn create_usdc_ata(&self, wallet: Pubkey) -> Result<Signature, SolanaError> {
        todo!()
    }
    async fn mint_usdc(&self, wallet: Pubkey, amount: FPUSDC) -> Result<Signature, SolanaError> {
        todo!()
    }
    async fn get_nez_balance_by_wallet(&self, wallet: Pubkey) -> Result<FPUSDC, SolanaError> {
        todo!()
    }
}
