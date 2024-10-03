//! Deposit/withdraw requests.

use borsh::{BorshDeserialize, BorshSerialize};
use num_derive::FromPrimitive;
use solana_program::{program_pack::IsInitialized, pubkey::Pubkey};

use super::{AccountType, ContractVersion, HasAccountType};

/// Deposit/withdraw request.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct StakeUpdateRequest {
    pub account_type: AccountType,
    pub contract_version: ContractVersion,
    pub is_initialized: bool,
    pub owner: Pubkey,
    /// Negative means withdrawal, positive means deposit. Zero is not allowed.
    pub amount: i64,
    pub state: StakeUpdateState,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Copy, Clone, FromPrimitive)]
pub enum StakeUpdateState {
    /// Waiting for anti money laundering (AML) check by the admin.
    PendingApproval,
    /// Approved by the admin.
    /// Waiting for the epoch to be in the Running state, and the funds to be available after
    /// moving out of the investment platform.
    Queued,
}

impl StakeUpdateState {
    pub fn as_display(&self) -> &'static str {
        match self {
            StakeUpdateState::PendingApproval => "PendingApproval",
            StakeUpdateState::Queued => "Queued",
        }
    }
}

impl HasAccountType for StakeUpdateRequest {
    fn account_type() -> AccountType {
        AccountType::StakeUpdateRequest
    }
}

impl IsInitialized for StakeUpdateRequest {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl StakeUpdateRequest {
    pub const fn max_len() -> usize {
        1 +     // account_type: AccountType (u8),
        1 +     // contract_version: ContractVersion (u8),
        1 +     // is_initialized: bool
        32 +    // owner: Pubkey
        8 +     // amount: u64
        1 +     // state: StakeUpdateState
        0
    }
}

#[test]
fn test_max_len() {
    use crate::state::STAKE_UPDATE_REQUEST_LEN;

    let mut v = Vec::new();
    StakeUpdateRequest {
        account_type: AccountType::StakeUpdateRequest,
        contract_version: ContractVersion::V1,
        is_initialized: true,
        owner: Pubkey::new_unique(),
        amount: 0,
        state: StakeUpdateState::PendingApproval,
    }
    .serialize(&mut v)
    .unwrap();
    assert_eq!(v.len(), STAKE_UPDATE_REQUEST_LEN);
}
