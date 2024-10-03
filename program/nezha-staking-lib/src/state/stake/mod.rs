//! Stake account.

mod balance;
pub use balance::*;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{program_pack::IsInitialized, pubkey::Pubkey};

use super::{AccountType, ContractVersion, HasAccountType};

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Stake {
    pub account_type: AccountType,
    pub contract_version: ContractVersion,
    pub is_initialized: bool,
    pub owner: Pubkey,
    /// See [`stake_calculation`](super::stake_calculation).
    pub balance: FloatingBalance,
    /// The index of the epoch, in which this account was created.
    pub created_epoch_index: u64,
    /// The index of the epoch, in which this account was last updated.
    pub updated_epoch_index: u64,
}

impl HasAccountType for Stake {
    fn account_type() -> AccountType {
        AccountType::Stake
    }
}

impl IsInitialized for Stake {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Stake {
    pub const fn max_len() -> usize {
        1 +                             // account_type: AccountType (u8),
        1 +                             // contract_version: ContractVersion (u8),
        1 +                             // is_initialized: bool
        32 +                            // owner: Pubkey
        FloatingBalance::max_len() +    // balance: FloatingBalance
        8 +                             // created_epoch_index: u64
        8 +                             // updated_epoch_index: u64
        0
    }
}

#[test]
fn test_max_len() {
    use crate::fixed_point::FixedPoint;
    use crate::state::CumulativeReturnRate;
    use crate::state::STAKE_LEN;

    let mut v = Vec::new();
    Stake {
        account_type: AccountType::Stake,
        contract_version: ContractVersion::V1,
        is_initialized: true,
        owner: Pubkey::new_unique(),
        balance: FloatingBalance::new(FixedPoint::zero(), CumulativeReturnRate::unity()),
        created_epoch_index: 0,
        updated_epoch_index: 0,
    }
    .serialize(&mut v)
    .unwrap();
    assert_eq!(v.len(), STAKE_LEN);
}
