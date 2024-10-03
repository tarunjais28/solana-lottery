use crate::fixed_point::FPUSDC;
use crate::state::{utils::vec_max_len, AccountType, ContractVersion, HasAccountType};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Max number of winners of that fits in a page.
/// Empirically calculated to not overflow the max transaction size.
pub const MAX_NUM_WINNERS_PER_PAGE: usize = 10;

/// List of winners of an epoch, divided into pages.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct EpochWinnersPage {
    pub account_type: AccountType,
    pub contract_version: ContractVersion,
    pub is_initialized: bool,
    pub page_index: u32,
    pub winners: Vec<Winner>,
}

impl HasAccountType for EpochWinnersPage {
    fn account_type() -> AccountType {
        AccountType::EpochWinnersPage
    }
}

impl EpochWinnersPage {
    pub const fn max_len() -> usize {
        1 +                 // account_type: AccountType (u8),
        1 +                 // contract_version: ContractVersion (u8),
        1 +                 // is_initialized: bool
        4 +                 // page_index: u32
        vec_max_len(
            Winner::max_len(),
            MAX_NUM_WINNERS_PER_PAGE
        ) +                 // winners: Vec<Winner>
        0 // (this line is for formatting)
    }
}

/// Winner details.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Winner {
    /// Index of this winner in the page.
    pub index: u32,
    /// Owner pubkey.
    pub address: Pubkey,
    /// Tier of prize.
    pub tier: u8,
    /// Prize amount.
    pub prize: FPUSDC,
    /// Is this prize claimed.
    pub claimed: bool,
}

impl Winner {
    pub const fn max_len() -> usize {
        4 +                 // index: u32
        32 +                // address: Pubkey
        1 +                 // tier: u8,
        FPUSDC::max_len() + // prize: FPUSDC
        1 +                 // claimed: bool
        0 // (this line is for formatting)
    }
}

#[test]
fn test_max_len_epoch_winners_page() {
    use crate::state::EPOCH_WINNERS_PAGE_LEN;

    let mut v = Vec::new();
    EpochWinnersPage {
        account_type: AccountType::EpochWinnersPage,
        contract_version: ContractVersion::V1,
        is_initialized: true,
        page_index: 0,
        winners: std::iter::repeat(Winner {
            index: 0,
            prize: 0u8.into(),
            address: Pubkey::new_unique(),
            tier: 0,
            claimed: true,
        })
        .take(MAX_NUM_WINNERS_PER_PAGE)
        .collect(),
    }
    .serialize(&mut v)
    .unwrap();
    assert_eq!(v.len(), EPOCH_WINNERS_PAGE_LEN);
}
