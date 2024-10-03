use anchor_lang::InstructionData;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    system_program,
    sysvar::{clock, rent},
};
use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;
use staking::instruction::{Deposit, Withdraw};

use crate::util::{user_balance_address, vault_address, vault_token_address};

pub fn deposit_instruction(program_id: Pubkey, user_pubkey: Pubkey, token_mint: Pubkey, amount: u64) -> Instruction {
    let user_balance = user_balance_address(&program_id, &user_pubkey, &token_mint);
    let user_ata = get_associated_token_address(&user_pubkey, &token_mint);
    let vault_token = vault_token_address(&program_id, &token_mint);
    let vault = vault_address(&program_id);
    let deposit = Deposit { amount };
    Instruction::new_with_bytes(
        program_id,
        &deposit.data(),
        vec![
            AccountMeta::new(user_pubkey, true),
            AccountMeta::new(user_balance, false),
            AccountMeta::new(user_ata, false),
            AccountMeta::new_readonly(token_mint, false),
            AccountMeta::new(vault_token, false),
            AccountMeta::new_readonly(vault, false),
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new_readonly(rent::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    )
}

pub fn withdraw_instruction(program_id: Pubkey, user_pubkey: Pubkey, token_mint: Pubkey, amount: u64) -> Instruction {
    let user_balance = user_balance_address(&program_id, &user_pubkey, &token_mint);
    let user_ata = get_associated_token_address(&user_pubkey, &token_mint);
    let vault_token = vault_token_address(&program_id, &token_mint);
    let vault = vault_address(&program_id);
    let withdraw = Withdraw { amount };
    Instruction::new_with_bytes(
        program_id,
        &withdraw.data(),
        vec![
            AccountMeta::new(user_pubkey, true),
            AccountMeta::new(user_balance, false),
            AccountMeta::new(user_ata, false),
            AccountMeta::new(vault_token, false),
            AccountMeta::new_readonly(vault, false),
            AccountMeta::new_readonly(clock::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    )
}
