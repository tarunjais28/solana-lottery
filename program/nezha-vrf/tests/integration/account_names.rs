use nezha_vrf_lib::{
    accounts as ac,
    switchboard::{get_permission_pda, get_program_state_pda},
};
use solana_program::{pubkey::Pubkey, system_program, sysvar};
use solana_sdk::signer::Signer;
use spl_associated_token_account::get_associated_token_address;
use std::collections::HashMap;

use crate::accounts::Accounts;

pub fn build_account_names_map(accounts: &Accounts) -> HashMap<Pubkey, String> {
    let program_id = &accounts.program_id;

    HashMap::from([
        (accounts.super_admin.pubkey(), "super_admin".into()),
        (accounts.admin.pubkey(), "admin".into()),
        (accounts.random1.pubkey(), "random1".into()),
        (accounts.random2.pubkey(), "random2".into()),
        (accounts.switchboard_queue, "switchboard_queue".into()),
        (accounts.switchboard_queue_mint, "switchboard_queue_mint".into()),
        (
            accounts.switchboard_queue_authority,
            "switchboard_queue_authority".into(),
        ),
        (
            accounts.switchboard_queue_data_buffer,
            "switchboard_queue_data_buffer".into(),
        ),
        (
            ac::nezha_vrf_program_state(program_id).pubkey,
            "nezha_vrf_program_state".into(),
        ),
        (
            ac::nezha_vrf_request(program_id, 1).pubkey,
            "nezha_vrf_request_1".into(),
        ),
        (
            ac::nezha_vrf_request(program_id, 2).pubkey,
            "nezha_vrf_request_2".into(),
        ),
        (
            ac::nezha_vrf_request(program_id, 3).pubkey,
            "nezha_vrf_request_3".into(),
        ),
        (
            ac::switchboard_vrf_lite(program_id).pubkey,
            "switchboard_vrf_lite".into(),
        ),
        (
            ac::switchboard_authority(program_id).pubkey,
            "switchboard_authority".into(),
        ),
        (system_program::id(), "system_program".into()),
        (spl_token::id(), "spl_token".into()),
        (
            spl_associated_token_account::id(),
            "spl_associated_token_account".into(),
        ),
        (sysvar::rent::id(), "sysvar_rent".into()),
        (sysvar::clock::id(), "sysvar_clock".into()),
        (switchboard_v2::ID, "switchboard_program".into()),
        {
            let vrf_lite = ac::switchboard_vrf_lite(program_id).pubkey;
            let escrow = get_associated_token_address(&vrf_lite, &accounts.switchboard_queue_mint);
            (escrow, "switchboard_escrow".into())
        },
        {
            let vrf_lite = ac::switchboard_vrf_lite(program_id).pubkey;
            let (permission_pda, _permission_pda_bump) = get_permission_pda(
                &switchboard_v2::ID,
                &accounts.switchboard_queue_authority,
                &accounts.switchboard_queue,
                &vrf_lite,
            );
            (permission_pda, "switchboard_permission".into())
        },
        {
            let (switchboard_program_state_pda, _switchboard_program_state_pda_bump) =
                get_program_state_pda(&switchboard_v2::ID);
            (switchboard_program_state_pda, "switchboard_program_state".into())
        },
    ])
}
