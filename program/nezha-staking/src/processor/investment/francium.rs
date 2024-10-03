//! Francium investment.
use borsh::BorshDeserialize;
use nezha_utils::load_accounts;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction, sysvar,
};
use spl_associated_token_account::instruction as ata_ixns;
use spl_token::amount_to_ui_amount;

use crate::{
    accounts as ac,
    accounts::VerifyPDA,
    error::StakingError,
    fixed_point::FPUSDC,
    francium::accounts as fr_accounts,
    francium::constants as fr_consts,
    francium::instruction as fr_ixns,
    state::{LatestEpoch, TicketsInfo},
    utils::*,
};

use nezha_utils::checks::*;

/// Init the accounts needed for investing into Francium protocol
pub fn process_francium_init<'a>(program_id: &Pubkey, accounts: &[AccountInfo<'a>]) -> ProgramResult {
    msg!("Ixn: Francium Init");

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        admin,
        latest_epoch,
        francium_authority,
        //
        usdc_token_ata,
        share_token_ata,
        rewards_token_ata,
        rewards_token_b_ata,
        farming_info,
        //
        liquidity_token_mint,
        share_token_mint,
        rewards_token_mint,
        rewards_token_b_mint,
        //
        rewards_program,
        farming_pool,
        //
        system_program,
        token_program,
        ata_program,
        rent_sysvar,
    );

    // Checks

    ac::latest_epoch(program_id).verify(latest_epoch)?;
    let latest_epoch = LatestEpoch::try_from_slice(&latest_epoch.data.borrow())?;

    check_admin(admin, &latest_epoch)?;

    let francium_authority_pda = ac::francium_authority(program_id);
    francium_authority_pda.verify(francium_authority)?;

    let mints = fr_consts::get_mints();
    check_ata_account(
        "usdc_token_ata",
        usdc_token_ata.key,
        francium_authority.key,
        &mints.usdc_mint,
    )?;
    check_ata_account(
        "share_token_ata",
        share_token_ata.key,
        francium_authority.key,
        &mints.share_token_mint,
    )?;
    check_ata_account(
        "rewards_token_ata",
        rewards_token_ata.key,
        francium_authority.key,
        &mints.rewards_token_mint,
    )?;
    check_ata_account(
        "rewards_token_b_ata",
        rewards_token_b_ata.key,
        francium_authority.key,
        &mints.rewards_token_b_mint,
    )?;
    check_pubkey(
        "farming_info",
        farming_info,
        &fr_accounts::farming_info(francium_authority.key, &mints),
    )?;

    check_pubkey("liquidity_token_mint", liquidity_token_mint, &mints.usdc_mint)?;
    check_pubkey("share_token_mint", share_token_mint, &mints.share_token_mint)?;
    check_pubkey("rewards_token_mint", rewards_token_mint, &mints.rewards_token_mint)?;
    check_pubkey(
        "rewards_token_b_mint",
        rewards_token_b_mint,
        &mints.rewards_token_b_mint,
    )?;

    check_pubkey(
        "rewards_program",
        rewards_program,
        &fr_consts::LENDING_REWARDS_PROGRAM_ID,
    )?;
    check_pubkey("farming_pool", farming_pool, &fr_consts::FARMING_POOL)?;

    check_system_program(system_program)?;
    check_token_program(token_program)?;
    check_ata_program(ata_program)?;
    check_rent_sysvar(rent_sysvar)?;

    // End Of Checks

    let create_ata = |mint: &AccountInfo<'a>, ata: &AccountInfo<'a>| -> ProgramResult {
        invoke(
            &ata_ixns::create_associated_token_account(admin.key, francium_authority.key, mint.key, token_program.key),
            &[
                admin.clone(),
                ata.clone(),
                francium_authority.clone(),
                mint.clone(),
                system_program.clone(),
                token_program.clone(),
                ata_program.clone(),
            ],
        )?;
        Ok(())
    };

    msg!("Create Liquidity Token ATA");
    create_ata(liquidity_token_mint, usdc_token_ata)?;
    msg!("Create Share Token ATA");
    create_ata(share_token_mint, share_token_ata)?;
    msg!("Create Rewards Token ATA");
    create_ata(rewards_token_mint, rewards_token_ata)?;
    msg!("Create Rewards Token B ATA");
    create_ata(rewards_token_b_mint, rewards_token_b_ata)?;

    msg!("Transfer SOLs to francium authority");
    invoke(
        &system_instruction::transfer(
            admin.key,
            francium_authority.key,
            3069360, // fr_ixns::init_farming_info() seems to need these many lamports in francium_authority
        ),
        &[admin.clone(), francium_authority.clone(), system_program.clone()],
    )
    .map_err(StakingError::system_program_error)?;

    msg!("Init farming info");
    invoke_signed(
        &fr_ixns::init_farming_info(
            rewards_program.key,
            francium_authority.key,
            farming_info.key,
            farming_pool.key,
            share_token_ata.key,
            rewards_token_ata.key,
            rewards_token_b_ata.key,
            system_program.key,
            rent_sysvar.key,
        ),
        &[
            rewards_program.clone(),
            francium_authority.clone(),
            farming_info.clone(),
            farming_pool.clone(),
            share_token_ata.clone(),
            rewards_token_ata.clone(),
            rewards_token_b_ata.clone(),
            system_program.clone(),
            rent_sysvar.clone(),
        ],
        &[&francium_authority_pda.seeds()],
    )
    .map_err(StakingError::francium_farming_error)?;

    Ok(())
}

/// Invest into francium
pub fn process_francium_invest<'a>(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'a>],
    tickets_info: TicketsInfo,
) -> ProgramResult {
    msg!("Ixn: Francium Invest");

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        admin,
        francium_authority,
        vault_authority,
        //
        latest_epoch,
        epoch,
        deposit_vault,
        //
        usdc_token_ata,
        share_token_ata,
        rewards_token_ata,
        rewards_token_b_ata,
        farming_info,
        //
        lending_program,
        lending_market_info,
        lending_market_authority,
        lending_pool_info,
        share_token_mint,
        lending_pool_liquidity_token_account,
        //
        lending_rewards_program,
        farming_pool,
        farming_pool_authority,
        farming_pool_share_token_account,
        farming_pool_rewards_token_account,
        farming_pool_rewards_token_b_account,
        //
        clock_sysvar,
        token_program,
    );

    // Checks

    ac::latest_epoch(program_id).verify(latest_epoch)?;
    let latest_epoch_ = LatestEpoch::try_from_slice(&latest_epoch.data.borrow())?;

    check_admin(admin, &latest_epoch_)?;

    let francium_authority_pda = ac::francium_authority(program_id);
    francium_authority_pda.verify(francium_authority)?;

    let vault_authority_pda = ac::vault_authority(program_id);
    vault_authority_pda.verify(vault_authority)?;

    ac::epoch(program_id, latest_epoch_.index).verify(epoch)?;
    ac::deposit_vault(program_id).verify(deposit_vault)?;

    let mints = fr_consts::get_mints();

    check_ata_account(
        "usdc_token_ata",
        usdc_token_ata.key,
        francium_authority.key,
        &mints.usdc_mint,
    )?;
    check_ata_account(
        "share_token_ata",
        share_token_ata.key,
        francium_authority.key,
        &mints.share_token_mint,
    )?;
    check_ata_account(
        "rewards_token_ata",
        rewards_token_ata.key,
        francium_authority.key,
        &mints.rewards_token_mint,
    )?;
    check_ata_account(
        "rewards_token_b_ata",
        rewards_token_b_ata.key,
        francium_authority.key,
        &mints.rewards_token_b_mint,
    )?;

    check_pubkey(
        "farming_info",
        farming_info,
        &fr_accounts::farming_info(francium_authority.key, &mints),
    )?;

    check_pubkey("lending_program", lending_program, &fr_consts::LENDING_PROGRAM_ID)?;
    check_pubkey(
        "lending_market_info",
        lending_market_info,
        &fr_consts::LENDING_MARKET_INFO,
    )?;
    check_pubkey(
        "lending_market_authority",
        lending_market_authority,
        &fr_consts::LENDING_MARKET_AUTHORITY,
    )?;
    check_pubkey("lending_pool_info", lending_pool_info, &fr_consts::LENDING_POOL_INFO)?;
    check_pubkey("share_token_mint", share_token_mint, &mints.share_token_mint)?;
    check_pubkey(
        "lending_pool_liquidity_token_account",
        lending_pool_liquidity_token_account,
        &fr_consts::LENDING_POOL_USDC_ACCOUNT,
    )?;

    check_pubkey(
        "lending_rewards_program",
        lending_rewards_program,
        &fr_consts::LENDING_REWARDS_PROGRAM_ID,
    )?;
    check_pubkey("farming_pool", farming_pool, &fr_consts::FARMING_POOL)?;
    check_pubkey(
        "farming_pool_authority",
        farming_pool_authority,
        &fr_consts::FARMING_POOL_AUTHORITY,
    )?;
    check_pubkey(
        "farming_pool_share_token_account",
        farming_pool_share_token_account,
        &fr_consts::FARMING_POOL_SHARE_TOKEN_ACCOUNT,
    )?;
    check_pubkey(
        "farming_pool_rewards_token_account",
        farming_pool_rewards_token_account,
        &fr_consts::FARMING_POOL_REWARDS_TOKEN_ACCOUNT,
    )?;
    check_pubkey(
        "farming_pool_rewards_token_b_account",
        farming_pool_rewards_token_b_account,
        &fr_consts::FARMING_POOL_REWARDS_TOKEN_B_ACCOUNT,
    )?;

    check_token_program(token_program)?;
    check_clock_sysvar(clock_sysvar)?;

    // End Of Checks

    let destination_account = usdc_token_ata;

    // send all usdc token from vault to investor for yield
    let vault_usdc_amount = FPUSDC::from_usdc(spl_token::state::Account::unpack(&deposit_vault.data.borrow())?.amount);

    super::invest(
        program_id,
        deposit_vault,
        vault_authority,
        destination_account,
        latest_epoch,
        epoch,
        token_program,
        tickets_info,
        vault_usdc_amount,
    )?;

    let invested_amount = vault_usdc_amount.as_usdc();

    msg!("Update lending pool");
    invoke(
        &fr_ixns::update_lending_pool(
            lending_program.key,
            lending_market_info.key,
            lending_pool_info.key,
            clock_sysvar.key,
        ),
        &[
            lending_program.clone(),
            lending_market_info.clone(),
            lending_pool_info.clone(),
            clock_sysvar.clone(),
        ],
    )
    .map_err(StakingError::francium_lending_error)?;

    msg!("Deposit");
    invoke_signed(
        &fr_ixns::deposit(
            lending_program.key,
            usdc_token_ata.key,
            share_token_ata.key,
            lending_pool_info.key,
            lending_pool_liquidity_token_account.key,
            share_token_mint.key,
            lending_market_info.key,
            lending_market_authority.key,
            francium_authority.key,
            clock_sysvar.key,
            token_program.key,
            invested_amount,
        ),
        &[
            lending_program.clone(),
            usdc_token_ata.clone(),
            share_token_ata.clone(),
            lending_pool_info.clone(),
            lending_pool_liquidity_token_account.clone(),
            share_token_mint.clone(),
            lending_market_info.clone(),
            lending_market_authority.clone(),
            francium_authority.clone(),
            clock_sysvar.clone(),
            token_program.clone(),
        ],
        &[&francium_authority_pda.seeds()],
    )
    .map_err(StakingError::francium_lending_error)?;

    msg!("Stake to farming pool");
    invoke_signed(
        &fr_ixns::stake_to_farming_pool(
            &lending_rewards_program.key,
            &francium_authority.key,
            &farming_info.key,
            &share_token_ata.key,
            &rewards_token_ata.key,
            &rewards_token_b_ata.key,
            &farming_pool.key,
            &farming_pool_authority.key,
            &farming_pool_share_token_account.key,
            &farming_pool_rewards_token_account.key,
            &farming_pool_rewards_token_b_account.key,
            &token_program.key,
            &clock_sysvar.key,
            0, // taken directly from the CLI (not sure if this is supposed to be 0)
        ),
        &[
            lending_rewards_program.clone(),
            francium_authority.clone(),
            farming_info.clone(),
            share_token_ata.clone(),
            rewards_token_ata.clone(),
            rewards_token_b_ata.clone(),
            farming_pool.clone(),
            farming_pool_authority.clone(),
            farming_pool_share_token_account.clone(),
            farming_pool_rewards_token_account.clone(),
            farming_pool_rewards_token_b_account.clone(),
            token_program.clone(),
            clock_sysvar.clone(),
        ],
        &[&francium_authority_pda.seeds()],
    )
    .map_err(StakingError::francium_farming_error)?;

    Ok(())
}

/// Withdraw investment from francium
pub fn process_francium_withdraw<'a>(program_id: &Pubkey, accounts: &[AccountInfo<'a>]) -> ProgramResult {
    msg!("Ixn: Francium withdraw");

    let account_info_iter = &mut accounts.iter();

    load_accounts!(
        account_info_iter,
        //
        admin,
        francium_authority,
        //
        latest_epoch,
        epoch,
        //
        deposit_vault,
        treasury_vault,
        insurance_vault,
        tier2_prize_vault,
        tier3_prize_vault,
        //
        usdc_token_ata,
        share_token_ata,
        rewards_token_ata,
        rewards_token_b_ata,
        //
        farming_info,
        //
        share_token_mint,
        //
        lending_program,
        lending_market_info,
        lending_market_authority,
        lending_pool_info,
        lending_pool_usdc_account,
        //
        lending_rewards_program,
        farming_pool,
        farming_pool_authority,
        farming_pool_share_token_account,
        farming_pool_rewards_token_account,
        farming_pool_rewards_token_b_account,
        //
        clock_sysvar,
        token_program,
    );

    ac::latest_epoch(program_id).verify(latest_epoch)?;
    let latest_epoch_ = LatestEpoch::try_from_slice(&latest_epoch.data.borrow())?;

    check_admin(admin, &latest_epoch_)?;

    ac::epoch(program_id, latest_epoch_.index).verify(epoch)?;

    let francium_authority_pda = ac::francium_authority(program_id);
    francium_authority_pda.verify(francium_authority)?;

    ac::deposit_vault(program_id).verify(deposit_vault)?;
    ac::treasury_vault(program_id).verify(treasury_vault)?;
    ac::insurance_vault(program_id).verify(insurance_vault)?;
    ac::prize_vault(program_id, 2).verify(tier2_prize_vault)?;
    ac::prize_vault(program_id, 3).verify(tier3_prize_vault)?;

    let mints = fr_consts::get_mints();

    check_ata_account(
        "usdc_token_ata",
        usdc_token_ata.key,
        francium_authority.key,
        &mints.usdc_mint,
    )?;
    check_ata_account(
        "share_token_ata",
        share_token_ata.key,
        francium_authority.key,
        &mints.share_token_mint,
    )?;
    check_ata_account(
        "rewards_token_ata",
        rewards_token_ata.key,
        francium_authority.key,
        &mints.rewards_token_mint,
    )?;
    check_ata_account(
        "rewards_token_b_ata",
        rewards_token_b_ata.key,
        francium_authority.key,
        &mints.rewards_token_b_mint,
    )?;

    check_pubkey(
        "farming_info",
        farming_info,
        &fr_accounts::farming_info(francium_authority.key, &mints),
    )?;

    check_pubkey("share_token_mint", share_token_mint, &mints.share_token_mint)?;

    check_pubkey("lending_program", lending_program, &fr_consts::LENDING_PROGRAM_ID)?;
    check_pubkey(
        "lending_market_info",
        lending_market_info,
        &fr_consts::LENDING_MARKET_INFO,
    )?;
    check_pubkey(
        "lending_market_authority",
        lending_market_authority,
        &fr_consts::LENDING_MARKET_AUTHORITY,
    )?;
    check_pubkey("lending_pool_info", lending_pool_info, &fr_consts::LENDING_POOL_INFO)?;
    check_pubkey(
        "lending_pool_usdc_account",
        lending_pool_usdc_account,
        &fr_consts::LENDING_POOL_USDC_ACCOUNT,
    )?;

    check_pubkey(
        "lending_rewards_program",
        lending_rewards_program,
        &fr_consts::LENDING_REWARDS_PROGRAM_ID,
    )?;
    check_pubkey("farming_pool", farming_pool, &fr_consts::FARMING_POOL)?;
    check_pubkey(
        "farming_pool_authority",
        farming_pool_authority,
        &fr_consts::FARMING_POOL_AUTHORITY,
    )?;
    check_pubkey(
        "farming_pool_share_token_account",
        farming_pool_share_token_account,
        &fr_consts::FARMING_POOL_SHARE_TOKEN_ACCOUNT,
    )?;
    check_pubkey(
        "farming_pool_rewards_token_account",
        farming_pool_rewards_token_account,
        &fr_consts::FARMING_POOL_REWARDS_TOKEN_ACCOUNT,
    )?;
    check_pubkey(
        "farming_pool_rewards_token_b_account",
        farming_pool_rewards_token_b_account,
        &fr_consts::FARMING_POOL_REWARDS_TOKEN_B_ACCOUNT,
    )?;

    check_clock_sysvar(clock_sysvar)?;
    check_token_program(token_program)?;

    // End Of Checks

    msg!("Update lending pool");
    invoke(
        &fr_ixns::update_lending_pool(
            lending_program.key,
            lending_market_info.key,
            lending_pool_info.key,
            clock_sysvar.key,
        ),
        &[
            lending_program.clone(),
            lending_market_info.clone(),
            lending_pool_info.clone(),
            clock_sysvar.clone(),
        ],
    )
    .map_err(StakingError::francium_lending_error)?;

    let farming_user =
        francium_lending_rewards_pool::state::farming_user::FarmingUser::unpack(&farming_info.try_borrow_data()?)?;

    let rewards_amount = farming_user.staked_amount;
    msg!("Unstaking rewards {}", amount_to_ui_amount(rewards_amount, 6));

    invoke_signed(
        &fr_ixns::unstake_from_farming_pool(
            &lending_rewards_program.key,
            &francium_authority.key,
            &farming_info.key,
            &share_token_ata.key,
            &rewards_token_ata.key,
            &rewards_token_b_ata.key,
            &farming_pool.key,
            &farming_pool_authority.key,
            &farming_pool_share_token_account.key,
            &farming_pool_rewards_token_account.key,
            &farming_pool_rewards_token_b_account.key,
            &spl_token::id(),
            &sysvar::clock::id(),
            rewards_amount,
        ),
        &[
            lending_rewards_program.clone(),
            francium_authority.clone(),
            farming_info.clone(),
            share_token_ata.clone(),
            rewards_token_ata.clone(),
            rewards_token_b_ata.clone(),
            farming_pool.clone(),
            farming_pool_authority.clone(),
            farming_pool_share_token_account.clone(),
            farming_pool_rewards_token_account.clone(),
            farming_pool_rewards_token_b_account.clone(),
            clock_sysvar.clone(),
            token_program.clone(),
        ],
        &[&francium_authority_pda.seeds()],
    )
    .map_err(StakingError::francium_farming_error)?;

    let share_token_balance = {
        let share_token_ata_account = spl_token::state::Account::unpack(&share_token_ata.try_borrow_data()?)?;

        share_token_ata_account.amount
    };

    msg!("Share token balance {}", amount_to_ui_amount(share_token_balance, 6));

    invoke_signed(
        &fr_ixns::withdraw(
            &lending_program.key,
            &share_token_ata.key,
            &usdc_token_ata.key,
            &lending_pool_info.key,
            &share_token_mint.key,
            &lending_pool_usdc_account.key,
            &lending_market_info.key,
            &lending_market_authority.key,
            &francium_authority.key,
            &clock_sysvar.key,
            &token_program.key,
            share_token_balance,
        ),
        &[
            lending_program.clone(),
            share_token_ata.clone(),
            usdc_token_ata.clone(),
            lending_pool_info.clone(),
            share_token_mint.clone(),
            lending_pool_usdc_account.clone(),
            lending_market_info.clone(),
            lending_market_authority.clone(),
            francium_authority.clone(),
            clock_sysvar.clone(),
            token_program.clone(),
        ],
        &[&francium_authority_pda.seeds()],
    )
    .map_err(StakingError::francium_lending_error)?;

    let amount = {
        let usdc_token_ata_data = usdc_token_ata.try_borrow_data()?;
        let usdc_token_ata_account = spl_token::state::Account::unpack(&usdc_token_ata_data)?;

        usdc_token_ata_account.amount
    };

    msg!("Francium returns {}", amount_to_ui_amount(amount, 6));

    let source_account_info = usdc_token_ata;
    let source_authority_info = francium_authority;
    let return_amount = amount;

    super::withdraw(
        program_id,
        latest_epoch,
        epoch,
        source_account_info,
        source_authority_info,
        Some(&francium_authority_pda.seeds()),
        deposit_vault,
        treasury_vault,
        insurance_vault,
        tier2_prize_vault,
        tier3_prize_vault,
        token_program,
        return_amount,
    )?;
    Ok(())
}
