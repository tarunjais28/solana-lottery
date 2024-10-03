//! State of an epoch.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_pack::IsInitialized;

use super::{option_max_len, AccountType, ContractVersion, HasAccountType};
use crate::fixed_point::*;

mod status;
pub use status::*;

mod tickets;
pub use tickets::*;

/// State of an epoch.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Epoch {
    pub account_type: AccountType,
    pub contract_version: ContractVersion,
    pub is_initialized: bool,
    pub index: u64,
    pub status: EpochStatus,
    pub yield_split_cfg: YieldSplitCfg,
    pub start_at: i64,
    pub expected_end_at: i64,
    /// set when going into yielding
    pub tickets_info: Option<TicketsInfo>,
    /// set when going into yielding
    pub total_invested: Option<FPUSDC>,
    /// set after investor returns
    pub returns: Option<Returns>,
    /// set after investor returns
    pub draw_enabled: Option<bool>,
    /// set after epoch ends
    pub end_at: Option<i64>,
}

/// See the source of [`crate::processor::investment::withdraw`] and
/// [`crate::processor::investment::returns`] to see how this is used.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct YieldSplitCfg {
    pub jackpot: FPUSDC,
    pub insurance: InsuranceCfg,
    pub treasury_ratio: FixedPoint<3>,
    pub tier2_prize_share: u8,
    pub tier3_prize_share: u8,
}

/// Jackpot insurance configuration.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct InsuranceCfg {
    pub premium: FPUSDC,
    pub probability: FPInternal,
}

impl InsuranceCfg {
    /// Calculate the insurance amount owed for the given number of tickets and jackpot prize.
    pub fn calculate_amount(&self, num_tickets: u64, jackpot: FPUSDC) -> Option<FPUSDC> {
        let premium: FPInternal = self.premium.change_precision();
        let amount: FPInternal = premium
            .checked_mul(num_tickets.into())?
            .checked_mul(jackpot.change_precision())?
            .checked_mul(self.probability)?;
        Some(amount.change_precision())
    }
}

/// How the returns of an epoch are distributed.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Returns {
    pub total: FPUSDC,
    pub deposit_back: FPUSDC,
    pub insurance: FPUSDC,
    pub treasury: FPUSDC,
    pub tier2_prize: FPUSDC,
    pub tier3_prize: FPUSDC,
}

//

impl HasAccountType for Epoch {
    fn account_type() -> AccountType {
        AccountType::Epoch
    }
}

impl IsInitialized for Epoch {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

//

impl Epoch {
    pub const fn max_len() -> usize {
        1 +                                         // account_type: AccountType (u8),
        1 +                                         // contract_version: ContractVersion (u8),
        1 +                                         // is_initialized: bool,
        8 +                                         // index: u64,
        EpochStatus::max_len() +                    // status: EpochStatus,
        YieldSplitCfg::max_len() +                  // yield_split_cfg: YieldSplitCfg,
        8 +                                         // start_at: i64,
        8 +                                         // expected_end_at: i64,
        //
        option_max_len(TicketsInfo::max_len()) +    // tickets_info: Option<TicketsInfo>,
        option_max_len(FPUSDC::max_len()) +         // total_invested: Option<FPUSDC>,
        //
        option_max_len(Returns::max_len()) +        // total_returned: Option<FPUSDC>,
        option_max_len(1) +                         // draw_enabled: Option<bool>,
        //
        option_max_len(8) +                         // end_at: Option<i64>,
        0 // (this line is for formatting)
    }
}

impl YieldSplitCfg {
    pub const fn max_len() -> usize {
        FPUSDC::max_len() +         // jackpot: FPUSDC,
        InsuranceCfg::max_len() +   // insurance: InsuranceCfg,
        FixedPoint::<3>::max_len() + // treasury_ratio: FixedPoint<3>,
        1 +                         // tier2_prize_share: u8,
        1 +                         // tier3_prize_share: u8,
        0 //
    }
}

impl InsuranceCfg {
    pub const fn max_len() -> usize {
        FPUSDC::max_len() + // premium
        FPInternal::max_len() + // probability
        0 //
    }
}

impl Returns {
    pub const fn max_len() -> usize {
        FPUSDC::max_len() +     // total: FPUSDC,
        FPUSDC::max_len() +     // deposit_back: FPUSDC,
        FPUSDC::max_len() +     // insurance: FPUSDC,
        FPUSDC::max_len() +     // treasury: FPUSDC,
        FPUSDC::max_len() +     // tier2_prize: FPUSDC,
        FPUSDC::max_len() +     // tier3_prize: FPUSDC,
        0 //
    }
}

//

#[test]
fn epoch_len() {
    use crate::fixed_point::FixedPoint;
    use crate::state::EPOCH_LEN;

    let mut v = Vec::new();
    let e = Epoch {
        account_type: AccountType::Epoch,
        contract_version: ContractVersion::V1,
        is_initialized: true,
        index: 0,
        status: EpochStatus::Running,
        yield_split_cfg: crate::state::YieldSplitCfg {
            insurance: InsuranceCfg {
                premium: FPUSDC::from(0u8),
                probability: FPInternal::from(0u8),
            },
            jackpot: FPUSDC::from(0u8),
            treasury_ratio: FixedPoint::from(0u8),
            tier2_prize_share: 0u8,
            tier3_prize_share: 0u8,
        },
        start_at: 0,
        expected_end_at: 0,
        //
        tickets_info: Some(TicketsInfo {
            num_tickets: 10,
            tickets_url: String::from_utf8(vec!['a' as u8; TICKETS_URL_MAX_LEN]).unwrap(),
            tickets_hash: vec![0; TICKETS_HASH_MAX_LEN],
            tickets_version: 1,
        }),
        total_invested: Some(0u8.into()),
        //
        returns: Some(Returns {
            total: 0u8.into(),
            deposit_back: 0u8.into(),
            insurance: 0u8.into(),
            treasury: 0u8.into(),
            tier2_prize: 0u8.into(),
            tier3_prize: 0u8.into(),
        }),
        draw_enabled: Some(true),
        //
        end_at: Some(0),
    };
    e.serialize(&mut v).unwrap();
    assert_eq!(v.len(), EPOCH_LEN);
}
