//! Processor functions.

use ::borsh::BorshSerialize;

use anchor_lang::prelude::{Rent, SolanaSysvar};
use borsh::BorshDeserialize;
use nezha_utils::{borsh_length::BorshLength, load_accounts};
use nezha_vrf_lib::error::NezhaVrfError;
use nezha_vrf_lib::instruction::NezhaVrfInstruction;
use solana_program::program::invoke_signed;
use solana_program::system_instruction;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

use crate::accounts as ac;
use crate::state::*;

pub fn process_instruction<'a>(program_id: &Pubkey, accounts: &[AccountInfo<'a>], input: &[u8]) -> ProgramResult {
    let instruction = NezhaVrfInstruction::try_from_slice(input)?;
    match instruction {
        NezhaVrfInstruction::MockSetWinningCombination {
            epoch_index,
            winning_combination,
        } => process_set_winning_combination(program_id, accounts, epoch_index, &winning_combination),
        _ => unreachable!(),
    }
}

pub fn process_set_winning_combination<'a>(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'a>],
    epoch_index: u64,
    winning_combination: &[u8; 6],
) -> ProgramResult {
    msg!("Ixn: Set Winning Combination");

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        payer,
        nezha_vrf_request_info,
        //
        system_program_info,
        rent_sysvar_info,
    );

    let nezha_vrf_request_pda = ac::nezha_vrf_request(program_id, epoch_index);
    let winning_combination = Some(*winning_combination);

    let nezha_vrf_request = NezhaVrfRequest {
        account_type: NezhaVrfRequest::account_type(),
        contract_version: crate::state::CONTRACT_VERSION,
        status: NezhaVrfRequestStatus::Success,
        vrf_counter: 0,
        winning_combination,
        request_start: 0,
        request_end: None,
    };
    create_or_update_account(
        &nezha_vrf_request,
        program_id,
        payer,
        nezha_vrf_request_info,
        Some(&nezha_vrf_request_pda.seeds()),
        system_program_info,
        rent_sysvar_info,
    )?;

    Ok(())
}
fn create_or_update_account<'a, T>(
    value: &T,
    program_id: &Pubkey,
    payer_info: &AccountInfo<'a>,
    dest_info: &AccountInfo<'a>,
    dest_seeds: Option<&[&[u8]]>,
    system_program_info: &AccountInfo<'a>,
    rent_sysvar_info: &AccountInfo<'a>,
) -> ProgramResult
where
    T: BorshSerialize + BorshLength,
{
    if dest_info.lamports() <= 0 {
        let account_length = T::borsh_length();
        let rent = Rent::from_account_info(rent_sysvar_info)?;
        let required_lamports = rent.minimum_balance(account_length);

        invoke_signed(
            &system_instruction::create_account(
                payer_info.key,
                dest_info.key,
                required_lamports,
                account_length as u64,
                program_id,
            ),
            &[system_program_info.clone(), payer_info.clone(), dest_info.clone()],
            dest_seeds.as_ref().map(std::slice::from_ref).unwrap_or_default(),
        )
        .map_err(NezhaVrfError::system_program_error)?;
    }

    BorshSerialize::serialize(&value, &mut *dest_info.try_borrow_mut_data()?)?;

    Ok(())
}
