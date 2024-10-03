use nezha_staking::fixed_point::FPUSDC;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prize {
    pub wallet: Pubkey,
    pub epoch_index: u64,
    pub page: u32,
    pub winner_index: u32,
    pub tier: u8,
    pub amount: FPUSDC,
    pub claimable: bool,
    pub claimed: bool,
}

impl Ord for Prize {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.wallet
            .cmp(&other.wallet)
            .then(self.epoch_index.cmp(&other.epoch_index))
            .then(self.page.cmp(&other.page))
            .then(self.winner_index.cmp(&other.winner_index))
    }
}

impl PartialOrd for Prize {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
