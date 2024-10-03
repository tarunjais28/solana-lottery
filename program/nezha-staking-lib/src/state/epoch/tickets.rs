use borsh::{BorshDeserialize, BorshSerialize};

use crate::state::utils::vec_max_len;

pub const TICKETS_URL_MAX_LEN: usize = 50;
// Longest hash length in common use as of this writing is 512 bits = 64 bytes
pub const TICKETS_HASH_MAX_LEN: usize = 64;

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct TicketsInfo {
    pub num_tickets: u64,
    pub tickets_url: String,
    pub tickets_hash: Vec<u8>,
    pub tickets_version: u8,
}

impl TicketsInfo {
    pub const fn max_len() -> usize {
        8 +                                     // num_tickets: u64
        vec_max_len(1, TICKETS_URL_MAX_LEN) +   // tickets_url: String (TICKETS_URL_MAX_LEN)
        vec_max_len(1, TICKETS_HASH_MAX_LEN) +  // tickets_hash: Vec<u8> (TICKETS_HASH_MAX_LEN)
        1 +                                     // tickets_version: u8
        0 //
    }
}
