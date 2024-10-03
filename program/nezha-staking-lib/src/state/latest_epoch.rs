//! Global state of the program.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{program_pack::IsInitialized, pubkey::Pubkey};

use super::{AccountType, ContractVersion, EpochStatus, HasAccountType};
use crate::{fixed_point::*, state::CumulativeReturnRate};

/// Ideally this struct should have been named ProgramState.
/// This contains global data belonging to the program.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct LatestEpoch {
    pub account_type: AccountType,
    pub contract_version: ContractVersion,
    pub is_initialized: bool,
    pub index: u64,
    pub status: EpochStatus,
    pub epoch: Pubkey,
    /// See [`stake_calculation`](crate::state::stake_calculation)
    pub cumulative_return_rate: CumulativeReturnRate,
    pub pending_funds: PendingFunds,
    pub pubkeys: Pubkeys,
}

impl HasAccountType for LatestEpoch {
    fn account_type() -> AccountType {
        AccountType::LatestEpoch
    }
}

impl IsInitialized for LatestEpoch {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl LatestEpoch {
    pub const fn max_len() -> usize {
        1 +                                 // account_type: AccountType (u8),
        1 +                                 // contract_version: ContractVersion (u8),
        1 +                                 // is_initialized: bool
        8 +                                 // index: u64
        1 +                                 // status: EpochStatus
        32 +                                // epoch: Pubkey
        CumulativeReturnRate::max_len() +   // cumulative_return_rate: FPInternal
        PendingFunds::max_len() +           // pending_funds: PendingFunds,
        Pubkeys::max_len() +                // pubkeys: Pubkeys,
        0
    }
}

/// Funds which have been distributed but not used yet.
/// For example, we allocated 20% of the yield to tier 2 prize, but there are no tier2 winners this
/// epoch.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone, Default)]
pub struct PendingFunds {
    pub insurance: FPUSDC,
    pub tier2_prize: FPUSDC,
    pub tier3_prize: FPUSDC,
}

impl PendingFunds {
    pub const fn max_len() -> usize {
        FPUSDC::max_len() + // insurance: FPUSDC,
        FPUSDC::max_len() + // tier2_prize: FPUSDC,
        FPUSDC::max_len() + // tier3_prize: FPUSDC,
        0
    }
}

/// Pubkeys used for authentication.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone, Default)]
pub struct Pubkeys {
    pub super_admin: Pubkey,
    pub admin: Pubkey,
    pub investor: Pubkey,
    pub nezha_vrf_program_id: Pubkey,
}

impl Pubkeys {
    pub const fn max_len() -> usize {
        32 + // super_admin: Pubkey,
        32 + // admin: Pubkey,
        32 + // investor: Pubkey,
        32 + // nezha_vrf_program_id: Pubkey
        0
    }
}

#[test]
fn test_max_len() {
    use crate::state::LATEST_EPOCH_LEN;

    let mut v = Vec::new();
    LatestEpoch {
        account_type: AccountType::LatestEpoch,
        contract_version: ContractVersion::V1,
        is_initialized: true,
        index: 0,
        status: EpochStatus::Running,
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
    }
    .serialize(&mut v)
    .unwrap();
    assert_eq!(v.len(), LATEST_EPOCH_LEN);
}
