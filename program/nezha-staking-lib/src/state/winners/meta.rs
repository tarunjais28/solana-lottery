use crate::fixed_point::FPUSDC;
use crate::state::{AccountType, ContractVersion, HasAccountType};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Summary of winners of an epoch.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct EpochWinnersMeta {
    pub account_type: AccountType,
    pub contract_version: ContractVersion,
    pub is_initialized: bool,
    pub epoch_pubkey: Pubkey,
    pub epoch_index: u64,
    pub tier1_meta: TierWinnersMeta,
    pub tier2_meta: TierWinnersMeta,
    pub tier3_meta: TierWinnersMeta,
    pub total_num_pages: u32,
    pub total_num_winners: u32,
    pub jackpot_claimable: bool,
    pub status: WinnerProcessingStatus,
}

impl HasAccountType for EpochWinnersMeta {
    fn account_type() -> AccountType {
        AccountType::EpochWinnersMeta
    }
}

impl EpochWinnersMeta {
    pub const fn max_len() -> usize {
        1 +                                 // account_type
        1 +                                 // contract_version
        1 +                                 // is_initialized
        32 +                                // epoch_pubkey
        8 +                                 // epoch_index
        TierWinnersMeta::max_len() +        // tier1_meta
        TierWinnersMeta::max_len() +        // tier2_meta
        TierWinnersMeta::max_len() +        // tier3_meta
        4 +                                 // total_num_pages
        4 +                                 // total_num_winners
        1 +                                 // jackpot_claimable
        WinnerProcessingStatus::max_len() + // status
        0 //
    }
}

/// Metadata of a tier.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct TierWinnersMeta {
    /// Total prize of this tier.
    pub total_prize: FPUSDC,
    /// Total number of winners in this tier.
    pub total_num_winners: u32,
    /// Total number of winning tickets in this tier.
    pub total_num_winning_tickets: u32,
}

impl TierWinnersMeta {
    pub const fn max_len() -> usize {
        FPUSDC::max_len() +     // total_prize
        4 +                     // total_num_winners: u32
        4 +                     // total_num_winning_tickets: u32
        0 //
    }
}

/// Status of winners list upload.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum WinnerProcessingStatus {
    Completed,
    InProgress {
        num_pages: u32,
        num_processed_winners: u32,
        tier1_status: TierStatus,
        tier2_status: TierStatus,
        tier3_status: TierStatus,
    },
}

impl WinnerProcessingStatus {
    pub const fn max_len() -> usize {
        1 +                     // discriminator
        4 +                     // num_pages: u32
        4 +                     // num_processed_winners: u32
        TierStatus::max_len() + // tier1_status
        TierStatus::max_len() + // tier2_status
        TierStatus::max_len() + // tier3_status
        0 //
    }
}

/// Per-tier status of winners list upload.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone, Default)]
pub struct TierStatus {
    /// remaining number of winners to be uploaded
    pub rem_num_winners: u32,
    /// remaining number of tickets to be uploaded
    pub rem_num_winning_tickets: u32,
    /// remaining prize to be allocated.
    pub rem_prize: FPUSDC,
}

impl TierStatus {
    pub const fn max_len() -> usize {
        4 +                         // rem_num_winners: u32
        4 +                         // rem_num_winning_tickets: u32
        FPUSDC::max_len() +         // rem_prize
        0 //
    }
}

#[test]
fn test_max_len_epoch_winners_meta() {
    use crate::fixed_point::FixedPoint;
    use crate::state::EPOCH_WINNERS_META_LEN;

    let tier_meta = TierWinnersMeta {
        total_prize: FixedPoint::zero(),
        total_num_winners: 0,
        total_num_winning_tickets: 0,
    };

    let tier_status = TierStatus {
        rem_num_winners: 0,
        rem_num_winning_tickets: 0,
        rem_prize: FixedPoint::zero(),
    };

    let mut v = Vec::new();
    EpochWinnersMeta {
        account_type: AccountType::EpochWinnersPage,
        contract_version: ContractVersion::V1,
        is_initialized: true,
        epoch_pubkey: Pubkey::new_unique(),
        epoch_index: 0,
        tier1_meta: tier_meta.clone(),
        tier2_meta: tier_meta.clone(),
        tier3_meta: tier_meta.clone(),
        total_num_pages: 0,
        total_num_winners: 0,
        jackpot_claimable: false,
        status: WinnerProcessingStatus::InProgress {
            num_pages: 0,
            num_processed_winners: 0,
            tier1_status: tier_status.clone(),
            tier2_status: tier_status.clone(),
            tier3_status: tier_status.clone(),
        },
    }
    .serialize(&mut v)
    .unwrap();
    assert_eq!(v.len(), EPOCH_WINNERS_META_LEN);
}
