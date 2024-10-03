use borsh::{BorshDeserialize, BorshSerialize};
use nezha_utils::load_accounts;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

use crate::{
    accounts::{self as ac, VerifyPDA},
    instruction::*,
    state::*,
    utils::*,
};

pub fn process_rotate_key(program_id: &Pubkey, accounts: &[AccountInfo], key_type: RotateKeyType) -> ProgramResult {
    msg!("Ixn: Rotate Key: {:?}", key_type);

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        super_admin_info,
        nezha_vrf_program_state_info,
        new_key_info,
    );

    ac::nezha_vrf_program_state(program_id).verify(nezha_vrf_program_state_info)?;
    let mut nezha_vrf_program_state =
        NezhaVrfProgramState::try_from_slice(&nezha_vrf_program_state_info.data.borrow())?;

    check_super_admin(super_admin_info, &nezha_vrf_program_state)?;

    let new_key = new_key_info.key.clone();

    match key_type {
        RotateKeyType::SuperAdmin => nezha_vrf_program_state.pubkeys.super_admin = new_key,
        RotateKeyType::Admin => nezha_vrf_program_state.pubkeys.admin = new_key,
    };

    BorshSerialize::serialize(
        &nezha_vrf_program_state,
        &mut *nezha_vrf_program_state_info.try_borrow_mut_data()?,
    )?;

    Ok(())
}
