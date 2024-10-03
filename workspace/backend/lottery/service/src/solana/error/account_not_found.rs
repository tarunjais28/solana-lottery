use nezha_staking::state::AccountType;
use solana_program::pubkey::Pubkey;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AccountNotFound {
    #[error("Epoch not found: (index {index:?}) {pubkey}")]
    Epoch { pubkey: Pubkey, index: Option<u64> },
    #[error("LatestEpoch not found")]
    LatestEpoch { pubkey: Pubkey },
    #[error("EpochWinners meta not found: (epoch index {epoch_index:?}) {pubkey}")]
    EpochWinnersMeta { epoch_index: u64, pubkey: Pubkey },
    #[error("EpochWinnersPage not found: (epoch index {epoch_index:?}) (page_index {page_index:?}) {pubkey}")]
    EpochWinnersPage {
        epoch_index: u64,
        page_index: u32,
        pubkey: Pubkey,
    },
    #[error("NezhaVrfRequest not found: (epoch index {epoch_index:?}) {pubkey}, program_id: {program_id}")]
    NezhaVrfRequest {
        program_id: Pubkey,
        epoch_index: u64,
        pubkey: Pubkey,
    },
    #[error("Stake not found: (wallet {wallet:?}) {pubkey}")]
    Stake { wallet: Pubkey, pubkey: Pubkey },
}

impl AccountNotFound {
    pub fn is_account(&self, account_type: AccountType) -> bool {
        match self {
            AccountNotFound::Epoch { .. } => account_type == AccountType::Epoch,
            AccountNotFound::LatestEpoch { .. } => account_type == AccountType::LatestEpoch,
            AccountNotFound::Stake { .. } => account_type == AccountType::Stake,
            AccountNotFound::EpochWinnersMeta { .. } => account_type == AccountType::EpochWinnersMeta,
            AccountNotFound::EpochWinnersPage { .. } => account_type == AccountType::EpochWinnersPage,
            AccountNotFound::NezhaVrfRequest {
                program_id,
                epoch_index,
                pubkey,
            } => account_type == AccountType::NezhaVrfRequest,
        }
    }
}
