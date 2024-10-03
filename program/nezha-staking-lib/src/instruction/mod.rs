//! Instructions.
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::accounts as ac;

use crate::state::*;

/// Instruction creation functions.
mod fns;
pub use fns::*;

/// The instructions of the contract.
/// Please see the instruction creation functions to see what each instruction does and the
/// accounts needed by each instruction.
///
/// RemovedX are placeholders for old instructions that no longer exists.
/// They are kept there so that the instruction number of other instructions won't change.
/// Executing them will return `StakingError::RemovedInstruction`.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub enum StakingInstruction {
    Init,
    RequestStakeUpdate {
        amount: i64,
    },
    ApproveStakeUpdate {
        amount: i64,
    },
    CancelStakeUpdate {
        amount: i64,
    },
    Removed3,
    Removed4,
    CreateEpoch {
        expected_end_at: i64,
        yield_split_cfg: YieldSplitCfg,
    },
    Removed1,
    Removed5,
    ClaimWinning {
        epoch_index: u64,
        page: u32,
        winner_index: u32,
        tier: u8,
    },
    YieldWithdrawByInvestor {
        tickets_info: TicketsInfo,
    },
    YieldDepositByInvestor {
        return_amount: u64,
    },
    FundJackpot {
        epoch_index: u64,
    },
    Removed2,
    FranciumInit,
    FranciumInvest {
        tickets_info: TicketsInfo,
    },
    FranciumWithdraw,
    WithdrawVault {
        vault: WithdrawVault,
        amount: u64,
    },
    CompleteStakeUpdate,
    CreateEpochWinnersMeta {
        meta_args: CreateEpochWinnersMetaArgs,
    },
    PublishWinners {
        page_index: u32,
        winners_input: Vec<WinnerInput>,
    },
    RotateKey {
        key_type: RotateKeyType,
    },
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct WinnerInput {
    pub index: u32,
    pub address: Pubkey,
    pub tier: u8,
    pub num_winning_tickets: u32,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct TierWinnersMetaInput {
    pub total_num_winners: u32,
    pub total_num_winning_tickets: u32,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct CreateEpochWinnersMetaArgs {
    pub tier1_meta: TierWinnersMetaInput,
    pub tier2_meta: TierWinnersMetaInput,
    pub tier3_meta: TierWinnersMetaInput,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone, Copy)]
pub enum WithdrawVault {
    Insurance,
    Treasury,
}

impl WithdrawVault {
    pub fn get_pda(self, program_id: &Pubkey) -> ac::PDA {
        match self {
            WithdrawVault::Insurance => ac::insurance_vault(program_id),
            WithdrawVault::Treasury => ac::treasury_vault(program_id),
        }
    }
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone, Copy)]
pub enum RotateKeyType {
    SuperAdmin,
    Admin,
    Investor,
}
