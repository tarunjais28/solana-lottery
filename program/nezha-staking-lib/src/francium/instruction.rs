use francium_lending_pool::instruction::*;
use francium_lending_rewards_pool::instruction::*;

use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

// -- Lending Pool --

pub fn update_lending_pool(
    lending_program_id: &Pubkey,
    market_info_account: &Pubkey,
    lending_pool_info_account: &Pubkey,
    sysvar_clock: &Pubkey,
) -> Instruction {
    let keys = vec![
        AccountMeta::new(*market_info_account, false),
        AccountMeta::new(*lending_pool_info_account, false),
        AccountMeta::new_readonly(*sysvar_clock, false),
    ];
    let data: Vec<u8> = vec![12];
    Instruction {
        program_id: *lending_program_id,
        accounts: keys,
        data,
    }
}

pub fn deposit(
    lending_program_id: &Pubkey,
    user_liquidity_token_account: &Pubkey,
    user_share_token_account: &Pubkey,
    lending_pool_info_account: &Pubkey,
    lending_pool_liquidity_token_account: &Pubkey,
    share_token_mint: &Pubkey,
    market_info_account: &Pubkey,
    lending_market_authority: &Pubkey,
    user_pubkey: &Pubkey,
    sysvar_clock: &Pubkey,
    token_program_id: &Pubkey,
    liquidity_amount: u64,
) -> Instruction {
    let keys = vec![
        AccountMeta::new(*user_liquidity_token_account, false),
        AccountMeta::new(*user_share_token_account, false),
        AccountMeta::new(*lending_pool_info_account, false),
        AccountMeta::new(*lending_pool_liquidity_token_account, false),
        AccountMeta::new(*share_token_mint, false),
        AccountMeta::new_readonly(*market_info_account, false),
        AccountMeta::new_readonly(*lending_market_authority, false),
        AccountMeta::new_readonly(*user_pubkey, true),
        AccountMeta::new_readonly(*sysvar_clock, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    let data = LendingInstruction::DepositToLendingPool { liquidity_amount }.pack();
    Instruction {
        program_id: *lending_program_id,
        accounts: keys,
        data,
    }
}

pub fn withdraw(
    lending_program_id: &Pubkey,
    user_share_token_account: &Pubkey,
    user_liquidity_token_account: &Pubkey,
    lending_pool_info_account: &Pubkey,
    lending_pool_share_mint: &Pubkey,
    lending_pool_token_account: &Pubkey,
    market_info_account: &Pubkey,
    lending_market_authority: &Pubkey,
    user_pubkey: &Pubkey,
    sysvar_clock: &Pubkey,
    token_program_id: &Pubkey,
    share_amount: u64,
) -> Instruction {
    let keys = vec![
        AccountMeta::new(*user_share_token_account, false),
        AccountMeta::new(*user_liquidity_token_account, false),
        AccountMeta::new(*lending_pool_info_account, false),
        AccountMeta::new(*lending_pool_share_mint, false),
        AccountMeta::new(*lending_pool_token_account, false),
        AccountMeta::new_readonly(*market_info_account, false),
        AccountMeta::new_readonly(*lending_market_authority, false),
        AccountMeta::new_readonly(*user_pubkey, true),
        AccountMeta::new_readonly(*sysvar_clock, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    let data = LendingInstruction::WithdrawFromLendingPool { share_amount }.pack();
    Instruction {
        program_id: *lending_program_id,
        accounts: keys,
        data,
    }
}

// -- Lending Rewards Pool --

pub fn init_farming_info(
    rewards_program_id: &Pubkey,
    user_pubkey: &Pubkey,
    user_farming_info_pubkey: &Pubkey,
    farming_pool_account: &Pubkey,
    user_stake_token_account: &Pubkey,
    user_rewards_account: &Pubkey,
    user_rewards_accountb: &Pubkey,
    system_program_id: &Pubkey,
    sysvar_rent_pubkey: &Pubkey,
) -> Instruction {
    let keys = vec![
        AccountMeta::new(*user_pubkey, true),
        AccountMeta::new(*user_farming_info_pubkey, false),
        AccountMeta::new(*farming_pool_account, false),
        AccountMeta::new(*user_stake_token_account, false),
        AccountMeta::new(*user_rewards_account, false),
        AccountMeta::new(*user_rewards_accountb, false),
        AccountMeta::new_readonly(*system_program_id, false),
        AccountMeta::new_readonly(*sysvar_rent_pubkey, false),
    ];
    let data = vec![1];
    Instruction {
        program_id: *rewards_program_id,
        accounts: keys,
        data,
    }
}

pub fn stake_to_farming_pool(
    rewards_program_id: &Pubkey,
    user_pubkey: &Pubkey,
    user_farming_info_pubkey: &Pubkey,
    user_stake_token_account: &Pubkey,
    user_rewards_account: &Pubkey,
    user_rewards_account_b: &Pubkey,
    farming_pool_account: &Pubkey,
    farming_pool_authority: &Pubkey,
    farming_pool_stake_token_account: &Pubkey,
    farming_pool_rewards_token_account: &Pubkey,
    farming_pool_rewards_token_account_b: &Pubkey,
    token_program_id: &Pubkey,
    sysvar_clock_pubkey: &Pubkey,
    amount: u64,
) -> Instruction {
    let keys = vec![
        AccountMeta::new(*user_pubkey, true),
        AccountMeta::new(*user_farming_info_pubkey, false),
        AccountMeta::new(*user_stake_token_account, false),
        AccountMeta::new(*user_rewards_account, false),
        AccountMeta::new(*user_rewards_account_b, false),
        AccountMeta::new(*farming_pool_account, false),
        AccountMeta::new(*farming_pool_authority, false),
        AccountMeta::new(*farming_pool_stake_token_account, false),
        AccountMeta::new(*farming_pool_rewards_token_account, false),
        AccountMeta::new(*farming_pool_rewards_token_account_b, false),
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new_readonly(*sysvar_clock_pubkey, false),
    ];
    let data = FarmingInstructions::Stake { amount }.pack();
    Instruction {
        program_id: *rewards_program_id,
        accounts: keys,
        data,
    }
}

pub fn unstake_from_farming_pool(
    lending_rewards_program_id: &Pubkey,
    user_pubkey: &Pubkey,
    user_farming_info_pubkey: &Pubkey,
    user_stake_token_account: &Pubkey,
    user_rewards_account: &Pubkey,
    user_rewards_account_b: &Pubkey,
    farming_pool_account: &Pubkey,
    farming_pool_authority: &Pubkey,
    farming_pool_stake_token_account: &Pubkey,
    farming_pool_rewards_token_account: &Pubkey,
    farming_pool_rewards_token_account_b: &Pubkey,
    token_program_id: &Pubkey,
    sysvar_clock_pubkey: &Pubkey,
    amount: u64,
) -> Instruction {
    let keys = vec![
        AccountMeta::new(*user_pubkey, true),
        AccountMeta::new(*user_farming_info_pubkey, false),
        AccountMeta::new(*user_stake_token_account, false),
        AccountMeta::new(*user_rewards_account, false),
        AccountMeta::new(*user_rewards_account_b, false),
        AccountMeta::new(*farming_pool_account, false),
        AccountMeta::new(*farming_pool_authority, false),
        AccountMeta::new(*farming_pool_stake_token_account, false),
        AccountMeta::new(*farming_pool_rewards_token_account, false),
        AccountMeta::new(*farming_pool_rewards_token_account_b, false),
        AccountMeta::new_readonly(*token_program_id, false),
        AccountMeta::new_readonly(*sysvar_clock_pubkey, false),
    ];
    let data = FarmingInstructions::UnStake { amount }.pack();
    Instruction {
        program_id: *lending_rewards_program_id,
        accounts: keys,
        data,
    }
}
