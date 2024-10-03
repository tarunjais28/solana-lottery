use ::borsh::BorshSerialize;
use anchor_lang::prelude::{Account as AnchorAccount, AccountLoader};
use nezha_utils::{borsh_deserialize::borsh_deserialize, borsh_length::BorshLength, load_accounts};
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use solana_program::{
    account_info::AccountInfo,
    clock::Clock,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};
use switchboard_v2::{Callback, OracleQueueAccountData, VrfLiteAccountData, VrfLiteRequestRandomness, VrfStatus};

use crate::{
    accounts::{self as ac, VerifyPDA},
    error::{NezhaVrfError, SignatureType},
    instruction::NezhaVrfInstruction,
    processor::CONTRACT_VERSION,
    state::{HasAccountType, NezhaVrfProgramState, NezhaVrfRequest, NezhaVrfRequestStatus, Pubkeys},
    switchboard::{VrfLiteInitAccounts, VrfLiteInitParams},
    utils::{
        check_admin, check_ata_program, check_rent_sysvar, check_switchboard_program, check_system_program,
        check_token_program,
    },
};
use nezha_staking_lib::{accounts as staking_ac, state::EpochStatus};
use nezha_utils::checks::check_ata_account;
use nezha_vrf_lib::{
    accounts::AccountType,
    switchboard::{get_permission_pda, get_program_state_pda},
};

#[inline(never)]
pub fn process_init<'a>(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'a>],
    switchboard_program_state_pda_bump: u8,
) -> ProgramResult {
    msg!("Ixn: Init");

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        super_admin_info,
        admin_info,
        nezha_vrf_program_state_info,
        //
        switchboard_program_info,
        switchboard_authority_info,
        vrf_lite_info,
        escrow_mint_info,
        escrow_info,
        oracle_queue_authority_info,
        oracle_queue_info,
        permission_pda_info,
        program_state_pda_info,
        //
        nezha_staking_program_info,
        //
        token_program_info,
        ata_program_info,
        system_program_info,
        rent_sysvar_info,
    );

    if nezha_vrf_program_state_info.lamports() != 0 {
        return Err(NezhaVrfError::ProgramAlreadyInitialized.into());
    }

    let nezha_vrf_program_state_pda = ac::nezha_vrf_program_state(program_id);
    nezha_vrf_program_state_pda.verify(nezha_vrf_program_state_info)?;

    if !super_admin_info.is_signer {
        msg!("Error: Super admin need to be signer");
        return Err(NezhaVrfError::MissingSignature(SignatureType::SuperAdmin).into());
    }

    let vrf_lite_pda = ac::switchboard_vrf_lite(program_id);
    vrf_lite_pda.verify(vrf_lite_info)?;

    ac::switchboard_authority(program_id).verify(switchboard_authority_info)?;

    let oracle_queue_loader = AccountLoader::<OracleQueueAccountData>::try_from(oracle_queue_info)?;
    let oracle_queue = oracle_queue_loader.load()?;

    if *escrow_mint_info.key != oracle_queue.mint {
        msg!("Error: Invalid escrow mint");
        return Err(ProgramError::InvalidArgument);
    }

    drop(oracle_queue);
    drop(oracle_queue_loader);

    check_ata_account(
        "switchboard_escrow",
        escrow_info.key,
        vrf_lite_info.key,
        escrow_mint_info.key,
    )?;

    check_token_program(token_program_info)?;
    check_ata_program(ata_program_info)?;
    check_system_program(system_program_info)?;
    check_rent_sysvar(rent_sysvar_info)?;

    let (permission_pda, _) = get_permission_pda(
        switchboard_program_info.key,
        oracle_queue_authority_info.key,
        oracle_queue_info.key,
        vrf_lite_info.key,
    );
    if permission_pda_info.key != &permission_pda {
        msg!("Error: Invalid permission pda");
        return Err(ProgramError::InvalidArgument);
    }

    let (sw_program_state_pda, bump) = get_program_state_pda(switchboard_program_info.key);

    if program_state_pda_info.key != &sw_program_state_pda {
        msg!("Error: Invalid program state pda");
        return Err(ProgramError::InvalidArgument);
    }

    if switchboard_program_state_pda_bump != bump {
        msg!("Error: Invalid program state pda bump");
        return Err(ProgramError::InvalidArgument);
    }

    msg!("Init VrfLite");
    crate::switchboard::vrf_lite_init(
        &VrfLiteInitParams {
            callback: None,
            state_bump: switchboard_program_state_pda_bump,
            expiration: Some(0),
        },
        &VrfLiteInitAccounts {
            switchboard_program: switchboard_program_info.clone(),
            authority: switchboard_authority_info.clone(),
            vrf_lite: vrf_lite_info.clone(),
            vrf_lite_seeds: Some(&vrf_lite_pda.seeds()),
            mint: escrow_mint_info.clone(),
            escrow: escrow_info.clone(),
            queue_authority: oracle_queue_authority_info.clone(),
            queue: oracle_queue_info.clone(),
            permission: permission_pda_info.clone(),
            program_state: program_state_pda_info.clone(),
            payer: admin_info.clone(),
            token_program: token_program_info.clone(),
            ata_program: ata_program_info.clone(),
            system_program: system_program_info.clone(),
            rent_sysvar: rent_sysvar_info.clone(),
        },
    )
    .map_err(NezhaVrfError::switchboard_error)?;

    msg!("Write NezhaVrfProgramState");

    let nezha_vrf_program_state = NezhaVrfProgramState {
        account_type: NezhaVrfProgramState::account_type(),
        contract_version: CONTRACT_VERSION,
        pubkeys: Pubkeys {
            super_admin: *super_admin_info.key,
            admin: *admin_info.key,
            switchboard_program_id: *switchboard_program_info.key,
            nezha_staking_program_id: *nezha_staking_program_info.key,
        },
    };
    create_or_update_account(
        &nezha_vrf_program_state,
        program_id,
        admin_info,
        nezha_vrf_program_state_info,
        Some(&nezha_vrf_program_state_pda.seeds()),
        system_program_info,
        rent_sysvar_info,
    )?;

    Ok(())
}

#[inline(never)]
pub fn process_request_vrf<'a>(program_id: &Pubkey, accounts: &[AccountInfo<'a>], epoch_index: u64) -> ProgramResult {
    msg!("Ixn: Request VRF {}", epoch_index);

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        admin_info,
        nezha_vrf_program_state_info,
        nezha_vrf_request_info,
        latest_epoch_info,
        //
        switchboard_program_info,
        switchboard_authority_info,
        vrf_lite_info,
        oracle_queue_info,
        queue_authority_info,
        data_buffer_info,
        permission_info,
        escrow_info,
        switchboard_program_state_info,
        //
        token_program_info,
        system_program_info,
        rent_sysvar_info,
        recent_blockhashes_info,
        clock_sysvar_info,
    );

    check_token_program(&token_program_info)?;
    check_system_program(&system_program_info)?;
    check_rent_sysvar(&rent_sysvar_info)?;

    ac::nezha_vrf_program_state(program_id).verify(nezha_vrf_program_state_info)?;
    let nezha_vrf_request_pda = ac::nezha_vrf_request(program_id, epoch_index);
    nezha_vrf_request_pda.verify(nezha_vrf_request_info)?;

    let nezha_vrf_program_state: NezhaVrfProgramState = borsh_deserialize(nezha_vrf_program_state_info)?;
    check_admin(&admin_info, &nezha_vrf_program_state)?;

    check_switchboard_program(switchboard_program_info, &nezha_vrf_program_state)?;
    ac::switchboard_vrf_lite(program_id).verify(vrf_lite_info)?;
    let switchboard_authority_pda = ac::switchboard_authority(program_id);
    switchboard_authority_pda.verify(switchboard_authority_info)?;

    let oracle_queue_loader = AccountLoader::<OracleQueueAccountData>::try_from(oracle_queue_info)?;
    let oracle_queue = oracle_queue_loader.load()?;

    if *data_buffer_info.key != oracle_queue.data_buffer {
        msg!("Error: Data buffer passed is not the the queue's data buffer");
        return Err(ProgramError::InvalidArgument);
    }
    drop(oracle_queue);
    drop(oracle_queue_loader);

    staking_ac::latest_epoch(&nezha_vrf_program_state.pubkeys.nezha_staking_program_id)
        .with_account_type(AccountType::NezhaStakingLatestEpoch)
        .verify(latest_epoch_info)?;

    let latest_epoch: nezha_staking_lib::state::LatestEpoch = borsh_deserialize(latest_epoch_info)?;
    if latest_epoch.index != epoch_index {
        msg!(
            "epoch_index ({}) doesn't match LatestEpoch.index ({})",
            epoch_index,
            latest_epoch.index
        );
        return Err(ProgramError::InvalidArgument);
    }
    if latest_epoch.status != EpochStatus::Finalising {
        return Err(NezhaVrfError::EpochNotInFinalising.into());
    }
    drop(latest_epoch);

    if nezha_vrf_request_info.lamports() != 0 {
        let nezha_vrf_request: NezhaVrfRequest = borsh_deserialize(nezha_vrf_request_info)?;

        if nezha_vrf_request.winning_combination.is_some() {
            msg!("Error: Winning combination is already set for epoch {}", epoch_index);
            return Err(NezhaVrfError::WinningCombinationAlreadySet.into());
        }
    }

    let callback = {
        let accounts: Vec<switchboard_v2::AccountMetaBorsh> = vec![
            switchboard_v2::AccountMetaBorsh {
                pubkey: nezha_vrf_request_info.key.clone(),
                is_signer: false,
                is_writable: true,
            },
            switchboard_v2::AccountMetaBorsh {
                pubkey: *vrf_lite_info.key,
                is_signer: false,
                is_writable: false,
            },
            switchboard_v2::AccountMetaBorsh {
                pubkey: *clock_sysvar_info.key,
                is_signer: false,
                is_writable: false,
            },
        ];
        let ix_data = (NezhaVrfInstruction::ConsumeVRF { epoch_index }).try_to_vec()?;
        Callback {
            program_id: *program_id,
            accounts,
            ix_data,
        }
    };

    let escrow = AnchorAccount::try_from(escrow_info).map_err(NezhaVrfError::switchboard_error_anchor)?;

    // then request randomness
    let vrf_request_randomness = VrfLiteRequestRandomness {
        authority: switchboard_authority_info.clone(),
        vrf_lite: vrf_lite_info.clone(),
        queue: oracle_queue_info.clone(),
        queue_authority: queue_authority_info.clone(),
        data_buffer: data_buffer_info.clone(),
        permission: permission_info.clone(),
        escrow,
        recent_blockhashes: recent_blockhashes_info.clone(),
        program_state: switchboard_program_state_info.clone(),
        token_program: token_program_info.clone(),
    };

    fund_escrow(admin_info, escrow_info, system_program_info, token_program_info)?;

    msg!("Requesting randomness");
    vrf_request_randomness
        .invoke_signed(
            switchboard_program_info.clone(),
            Some(callback),
            &[&switchboard_authority_pda.seeds()],
        )
        .map_err(NezhaVrfError::switchboard_error)?;

    let vrf_account = Box::new(VrfLiteAccountData::new(vrf_lite_info)?);
    let vrf_counter = vrf_account.counter;
    drop(vrf_account);

    msg!("Writing NezhaVrfRequest");

    let clock = Clock::from_account_info(clock_sysvar_info)?;
    let nezha_vrf_request = NezhaVrfRequest {
        account_type: NezhaVrfRequest::account_type(),
        contract_version: crate::state::CONTRACT_VERSION,
        status: NezhaVrfRequestStatus::Waiting,
        vrf_counter,
        winning_combination: None,
        request_start: clock.unix_timestamp,
        request_end: None,
    };
    create_or_update_account(
        &nezha_vrf_request,
        program_id,
        admin_info,
        nezha_vrf_request_info,
        Some(&nezha_vrf_request_pda.seeds()),
        system_program_info,
        rent_sysvar_info,
    )?;

    Ok(())
}

pub fn process_consume_vrf<'a>(program_id: &Pubkey, accounts: &[AccountInfo<'a>], epoch_index: u64) -> ProgramResult {
    msg!("Ixn: Consume VRF {}", epoch_index);

    let account_info_iter = &mut accounts.iter();
    load_accounts!(
        account_info_iter,
        //
        nezha_vrf_request_info,
        //
        vrf_lite_account_info,
        //
        clock_sysvar_info,
    );

    ac::nezha_vrf_request(program_id, epoch_index).verify(nezha_vrf_request_info)?;

    let mut nezha_vrf_request: NezhaVrfRequest = borsh_deserialize(nezha_vrf_request_info)?;

    if nezha_vrf_request.winning_combination.is_some() {
        msg!("Error: Winning combination is already set for epoch {}", epoch_index);
        return Err(NezhaVrfError::WinningCombinationAlreadySet.into());
    }

    ac::switchboard_vrf_lite(program_id).verify(vrf_lite_account_info)?;

    let vrf_account = VrfLiteAccountData::new(vrf_lite_account_info)?;
    let vrf_counter = vrf_account.counter;
    if vrf_counter != nezha_vrf_request.vrf_counter {
        msg!(
            "Error: Counter mismatch. Expected {}. Got {}",
            &nezha_vrf_request.vrf_counter,
            &vrf_counter
        );
        nezha_vrf_request.status = NezhaVrfRequestStatus::Fail;
    } else {
        nezha_vrf_request.status = match vrf_account.status {
            VrfStatus::StatusNone | VrfStatus::StatusRequesting | VrfStatus::StatusVerifying => {
                NezhaVrfRequestStatus::Waiting
            }
            VrfStatus::StatusVerified | VrfStatus::StatusCallbackSuccess => NezhaVrfRequestStatus::Success,
            VrfStatus::StatusVerifyFailure => NezhaVrfRequestStatus::Fail,
        };
    }

    if nezha_vrf_request.status == NezhaVrfRequestStatus::Success {
        let winning_combination: [u8; 6] = {
            // unwrap: The result slice is guaranteed to be 32 size so ..16 is always possible.
            let mut rng = StdRng::from_seed(vrf_account.result);
            let mut sequence_vec = (1..=56)
                .collect::<Vec<u8>>()
                .choose_multiple(&mut rng, 5)
                .cloned()
                .collect::<Vec<u8>>();

            // Pick 1 random byte out of 10
            let last_number: u8 = rng.gen_range(1, 11);

            sequence_vec.push(last_number);
            sequence_vec
                .try_into()
                .expect("winning combination should fit into [u8;6]")
        };
        msg!("Winning combination: {:?}", winning_combination);
        nezha_vrf_request.winning_combination = Some(winning_combination);
    }

    let clock = Clock::from_account_info(clock_sysvar_info)?;
    nezha_vrf_request.request_end = Some(clock.unix_timestamp);
    nezha_vrf_request.serialize(&mut *nezha_vrf_request_info.try_borrow_mut_data()?)?;

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

fn fund_escrow<'a>(
    payer_info: &AccountInfo<'a>,
    escrow_info: &AccountInfo<'a>,
    system_program_info: &AccountInfo<'a>,
    token_program_info: &AccountInfo<'a>,
) -> ProgramResult {
    let lamports = 2_000_000;
    msg!("Fund Escrow: {} lamports");
    invoke(
        &system_instruction::transfer(payer_info.key, escrow_info.key, lamports),
        &[system_program_info.clone(), payer_info.clone(), escrow_info.clone()],
    )
    .map_err(NezhaVrfError::system_program_error)?;
    invoke(
        &spl_token::instruction::sync_native(token_program_info.key, escrow_info.key)?,
        &[token_program_info.clone(), escrow_info.clone()],
    )
    .map_err(NezhaVrfError::token_program_error)?;

    Ok(())
}
