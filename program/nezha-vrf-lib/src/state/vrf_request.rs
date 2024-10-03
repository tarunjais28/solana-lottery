//! VRF Request for an epoch

use borsh::{BorshDeserialize, BorshSerialize};
use nezha_utils::{impl_borsh_length, impl_borsh_length_struct};

use crate::accounts::AccountType;
use crate::impl_has_account_type;

use super::ContractVersion;

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct NezhaVrfRequest {
    pub account_type: AccountType,
    pub contract_version: ContractVersion,
    pub status: NezhaVrfRequestStatus,
    pub vrf_counter: u128,
    pub winning_combination: Option<[u8; 6]>,
    pub request_start: i64,
    pub request_end: Option<i64>,
}

impl_has_account_type!(NezhaVrfRequest, AccountType::NezhaVrfRequest);


impl_borsh_length_struct!(
    NezhaVrfRequest,
    /* account_type: */ AccountType,
    /* contract_version: */ ContractVersion,
    /* status: */ NezhaVrfRequestStatus,
    /* vrf_counter: */ u128,
    /* winning_combination: */ Option<[u8; 6]>,
    /* request_start: */ i64,
    /* request_end: */ Option<i64>
);

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum NezhaVrfRequestStatus {
    Waiting,
    Success,
    Fail,
}

impl_borsh_length!(NezhaVrfRequestStatus, 1);

#[test]
fn test_borsh_len() {
    use super::HasAccountType;
    use nezha_utils::borsh_length::BorshLength;

    let mut v = Vec::new();
    NezhaVrfRequest {
        account_type: NezhaVrfRequest::account_type(),
        contract_version: super::CONTRACT_VERSION,
        vrf_counter: 0,
        status: NezhaVrfRequestStatus::Waiting,
        winning_combination: Some([0u8; 6]),
        request_start: 0,
        request_end: Some(0),
    }
    .serialize(&mut v)
    .unwrap();

    assert_eq!(v.len(), NezhaVrfRequest::borsh_length());
}
