use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Debug)]
pub struct DepositParam {
    instruction: u8,
    amount: u64,
}

impl DepositParam {
    pub fn serialize_to_vec(amount: u64) -> Vec<u8> {
        let param = &BorrowParam {
            instruction: 4,
            amount,
        };
        param.try_to_vec().unwrap()
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Debug)]
pub struct WithdrawParam {
    instruction: u8,
    amount: u64,
}

impl WithdrawParam {
    pub fn serialize_to_vec(amount: u64) -> Vec<u8> {
        let param = &BorrowParam {
            instruction: 5,
            amount,
        };
        param.try_to_vec().unwrap()
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Debug)]
pub struct WithdrawParam2 {
    instruction: u8,
    amount: u64,
}

impl WithdrawParam2 {
    pub fn serialize_to_vec(amount: u64) -> Vec<u8> {
        let param = &BorrowParam {
            instruction: 6,
            amount,
        };
        param.try_to_vec().unwrap()
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Debug)]
pub struct BorrowParam {
    instruction: u8,
    amount: u64,
}

impl BorrowParam {
    pub fn serialize_to_vec(amount: u64) -> Vec<u8> {
        let param = &BorrowParam {
            instruction: 10,
            amount,
        };
        param.try_to_vec().unwrap()
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Debug)]
pub struct RepayParam {
    instruction: u8,
    amount: u64,
}

impl RepayParam {
    pub fn serialize_to_vec(amount: u64) -> Vec<u8> {
        let param = &BorrowParam {
            instruction: 11,
            amount,
        };
        param.try_to_vec().unwrap()
    }
}

pub fn update(
    lending_program_id: &Pubkey,
    lending_pool_info: &Pubkey,
) -> Result<Instruction> {
    let mut data:Vec<u8> = Vec::new();
    data.push(17);

    let accounts = vec![
        AccountMeta::new(*lending_pool_info, false),
    ];

    Ok(Instruction {
        program_id: *lending_program_id,
        accounts,
        data,
    })
}

/// deposit to lending pool
///    0. `[]` lending_program_id
///    1. `[writable]` User liquidity token account of user.
///    2. `[writable]` User share token account of user.
///    3. `[writable]` Lending pool info account
///    4. `[writable]` Liquidity token account of lending pool
///    5. `[writable]` Share token mint
///    6. `[writable]` Lending market account
///    7. `[]` Lending market authority
///    8. `[signer]` User transfer authority
///    9. `[]` Clock sysvar.
///    10. `[]` Token program id.
pub fn deposit(
    lending_program_id: &Pubkey,
    user_liquidity_token_account: &Pubkey,
    user_share_token_account: &Pubkey,
    lending_pool_info: &Pubkey,
    pool_liquidity_token_account: &Pubkey,
    share_token_mint: &Pubkey,
    lending_market_info: &Pubkey,
    lending_market_authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    sysvar_clock: &Pubkey,
    token_program_id: &Pubkey,
    amount: u64
) -> Result<Instruction> {
    assert!(spl_token::id().eq(token_program_id));

    let data = DepositParam::serialize_to_vec(amount);

    let accounts = vec![
        AccountMeta::new(*user_liquidity_token_account, false),
        AccountMeta::new(*user_share_token_account, false),
        AccountMeta::new(*lending_pool_info, false),
        AccountMeta::new(*pool_liquidity_token_account, false),
        AccountMeta::new(*share_token_mint, false),
        AccountMeta::new(*lending_market_info, false),
        AccountMeta::new_readonly(*lending_market_authority, false),
        AccountMeta::new_readonly(*user_transfer_authority, true),
        AccountMeta::new_readonly(*sysvar_clock, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: *lending_program_id,
        accounts,
        data,
    })
}

/// Withdraw liquidity form lending pool with share_amount
/// Redeem share token to get liquidity token
///   0. `[]` lending program id
///   1. `[writable]` user's share token account of user.
///                     $authority can transfer $share_amount.
///   2.  `[writable]` user's  liquidity token account of user.
///   3. `[writable]` Lending pool account
///   4. `[writable]` share mint account
///   5. `[writable]` Liquidity token account owned by lending pool's authority(PDA)
///   6. `[]` Lending market account
///   7. `[]` Lending market's authority (PDA)
///   8. `[signer]` User transfer authority ($authority).
///   9. `[]` Clock sysvar.
///   10. `[]` Token program id.
pub fn withdraw(
    lending_program_id: &Pubkey,
    user_share_token_account: &Pubkey,
    user_liquidity_token_account: &Pubkey,
    lending_pool_info: &Pubkey,
    share_token_mint: &Pubkey,
    pool_liquidity_token_account: &Pubkey,
    lending_market_info: &Pubkey,
    lending_market_authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    sysvar_clock: &Pubkey,
    token_program_id: &Pubkey,
    amount: u64
) -> Result<Instruction> {
    assert!(spl_token::id().eq(token_program_id));

    let data = WithdrawParam::serialize_to_vec(amount);

    let accounts = vec![
        AccountMeta::new(*user_share_token_account, false),
        AccountMeta::new(*user_liquidity_token_account, false),
        AccountMeta::new(*lending_pool_info, false),
        AccountMeta::new(*share_token_mint, false),
        AccountMeta::new(*pool_liquidity_token_account, false),
        AccountMeta::new(*lending_market_info, false),
        AccountMeta::new_readonly(*lending_market_authority, false),
        AccountMeta::new_readonly(*user_transfer_authority, true),
        AccountMeta::new_readonly(*sysvar_clock, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: *lending_program_id,
        accounts,
        data,
    })
}

/// Withdraw liquidity form lending pool with liquidity_amount directly
/// Redeem share token to get liquidity token
///   0. `[]` lending program id
///   1. `[writable]` user's share token account of user.
///                     $authority can transfer $share_amount.
///   2.  `[writable]` user's  liquidity token account of user.
///   3. `[writable]` Lending pool account
///   4. `[writable]` share mint account
///   5. `[writable]` Liquidity token account owned by lending pool's authority(PDA)
///   6. `[]` Lending market account
///   7. `[]` Lending market's authority (PDA)
///   8. `[signer]` User transfer authority ($authority).
///   9. `[]` Clock sysvar.
///   10. `[]` Token program id.
pub fn withdraw2(
    lending_program_id: &Pubkey,
    user_share_token_account: &Pubkey,
    user_liquidity_token_account: &Pubkey,
    lending_pool_info: &Pubkey,
    share_token_mint: &Pubkey,
    pool_liquidity_token_account: &Pubkey,
    lending_market_info: &Pubkey,
    lending_market_authority: &Pubkey,
    user_transfer_authority: &Pubkey,
    sysvar_clock: &Pubkey,
    token_program_id: &Pubkey,
    amount: u64
) -> Result<Instruction> {
    assert!(spl_token::id().eq(token_program_id));

    let data = WithdrawParam2::serialize_to_vec(amount);

    let accounts = vec![
        AccountMeta::new(*user_share_token_account, false),
        AccountMeta::new(*user_liquidity_token_account, false),
        AccountMeta::new(*lending_pool_info, false),
        AccountMeta::new(*share_token_mint, false),
        AccountMeta::new(*pool_liquidity_token_account, false),
        AccountMeta::new(*lending_market_info, false),
        AccountMeta::new_readonly(*lending_market_authority, false),
        AccountMeta::new_readonly(*user_transfer_authority, true),
        AccountMeta::new_readonly(*sysvar_clock, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];

    Ok(Instruction {
        program_id: *lending_program_id,
        accounts,
        data,
    })
}



/// Borrow from lending pool
/// Only strategy accounts which have credit_tokens can borrow from lending pool
///    0. `[writable]` lending_program_id
///    1. `[writable]` lending_pool_tkn_account
///    2. `[writable]` strategy_tkn_account
///    3. `[writable]` strategy_credit_account
///    4. `[writable]` lending_pool_credit_account
///    5. `[writable]` lending_pool_info_account
///    6. `[writable]` lending_market_account
///    7. `[writable]` lending_market_authority_info
///    8. `[writable]` strategy_authority_account
///    9. `[writable]` system_clock_info
///    10. `[writable]` token_program_id
pub fn borrow(
    lending_program_id: &Pubkey,
    lending_pool_tkn_account: &Pubkey,
    strategy_tkn_account: &Pubkey,
    strategy_credit_account: &Pubkey,
    lending_pool_credit_account: &Pubkey,
    lending_pool_info_account: &Pubkey,
    lending_market_account: &Pubkey,
    lending_market_authority_info: &Pubkey,
    strategy_authority_account: &Pubkey,
    system_clock_info: &Pubkey,
    token_program_id: &Pubkey,
    amount: u64,
) -> Result<Instruction> {
    let data = BorrowParam::serialize_to_vec(amount);

    let accounts = vec![
        AccountMeta::new(lending_pool_tkn_account.clone(), false),
        AccountMeta::new(strategy_tkn_account.clone(), false),
        AccountMeta::new(strategy_credit_account.clone(), false),
        AccountMeta::new(lending_pool_credit_account.clone(), false),
        AccountMeta::new(lending_pool_info_account.clone(), false),
        AccountMeta::new(lending_market_account.clone(), false),
        AccountMeta::new_readonly(lending_market_authority_info.clone(), false),
        AccountMeta::new_readonly(strategy_authority_account.clone(), true),
        AccountMeta::new_readonly(system_clock_info.clone(), false),
        AccountMeta::new_readonly(token_program_id.clone(), false),
    ];

    Ok(Instruction {
        program_id: *lending_program_id,
        accounts,
        data,
    })
}

/// repay
///    0. `[]` lending_program_id
///    1. `[writable]` strategy_tkn_account
///    2. `[writable]` lending_pool_tkn_account
///    3. `[writable]` strategy_credit_account
///    4. `[writable]` lending_pool_credit_mint
///    5. `[writable]` lending_pool_credit_account
///    6. `[writable]` lending_pool_info_account
///    7. `[writable]` lending_market_info
///    8. `[]` lending_market_authority_info
///    9. `[]` strategy_authority_info
///    10. `[]` system_clock_info
///    11. `[]` token_program_id
pub fn repay(
    lending_program_id: &Pubkey,
    strategy_tkn_account: &Pubkey,
    lending_pool_tkn_account: &Pubkey,
    strategy_credit_account: &Pubkey,
    lending_pool_credit_mint: &Pubkey,
    lending_pool_credit_account: &Pubkey,
    lending_pool_info_account: &Pubkey,
    lending_market_info: &Pubkey,
    lending_market_authority_info: &Pubkey,
    strategy_authority_info: &Pubkey,
    system_clock_info: &Pubkey,
    token_program_id: &Pubkey,
    amount: u64
) -> Result<Instruction> {
    let data = RepayParam::serialize_to_vec(amount);

    let accounts = vec![
        AccountMeta::new(strategy_tkn_account.clone(), false),
        AccountMeta::new(lending_pool_tkn_account.clone(), false),
        AccountMeta::new(strategy_credit_account.clone(), false),
        AccountMeta::new(lending_pool_credit_mint.clone(), false),
        AccountMeta::new(lending_pool_credit_account.clone(), false),
        AccountMeta::new(lending_pool_info_account.clone(), false),
        AccountMeta::new(lending_market_info.clone(), false),
        AccountMeta::new_readonly(lending_market_authority_info.clone(), false),
        AccountMeta::new_readonly(strategy_authority_info.clone(), true),
        AccountMeta::new_readonly(system_clock_info.clone(), false),
        AccountMeta::new_readonly(token_program_id.clone(), false),
    ];

    Ok(Instruction {
        program_id: *lending_program_id,
        accounts,
        data,
    })
}

