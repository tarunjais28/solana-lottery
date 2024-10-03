//! State account structs.

mod utils;

use borsh::{BorshDeserialize, BorshSerialize};
use utils::*;

pub mod epoch;
pub mod latest_epoch;
pub mod return_rate;
pub mod stake;
pub mod stake_calculation;
pub mod stake_update;
pub mod winners;

pub use epoch::*;
pub use latest_epoch::*;
pub use return_rate::*;
pub use stake::*;
pub use stake_update::*;
pub use winners::*;

pub use crate::accounts::AccountType;

// Max-lengths of the account structs
// Actual length of the data may be less, but we use the max length for allocating accounts, so
// that we won't have to resize the account if the value changes.
// For example:
// Option<u64> will take just 1 byte if it's None, but will take 1 + 8 bytes if it's Some(u64).
pub const STAKE_LEN: usize = Stake::max_len();
pub const STAKE_UPDATE_REQUEST_LEN: usize = StakeUpdateRequest::max_len();
pub const EPOCH_LEN: usize = Epoch::max_len();
pub const LATEST_EPOCH_LEN: usize = LatestEpoch::max_len();
pub const EPOCH_WINNERS_META_LEN: usize = EpochWinnersMeta::max_len();
pub const EPOCH_WINNERS_PAGE_LEN: usize = EpochWinnersPage::max_len();

/// Used to attach an AccountType value with an account struct.
/// For example, can be used to implement validation logic inside
///     `fn decode<T: AccountType>(data: &[u8])`.
pub trait HasAccountType {
    fn account_type() -> AccountType;
}

/// For migration of accounts in the future.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum ContractVersion {
    V1,
}
