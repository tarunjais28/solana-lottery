use anyhow::Result;
use async_trait::async_trait;
use nezha_staking::fixed_point::FPUSDC;
use solana_program::pubkey::Pubkey;

use super::{Ticket, TicketsWithCount, WalletRisqId, Winners};
use crate::solana::Stake;

#[async_trait]
pub trait TicketService: Sync + Send {
    async fn read_ticket_by_wallet_and_epoch_index(&self, wallet: &Pubkey, index: u64) -> Result<Option<Ticket>>;
    async fn read_tickets_by_epoch_index_and_prefix(
        &self,
        index: u64,
        limit: u8,
        prefix: &[u8],
    ) -> Result<TicketsWithCount>;
    async fn generate_ticket_for_stake(&self, stake: &Stake, epoch_index: Option<u64>) -> Result<Ticket>;
    async fn generate_ticket_for_wallet(&self, wallet: &Pubkey, epoch_index: Option<u64>) -> Result<Ticket>;
    async fn generate_tickets_for_all(&self) -> Result<Vec<Result<Ticket>>>;
    async fn update_arweave_url(&self, wallet: &Pubkey, index: u64, arweave_url: String) -> Result<Option<Ticket>>;
    async fn get_unsubmitted_tickets_in_epoch(&self, epoch_index: u64) -> Result<Vec<Ticket>>;
    async fn update_risq_ids(&self, epoch_index: u64, risq_ids: &[WalletRisqId]) -> Result<Vec<Ticket>>;
    async fn calculate_winners(&self) -> Result<Winners>;
    async fn calculate_optimal_winning_combination(&self) -> Result<Option<[u8; 6]>>;
    async fn random_winning_combination(&self) -> Result<Option<[u8; 6]>>;
    async fn ticket_price(&self) -> Result<FPUSDC>;
    async fn draws_played_by_wallet(&self, wallet: &Pubkey) -> Result<u64>;
    // Bonus info
    async fn num_signup_bonus_sequences(&self, wallet: &Pubkey, amount: FPUSDC) -> Result<u32>;
}
