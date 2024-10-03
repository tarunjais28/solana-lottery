pub use nezha_staking::instruction::WinnerInput;
pub use nezha_staking::state::{TierWinnersMeta, Winner};

#[derive(Debug, Clone, PartialEq)]
pub struct EpochWinners {
    pub epoch_index: u64,
    pub tier1_meta: TierWinnersMeta,
    pub tier2_meta: TierWinnersMeta,
    pub tier3_meta: TierWinnersMeta,
    pub jackpot_claimable: bool,
    pub winners: Vec<Winner>,
}
