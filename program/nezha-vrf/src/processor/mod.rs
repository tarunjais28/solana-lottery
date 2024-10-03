//! Processor functions.

pub mod rotate_key;
pub mod switchboard;

use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{instruction::*, state::*};

pub fn process_instruction<'a>(program_id: &Pubkey, accounts: &[AccountInfo<'a>], input: &[u8]) -> ProgramResult {
    let instruction = NezhaVrfInstruction::try_from_slice(input)?;
    match instruction {
        NezhaVrfInstruction::Init {
            switchboard_program_state_pda_bump,
        } => switchboard::process_init(program_id, accounts, switchboard_program_state_pda_bump),
        NezhaVrfInstruction::RequestVRF { epoch_index } => {
            switchboard::process_request_vrf(program_id, accounts, epoch_index)
        }
        NezhaVrfInstruction::ConsumeVRF { epoch_index } => {
            switchboard::process_consume_vrf(program_id, accounts, epoch_index)
        }
        NezhaVrfInstruction::RotateKey { key_type } => rotate_key::process_rotate_key(program_id, accounts, key_type),
        _ => unreachable!(),
    }
}
