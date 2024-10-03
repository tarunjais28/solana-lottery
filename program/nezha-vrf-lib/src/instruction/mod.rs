//! Instructions.
use borsh::{BorshDeserialize, BorshSerialize};

/// Instruction creation functions.
mod fns;
pub use fns::*;

/// The instructions of the contract.
/// Please see the instruction creation functions to see what each instruction does and the
/// accounts needed by each instruction.
///
/// RemovedX are placeholders for old instructions that no longer exists.
/// They are kept there so that the instruction number of other instructions won't change.
/// Executing them will return `NezhaVrfError::InvalidInstruction`.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub enum NezhaVrfInstruction {
    Init {
        switchboard_program_state_pda_bump: u8,
    },
    RequestVRF {
        epoch_index: u64,
    },
    ConsumeVRF {
        epoch_index: u64,
    },
    RotateKey {
        key_type: RotateKeyType,
    },
    MockSetWinningCombination {
        epoch_index: u64,
        winning_combination: [u8; 6],
    },
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone, Copy)]
pub enum RotateKeyType {
    SuperAdmin,
    Admin,
}
