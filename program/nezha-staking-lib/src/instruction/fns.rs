use nezha_utils::account_meta;
use nezha_utils::account_meta as accounts;

use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
    sysvar::{clock, rent},
};
use spl_associated_token_account::get_associated_token_address;

use super::*;
use crate::accounts as ac;
use crate::francium::accounts as fr_ac;
use crate::francium::constants as fr_consts;
use nezha_vrf_lib::accounts as vrf_ac;

/// SuperAdmin: Initialize the contract.
pub fn init(
    program_id: &Pubkey,
    super_admin: &Pubkey,
    admin: &Pubkey,
    investor: &Pubkey,
    usdc_mint: &Pubkey,
    nezha_vrf_program_id: &Pubkey,
) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::Init,
        accounts![
            [signer writable] super_admin.clone(),
            [] admin.clone(),
            [] investor.clone(),
            [] usdc_mint.clone(),
            [] ac::vault_authority(program_id).pubkey,
            [writable] ac::deposit_vault(program_id).pubkey,
            [writable] ac::treasury_vault(program_id).pubkey,
            [writable] ac::insurance_vault(program_id).pubkey,
            [writable] ac::prize_vault(program_id, 1).pubkey,
            [writable] ac::prize_vault(program_id, 2).pubkey,
            [writable] ac::prize_vault(program_id, 3).pubkey,
            [writable] ac::pending_deposit_vault(program_id).pubkey,
            [writable] ac::latest_epoch(program_id).pubkey,
            [] *nezha_vrf_program_id,
            [] solana_program::system_program::id(),
            [] spl_token::id(),
            [] rent::id(),
        ],
    )
}

/// Request a deposit or withdraw.
/// If amount > 0, it's considered a deposit.
/// If amount < 0, it's considered a withdraw.
/// If amount == 0, it's an error.
///
/// Only one outstanding request is allowed.
/// To modify an existing request, cancel the existing one and issue a new one.
///
/// The request may then be executed by the admin, according to the epoch state.
pub fn request_stake_update(
    program_id: &Pubkey,
    owner: &Pubkey,
    owner_usdc_token: &Pubkey,
    amount: i64,
) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::RequestStakeUpdate { amount },
        accounts![
            [signer writable] owner.clone(),
            [writable] owner_usdc_token.clone(),
            [] ac::stake(program_id, owner).pubkey,
            [] ac::latest_epoch(program_id).pubkey,
            [writable] ac::stake_update_request(program_id, owner).pubkey,
            [writable] ac::pending_deposit_vault(program_id).pubkey,
            [] solana_program::system_program::id(),
            [] spl_token::id(),
            [] rent::id(),
        ],
    )
}

/// Admin: Mark a stake update request as approved.
pub fn approve_stake_update(program_id: &Pubkey, admin: &Pubkey, owner: &Pubkey, amount: i64) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::ApproveStakeUpdate { amount },
        accounts![
            [signer] admin.clone(),
            [] owner.clone(),
            [writable] ac::stake_update_request(program_id, owner).pubkey,
            [] ac::latest_epoch(program_id).pubkey,
        ],
    )
}

/// Admin|User: Cancel a stake update request.
pub fn cancel_stake_update(
    program_id: &Pubkey,
    admin: Option<&Pubkey>,
    owner: &Pubkey,
    owner_usdc_token: &Pubkey,
    amount: i64,
) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::CancelStakeUpdate { amount },
        accounts![
            [signer writable] admin.unwrap_or(owner).clone(),
            [] owner.clone(),
            [writable] owner_usdc_token.clone(),
            [writable] ac::stake_update_request(program_id, owner).pubkey,
            [writable] ac::pending_deposit_vault(program_id).pubkey,
            [] ac::vault_authority(program_id).pubkey,
            [] ac::latest_epoch(program_id).pubkey,
            [] spl_token::id(),
        ],
    )
}

/// Admin: Create a new epoch.
/// `YieldSplitCfg` defines how the yield of this epoch should be split.
pub fn create_epoch(
    program_id: &Pubkey,
    admin: &Pubkey,
    epoch_index: u64,
    expected_end_at: i64,
    yield_split_cfg: YieldSplitCfg,
) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::CreateEpoch {
            expected_end_at,
            yield_split_cfg,
        },
        accounts![
            [signer writable] admin.clone(),
            [writable] ac::epoch(program_id, epoch_index).pubkey,
            [writable] ac::latest_epoch(program_id).pubkey,
            [] solana_program::system_program::id(),
            [] rent::id(),
        ],
    )
}

/// User: Claim a prize and stake it.
///
/// `epoch_index` The index of the epoch in which the prize was one.
/// `page` The page number of the winners list.
/// `winner_index` The index of the winning entry in the page.
/// `tier` The tier in which the prize was won.
pub fn claim_winning(
    program_id: &Pubkey,
    owner: &Pubkey,
    epoch_index: u64,
    page: u32,
    winner_index: u32,
    tier: u8,
) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::ClaimWinning {
            epoch_index,
            page,
            winner_index,
            tier,
        },
        accounts![
            [] owner.clone(),
            [writable] ac::epoch_winners_meta(program_id, epoch_index).pubkey,
            [writable] ac::epoch_winners_page(program_id, epoch_index, page).pubkey,
            [writable] ac::stake(program_id, owner).pubkey,
            [] ac::epoch(program_id, epoch_index).pubkey,
            [writable] ac::latest_epoch(program_id).pubkey,
            [] ac::vault_authority(program_id).pubkey,
            [writable] ac::prize_vault(program_id, tier).pubkey,
            [writable] ac::deposit_vault(program_id).pubkey,
            [] spl_token::id(),
        ],
    )
}

/// Admin: Move the deposited funds into the account of a manual investor.
pub fn yield_withdraw_by_investor(
    program_id: &Pubkey,
    admin: &Pubkey,
    investor_usdc_token: &Pubkey,
    epoch_index: u64,
    tickets_info: TicketsInfo,
) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::YieldWithdrawByInvestor { tickets_info },
        accounts![
            [signer] admin.clone(),
            [writable] investor_usdc_token.clone(),
            [writable] ac::epoch(program_id, epoch_index).pubkey,
            [writable] ac::latest_epoch(program_id).pubkey,
            [] ac::vault_authority(program_id).pubkey,
            [writable] ac::deposit_vault(program_id).pubkey,
            [] spl_token::id(),
        ],
    )
}

/// Investor: Return the funds after manual investing and distribute the yield.
pub fn yield_deposit_by_investor(
    program_id: &Pubkey,
    investor: &Pubkey,
    investor_usdc_token: &Pubkey,
    epoch_index: u64,
    return_amount: u64,
) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::YieldDepositByInvestor { return_amount },
        accounts![
            [signer] investor.clone(),
            [writable] investor_usdc_token.clone(),
            [writable] ac::epoch(program_id, epoch_index).pubkey,
            [writable] ac::latest_epoch(program_id).pubkey,
            [writable] ac::deposit_vault(program_id).pubkey,
            [writable] ac::treasury_vault(program_id).pubkey,
            [writable] ac::insurance_vault(program_id).pubkey,
            [writable] ac::prize_vault(program_id, 2).pubkey,
            [writable] ac::prize_vault(program_id, 3).pubkey,
            [] spl_token::id(),
        ],
    )
}

/// Admin: Provide the funds for paying out the jackpot winner.
pub fn fund_jackpot(program_id: &Pubkey, funder: &Pubkey, funder_usdc_token: &Pubkey, epoch_index: u64) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::FundJackpot { epoch_index },
        accounts![
            [signer] funder.clone(),
            [writable] funder_usdc_token.clone(),
            [writable] ac::epoch(program_id, epoch_index).pubkey,
            [writable] ac::epoch_winners_meta(program_id, epoch_index).pubkey,
            [writable] ac::prize_vault(program_id, 1).pubkey,
            [] spl_token::id(),
        ],
    )
}

/// Admin: Initialize the accounts needed for francium investment.
pub fn francium_init(program_id: &Pubkey, admin: &Pubkey, mints: &fr_consts::Mints) -> Instruction {
    let francium_authority = ac::francium_authority(program_id).pubkey;
    let fr_usdc_ata = get_associated_token_address(&francium_authority, &mints.usdc_mint);
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::FranciumInit,
        accounts![
            [signer writable] admin.clone(),
            [] ac::latest_epoch(program_id).pubkey,
            [writable] francium_authority,
            //
            [writable] fr_usdc_ata,
            [writable] fr_ac::share_token_ata(&francium_authority, mints),
            [writable] fr_ac::rewards_token_ata(&francium_authority, mints),
            [writable] fr_ac::rewards_token_b_ata(&francium_authority, mints),
            [writable] fr_ac::farming_info(&francium_authority, mints),
            //
            [] mints.usdc_mint,
            [] mints.share_token_mint,
            [] mints.rewards_token_mint,
            [] mints.rewards_token_b_mint,
            //
            [] fr_consts::LENDING_REWARDS_PROGRAM_ID,
            [writable] fr_consts::FARMING_POOL,
            //
            [] solana_program::system_program::id(),
            [] spl_token::id(),
            [] spl_associated_token_account::id(),
            [] rent::id(),
        ],
    )
}

/// Admin: Move the deposited funds into francium.
pub fn francium_invest(
    program_id: &Pubkey,
    admin: &Pubkey,
    epoch_index: u64,
    tickets_info: TicketsInfo,
    mints: &fr_consts::Mints,
) -> Instruction {
    let francium_authority = ac::francium_authority(program_id).pubkey;
    let fr_usdc_ata = get_associated_token_address(&francium_authority, &mints.usdc_mint);
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::FranciumInvest { tickets_info },
        accounts![
            [signer writable] admin.clone(),
            [writable] francium_authority,
            [writable] ac::vault_authority(program_id).pubkey,
            //
            [writable] ac::latest_epoch(program_id).pubkey,
            [writable] ac::epoch(program_id, epoch_index).pubkey,
            [writable] ac::deposit_vault(program_id).pubkey,
            //
            [writable] fr_usdc_ata,
            [writable] fr_ac::share_token_ata(&francium_authority, mints),
            [writable] fr_ac::rewards_token_ata(&francium_authority, mints),
            [writable] fr_ac::rewards_token_b_ata(&francium_authority, mints),
            [writable] fr_ac::farming_info(&francium_authority, mints),
            //
            [] fr_consts::LENDING_PROGRAM_ID,
            [writable] fr_consts::LENDING_MARKET_INFO,
            [] fr_consts::LENDING_MARKET_AUTHORITY,
            [writable] fr_consts::LENDING_POOL_INFO,
            [writable] mints.share_token_mint,
            [writable] fr_consts::LENDING_POOL_USDC_ACCOUNT,
            //
            [] fr_consts::LENDING_REWARDS_PROGRAM_ID,
            [writable] fr_consts::FARMING_POOL,
            [writable] fr_consts::FARMING_POOL_AUTHORITY,
            [writable] fr_consts::FARMING_POOL_SHARE_TOKEN_ACCOUNT,
            [writable] fr_consts::FARMING_POOL_REWARDS_TOKEN_ACCOUNT,
            [writable] fr_consts::FARMING_POOL_REWARDS_TOKEN_B_ACCOUNT,
            //
            [] clock::id(),
            [] spl_token::id(),
        ],
    )
}

// Admin: Return the funds from francium and distribute the yield.
pub fn francium_withdraw(
    program_id: &Pubkey,
    admin: &Pubkey,
    epoch_index: u64,
    mints: &fr_consts::Mints,
) -> Instruction {
    let francium_authority = ac::francium_authority(program_id).pubkey;
    let fr_usdc_ata = get_associated_token_address(&francium_authority, &mints.usdc_mint);
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::FranciumWithdraw,
        accounts![
            [signer writable] admin.clone(),
            [writable] francium_authority,
            //
            [writable] ac::latest_epoch(program_id).pubkey,
            [writable] ac::epoch(program_id, epoch_index).pubkey,
            //
            [writable] ac::deposit_vault(program_id).pubkey,
            [writable] ac::treasury_vault(program_id).pubkey,
            [writable] ac::insurance_vault(program_id).pubkey,
            [writable] ac::prize_vault(program_id, 2).pubkey,
            [writable] ac::prize_vault(program_id, 3).pubkey,
            //
            [writable] fr_usdc_ata,
            [writable] fr_ac::share_token_ata(&francium_authority, mints),
            [writable] fr_ac::rewards_token_ata(&francium_authority, mints),
            [writable] fr_ac::rewards_token_b_ata(&francium_authority, mints),
            [writable] fr_ac::farming_info(&francium_authority, mints),
            //
            [writable] mints.share_token_mint,
            //
            [] fr_consts::LENDING_PROGRAM_ID,
            [writable] fr_consts::LENDING_MARKET_INFO,
            [] fr_consts::LENDING_MARKET_AUTHORITY,
            [writable] fr_consts::LENDING_POOL_INFO,
            [writable] fr_consts::LENDING_POOL_USDC_ACCOUNT,
            //
            [] fr_consts::LENDING_REWARDS_PROGRAM_ID,
            [writable] fr_consts::FARMING_POOL,
            [writable] fr_consts::FARMING_POOL_AUTHORITY,
            [writable] fr_consts::FARMING_POOL_SHARE_TOKEN_ACCOUNT,
            [writable] fr_consts::FARMING_POOL_REWARDS_TOKEN_ACCOUNT,
            [writable] fr_consts::FARMING_POOL_REWARDS_TOKEN_B_ACCOUNT,
            //
            [] clock::id(),
            [] spl_token::id(),
        ],
    )
}

/// Admin: Withdraw funds from any of the `WithdrawVault`.
pub fn withdraw_vault(
    program_id: &Pubkey,
    admin: &Pubkey,
    vault: WithdrawVault,
    destination: &Pubkey,
    amount: u64,
) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::WithdrawVault { vault, amount },
        accounts![
            [signer writable] admin.clone(),
            [] ac::vault_authority(program_id).pubkey,
            [writable] vault.get_pda(program_id).pubkey,
            [writable] *destination,
            [] ac::latest_epoch(program_id).pubkey,
            //
            [] spl_token::id(),
        ],
    )
}

/// Admin: Complete and process the stake update.
/// Currently, this is only allowed in the `Running` state of epochs.
/// A background process is supposed to monitor the active stake update requests and complete them
/// during the `Running` state.
pub fn complete_stake_update(
    program_id: &Pubkey,
    payer: &Pubkey,
    owner: &Pubkey,
    owner_usdc_token: &Pubkey,
) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::CompleteStakeUpdate,
        accounts![
            [signer writable] payer.clone(),
            [] owner.clone(),
            [] ac::vault_authority(program_id).pubkey,
            [writable] ac::stake_update_request(program_id, owner).pubkey,
            [writable] ac::stake(program_id, owner).pubkey,
            [] ac::latest_epoch(program_id).pubkey,
            [writable] ac::pending_deposit_vault(program_id).pubkey,
            [writable] ac::deposit_vault(program_id).pubkey,
            [writable] owner_usdc_token.clone(),
            [] spl_token::id(),
            [] solana_program::system_program::id(),
            [] rent::id(),
        ],
    )
}

/// Admin: Store the meta data needed for uploading winners of an epoch.
pub fn create_epoch_winners_meta(
    program_id: &Pubkey,
    admin: &Pubkey,
    epoch_index: u64,
    meta_args: CreateEpochWinnersMetaArgs,
    nezha_vrf_program_id: &Pubkey,
) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::CreateEpochWinnersMeta { meta_args },
        accounts![
            [signer writable] admin.clone(),
            [writable] ac::epoch_winners_meta(program_id, epoch_index).pubkey,
            [writable] ac::latest_epoch(program_id).pubkey,
            [writable] ac::epoch(program_id, epoch_index).pubkey,
            [] vrf_ac::nezha_vrf_request(nezha_vrf_program_id, epoch_index).pubkey,
            //
            [] system_program::id(),
            [] rent::id(),
        ],
    )
}

/// Admin: Upload the list of winners page by page.
pub fn publish_winners(
    program_id: &Pubkey,
    admin: &Pubkey,
    epoch_index: u64,
    page_index: u32,
    winners_input: Vec<WinnerInput>,
    nezha_vrf_program_id: &Pubkey,
) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::PublishWinners {
            page_index,
            winners_input,
        },
        accounts![
            [signer writable] admin.clone(),
            [writable] ac::latest_epoch(program_id).pubkey,
            [writable] ac::epoch(program_id, epoch_index).pubkey,
            [writable] ac::epoch_winners_meta(program_id, epoch_index).pubkey,
            [writable] ac::epoch_winners_page(program_id, epoch_index, page_index).pubkey,
            [] vrf_ac::nezha_vrf_request(nezha_vrf_program_id, epoch_index).pubkey,
            //
            [] system_program::id(),
            [] rent::id(),
        ],
    )
}

/// SuperAdmin: Rotate the key authorized as SuperAdmin, Admin, or Investor.
pub fn rotate_key(program_id: &Pubkey, super_admin: &Pubkey, key_type: RotateKeyType, new_key: &Pubkey) -> Instruction {
    Instruction::new_with_borsh(
        program_id.clone(),
        &StakingInstruction::RotateKey { key_type },
        accounts![
            [signer] super_admin.clone(),
            [writable] ac::latest_epoch(program_id).pubkey,
            [] new_key.clone(),
            //
        ],
    )
}
