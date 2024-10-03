use nezha_staking_lib::accounts as ac;
use solana_program::{pubkey::Pubkey, system_program, sysvar};
use solana_sdk::signer::Signer;
use spl_associated_token_account::get_associated_token_address;
use std::collections::HashMap;

use crate::accounts::Accounts;

pub fn build_account_names_map(accounts: &Accounts) -> HashMap<Pubkey, String> {
    let owner_usdc = get_associated_token_address(&accounts.owner.pubkey(), &accounts.usdc_mint.pubkey());
    let program_id = &accounts.program_id;

    HashMap::from([
        (accounts.super_admin.pubkey(), "super_admin".into()),
        (accounts.admin.pubkey(), "admin".into()),
        (accounts.owner.pubkey(), "owner".into()),
        (owner_usdc, "owner_usdc".into()),
        (accounts.investor.pubkey(), "investor".into()),
        (accounts.usdc_mint.pubkey(), "usdc_mint".into()),
        (accounts.random1.pubkey(), "random1".into()),
        (accounts.random2.pubkey(), "random2".into()),
        (ac::vault_authority(program_id).pubkey, "vault_authority".into()),
        (ac::deposit_vault(program_id).pubkey, "deposit_vault".into()),
        (ac::treasury_vault(program_id).pubkey, "treasury_vault".into()),
        (ac::insurance_vault(program_id).pubkey, "insurance_vault".into()),
        (ac::prize_vault(program_id, 1).pubkey, "tier1_prize_vault".into()),
        (ac::prize_vault(program_id, 2).pubkey, "tier2_prize_vault".into()),
        (ac::prize_vault(program_id, 3).pubkey, "tier3_prize_vault".into()),
        (
            ac::pending_deposit_vault(program_id).pubkey,
            "pending_deposit_vault".into(),
        ),
        (ac::latest_epoch(program_id).pubkey, "latest_epoch".into()),
        (ac::epoch(program_id, 1).pubkey, "epoch 1".into()),
        (ac::epoch(program_id, 2).pubkey, "epoch 2".into()),
        (ac::stake(program_id, &accounts.owner.pubkey()).pubkey, "stake".into()),
        (
            ac::stake_update_request(program_id, &accounts.owner.pubkey()).pubkey,
            "stake_update_request".into(),
        ),
        (accounts.nezha_vrf_program_id, "nezha_vrf_program".into()),
        (system_program::id(), "system_program".into()),
        (spl_token::id(), "spl_token".into()),
        (
            spl_associated_token_account::id(),
            "spl_associated_token_account".into(),
        ),
        (sysvar::rent::id(), "sysvar_rent".into()),
        (sysvar::clock::id(), "sysvar_clock".into()),
    ])
}
