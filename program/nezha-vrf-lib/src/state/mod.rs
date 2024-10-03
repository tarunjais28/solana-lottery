//! State account structs.

#[macro_use]
mod has_account_type;
pub use has_account_type::*;

mod program_state;
pub use program_state::*;

mod vrf_request;
pub use vrf_request::*;

use borsh::{BorshDeserialize, BorshSerialize};
use nezha_utils::impl_borsh_length;

/// For migration of accounts in the future.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum ContractVersion {
    V1,
}

impl_borsh_length!(ContractVersion, 1);

pub const CONTRACT_VERSION: ContractVersion = ContractVersion::V1;
