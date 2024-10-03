//! Global state of the program.

use borsh::{BorshDeserialize, BorshSerialize};
use nezha_utils::impl_borsh_length_struct;
use solana_program::pubkey::Pubkey;

use crate::accounts::AccountType;
use crate::impl_has_account_type;

use super::ContractVersion;

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct NezhaVrfProgramState {
    pub account_type: AccountType,
    pub contract_version: ContractVersion,
    pub pubkeys: Pubkeys,
}

impl_has_account_type!(NezhaVrfProgramState, AccountType::NezhaVrfProgramState);

impl_borsh_length_struct!(
    NezhaVrfProgramState,
    /* account_type: */ AccountType,
    /* contract_version: */ ContractVersion,
    /* pubkeys: */ Pubkeys
);

/// Pubkeys used for authentication.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Pubkeys {
    pub super_admin: Pubkey,
    pub admin: Pubkey,
    pub switchboard_program_id: Pubkey,
    pub nezha_staking_program_id: Pubkey,
}

impl_borsh_length_struct! {
    Pubkeys,
    /* super_admin: */ Pubkey,
    /* admin: */ Pubkey,
    /* switchboard_program_id: */ Pubkey,
    /* nezha_staking_program_id: */ Pubkey
}

#[test]
fn test_borsh_len() {
    use super::HasAccountType;
    use nezha_utils::borsh_length::BorshLength;

    let mut v = Vec::new();
    NezhaVrfProgramState {
        account_type: NezhaVrfProgramState::account_type(),
        contract_version: super::CONTRACT_VERSION,
        pubkeys: Pubkeys {
            super_admin: Pubkey::new_unique(),
            admin: Pubkey::new_unique(),
            switchboard_program_id: Pubkey::new_unique(),
            nezha_staking_program_id: Pubkey::new_unique(),
        },
    }
    .serialize(&mut v)
    .unwrap();

    assert_eq!(v.len(), NezhaVrfProgramState::borsh_length());
}
