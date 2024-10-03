use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    sysvar::{clock, recent_blockhashes, rent},
};
use spl_associated_token_account::get_associated_token_address;

use nezha_utils::account_meta;

use super::*;
use crate::accounts as ac;
use crate::switchboard::{get_permission_pda, get_program_state_pda};

pub fn init(
    program_id: &Pubkey,
    super_admin: &Pubkey,
    admin: &Pubkey,
    switchboard_program_id: &Pubkey,
    queue: &Pubkey,
    queue_authority: &Pubkey,
    queue_mint: &Pubkey,
    nezha_staking_program_id: &Pubkey,
) -> Instruction {
    let vrf_lite = ac::switchboard_vrf_lite(program_id).pubkey;
    let escrow = get_associated_token_address(&vrf_lite, queue_mint);
    let (permission_pda, _permission_pda_bump) =
        get_permission_pda(switchboard_program_id, queue_authority, queue, &vrf_lite);
    let (program_state_pda, program_state_pda_bump) = get_program_state_pda(switchboard_program_id);

    Instruction::new_with_borsh(
        *program_id,
        &NezhaVrfInstruction::Init {
            switchboard_program_state_pda_bump: program_state_pda_bump,
        },
        account_meta![
            [signer] *super_admin,
            [signer writable] *admin,
            [writable] ac::nezha_vrf_program_state(program_id).pubkey,
            //
            [] *switchboard_program_id,
            [] ac::switchboard_authority(program_id).pubkey,
            [writable] vrf_lite,
            [] *queue_mint,
            [writable] escrow,
            [] *queue_authority,
            [] *queue,
            [writable] permission_pda,
            [] program_state_pda,
            //
            [] *nezha_staking_program_id,
            //
            [] spl_token::id(),
            [] spl_associated_token_account::id(),
            [] system_program::id(),
            [] rent::id(),
        ],
    )
}

pub fn request_vrf(
    program_id: &Pubkey,
    admin: &Pubkey,
    switchboard_program_id: &Pubkey,
    switchboard_oracle_queue: &Pubkey,
    switchboard_oracle_queue_authority: &Pubkey,
    switchboard_oracle_queue_mint: &Pubkey,
    switchboard_oracle_queue_data_buffer: &Pubkey,
    nezha_staking_latest_epoch: &Pubkey,
    epoch_index: u64,
) -> Instruction {
    let vrf_lite = ac::switchboard_vrf_lite(program_id).pubkey;
    let escrow = get_associated_token_address(&vrf_lite, switchboard_oracle_queue_mint);
    let (permission_pda, _permission_pda_bump) = get_permission_pda(
        switchboard_program_id,
        switchboard_oracle_queue_authority,
        switchboard_oracle_queue,
        &vrf_lite,
    );
    let (switchboard_program_state_pda, _switchboard_program_state_pda_bump) =
        get_program_state_pda(switchboard_program_id);
    Instruction::new_with_borsh(
        program_id.clone(),
        &NezhaVrfInstruction::RequestVRF { epoch_index },
        account_meta![
            [signer writable] admin.clone(),
            [] ac::nezha_vrf_program_state(program_id).pubkey,
            [writable] ac::nezha_vrf_request(program_id, epoch_index).pubkey,
            [] *nezha_staking_latest_epoch,
            //
            [] *switchboard_program_id,
            [] ac::switchboard_authority(program_id).pubkey,
            [writable] vrf_lite,
            [writable] *switchboard_oracle_queue,
            [] *switchboard_oracle_queue_authority,
            [] *switchboard_oracle_queue_data_buffer,
            [writable] permission_pda,
            [writable] escrow,
            [] switchboard_program_state_pda,
            //
            [] spl_token::id(),
            [] system_program::id(),
            [] rent::id(),
            [] recent_blockhashes::ID,
            [] clock::id(),
        ],
    )
}

/// Usually, this will be called by switchboard itself
pub fn consume_vrf(program_id: &Pubkey, epoch_index: u64) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &NezhaVrfInstruction::ConsumeVRF { epoch_index },
        account_meta![
            [writable] ac::nezha_vrf_request(program_id, epoch_index).pubkey,
            [] ac::switchboard_vrf_lite(program_id).pubkey,
            [] clock::id(),
        ],
    )
}

/// SuperAdmin: Rotate the key authorized as SuperAdmin, Admin, or Investor.
pub fn rotate_key(program_id: &Pubkey, super_admin: &Pubkey, key_type: RotateKeyType, new_key: &Pubkey) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &NezhaVrfInstruction::RotateKey { key_type },
        account_meta![
            [signer] super_admin.clone(),
            [writable] ac::nezha_vrf_program_state(program_id).pubkey,
            [] new_key.clone(),
            //
        ],
    )
}

pub fn mock_set_winning_combination(
    program_id: &Pubkey,
    admin: &Pubkey,
    epoch_index: u64,
    winning_combination: [u8; 6],
) -> Instruction {
    Instruction::new_with_borsh(
        *program_id,
        &NezhaVrfInstruction::MockSetWinningCombination {
            epoch_index,
            winning_combination,
        },
        account_meta![
            [signer writable] *admin,
            //
            [writable] ac::nezha_vrf_request(program_id, epoch_index).pubkey,
            //
            [] system_program::id(),
            [] rent::id(),
        ],
    )
}
