use anchor_lang::Discriminator;
use nezha_vrf_lib::switchboard::{get_permission_pda, get_program_state_pda};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke, program_error::ProgramError,
    pubkey::Pubkey, system_instruction::create_account, sysvar::recent_blockhashes,
};
use spl_associated_token_account::instruction::create_associated_token_account;
use std::{
    cell::RefCell,
    sync::atomic::{AtomicU64, Ordering},
};
use switchboard_v2::{AccountMetaZC, CallbackZC, VrfBuilder, VrfLiteAccountData};

thread_local! {
    static AUTHORITY: RefCell<Pubkey> = RefCell::new(Pubkey::default());
    static COUNTER: AtomicU64 = AtomicU64::new(0);
}

pub fn process(program_id: &Pubkey, account_infos: &[AccountInfo], input: &[u8]) -> ProgramResult {
    msg!("VRF: Program");
    let discriminator = &input[0..8];
    if discriminator == nezha_vrf::switchboard::VRF_LITE_INIT_DISCRIMINATOR {
        msg!("VRF: Init");
        let authority_info = &account_infos[0];
        AUTHORITY.with(|a| a.replace(*authority_info.key));

        let vrf_lite_info = &account_infos[1];
        let mint_info = &account_infos[2];
        let escrow_info = &account_infos[3];
        let payer_info = &account_infos[8];
        let token_program_info = &account_infos[9];
        let system_program_info = &account_infos[11];
        invoke(
            &create_account(
                payer_info.key,
                vrf_lite_info.key,
                1,
                (std::mem::size_of::<VrfLiteAccountData>() + 8) as _,
                program_id,
            ),
            &[payer_info.clone(), vrf_lite_info.clone()],
        )?;
        invoke(
            &create_associated_token_account(payer_info.key, vrf_lite_info.key, mint_info.key, &spl_token::ID),
            &[
                vrf_lite_info.clone(),
                payer_info.clone(),
                escrow_info.clone(),
                mint_info.clone(),
                token_program_info.clone(),
                system_program_info.clone(),
            ],
        )?;
        let vrf_lite = VrfLiteAccountData {
            state_bump: 0,
            permission_bump: 0,
            vrf_pool: Pubkey::default(),
            status: switchboard_v2::VrfStatus::StatusCallbackSuccess,
            result: [0u8; 32],
            counter: 99999,
            alpha: [0u8; 256],
            alpha_len: 0,
            request_slot: 0,
            request_timestamp: 0,
            authority: Pubkey::default(),
            queue: Pubkey::default(),
            escrow: Pubkey::default(),
            callback: CallbackZC {
                program_id: Pubkey::default(),
                accounts: [AccountMetaZC {
                    pubkey: Pubkey::default(),
                    is_signer: false,
                    is_writable: false,
                }; 32],
                accounts_len: 0,
                ix_data: [0u8; 1024],
                ix_data_len: 0,
            },
            builder: VrfBuilder::default(),
            expiration: 0,
        };

        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&VrfLiteAccountData::DISCRIMINATOR);
        data.extend_from_slice(bytemuck::bytes_of(&vrf_lite));
        vrf_lite_info.try_borrow_mut_data()?.copy_from_slice(&data);
    } else if discriminator == switchboard_v2::VrfLiteRequestRandomness::DISCRIMINATOR {
        msg!("VRF: Request randomness");
        let authority_info = &account_infos[0];
        let vrf_lite_info = &account_infos[1];
        if !vrf_lite_info.is_writable {
            msg!("Error: VRF Lite needs to be writable");
            return Err(ProgramError::InvalidArgument);
        }
        let queue_info = &account_infos[2];
        let queue_authority_info = &account_infos[3];
        let permission_info = &account_infos[5];
        let recent_blockhashes_info = &account_infos[7];
        let program_state_info = &account_infos[8];
        let stored_authority = AUTHORITY.with(|a| a.borrow().clone());
        if stored_authority == Pubkey::default() {
            msg!("Error: Switchboard has not been initialized");
            return Err(ProgramError::InvalidArgument);
        }
        if authority_info.key != &stored_authority {
            msg!("Error: Invalid switchboard authority");
            return Err(ProgramError::InvalidArgument);
        }

        let (permission_pda, permission_bump) =
            get_permission_pda(program_id, queue_authority_info.key, queue_info.key, vrf_lite_info.key);
        if permission_pda != *permission_info.key {
            msg!("Error: Invalid permission PDA");
            return Err(ProgramError::InvalidArgument);
        }

        let (program_state_pda, state_bump) = get_program_state_pda(program_id);
        if program_state_pda != *program_state_info.key {
            msg!("Error: Invalid program state PDA");
            return Err(ProgramError::InvalidArgument);
        }
        if recent_blockhashes_info.key != &recent_blockhashes::ID {
            msg!("Error: Invalid recent blockhashes");
            return Err(ProgramError::InvalidArgument);
        }

        if !queue_info.is_writable {
            msg!("Error: Queue must be writable");
            return Err(ProgramError::InvalidArgument);
        }

        if !permission_info.is_writable {
            msg!("Error: Permission must be writable");
            return Err(ProgramError::InvalidArgument);
        }

        let mut vrf_lite = VrfLiteAccountData {
            state_bump,
            permission_bump,
            vrf_pool: Pubkey::default(),
            status: switchboard_v2::VrfStatus::StatusCallbackSuccess,
            result: [0u8; 32],
            counter: 0,
            alpha: [0u8; 256],
            alpha_len: 0,
            request_slot: 0,
            request_timestamp: 0,
            authority: Pubkey::default(),
            queue: Pubkey::default(),
            escrow: Pubkey::default(),
            callback: CallbackZC {
                program_id: Pubkey::default(),
                accounts: [AccountMetaZC {
                    pubkey: Pubkey::default(),
                    is_signer: false,
                    is_writable: false,
                }; 32],
                accounts_len: 0,
                ix_data: [0u8; 1024],
                ix_data_len: 0,
            },
            builder: VrfBuilder::default(),
            expiration: 0,
        };

        let counter = COUNTER.with(|c| c.fetch_add(1, Ordering::Relaxed));
        vrf_lite.result[0..8].copy_from_slice(&counter.to_le_bytes());
        vrf_lite.counter = counter as _;

        let mut data: Vec<u8> = Vec::new();
        data.extend_from_slice(&VrfLiteAccountData::DISCRIMINATOR);
        data.extend_from_slice(bytemuck::bytes_of(&vrf_lite));

        vrf_lite_info.try_borrow_mut_data()?.copy_from_slice(&data);
    }
    Ok(())
}
