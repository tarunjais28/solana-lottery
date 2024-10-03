use francium_lending_pool::instruction::LendingInstruction;
use francium_lending_rewards_pool::{instruction::FarmingInstructions, state::farming_user::FarmingUser};
use nezha_staking_lib::fixed_point::FPUSDC;
use nezha_utils::load_accounts;
use solana_program::program::invoke_signed;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, program_pack::Pack};
use solana_program::{msg, system_instruction};
use std::cell::RefCell;
use std::thread::LocalKey;

thread_local! {
    static FRANCIUM_RETURN_RATE: std::cell::RefCell<FPUSDC>  = RefCell::new(FPUSDC::zero());
    static FRANCIUM_DEPOSIT: std::cell::RefCell<FPUSDC>  = RefCell::new(FPUSDC::zero());
}

pub fn init(return_rate: FPUSDC) {
    set_thread_local_refcell(&FRANCIUM_RETURN_RATE, return_rate);
}

pub fn process_francium_lending(_program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let ixn = LendingInstruction::unpack(input)?;
    match ixn {
        LendingInstruction::UpdateLendingPool => {
            msg!("Fr Ixn: UpdateLendingPool");
        }
        LendingInstruction::DepositToLendingPool { liquidity_amount } => {
            msg!("Fr Ixn: DepositToLendingPool");
            deposit(FPUSDC::from_usdc(liquidity_amount));
        }
        LendingInstruction::WithdrawFromLendingPool { share_amount: _ } => {
            msg!("Fr Ixn: WithdrawFromLendingPool");
            let user_usdc_token_info = &accounts[1];
            let amount = get_deposit();
            withdraw(amount);
            let mut token_account =
                spl_token::state::Account::unpack(&user_usdc_token_info.try_borrow_mut_data().unwrap())?;
            token_account.amount += amount.as_usdc();
            spl_token::state::Account::pack(token_account, &mut user_usdc_token_info.try_borrow_mut_data().unwrap())?;
        }
        _ => unimplemented!(),
    }
    Ok(())
}

pub fn process_francium_rewards(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let ixn = FarmingInstructions::unpack(input)?;
    match ixn {
        FarmingInstructions::InitFarmingUser => {
            msg!("Fr Ixn: InitFarmingUser");
            // noop
        }
        FarmingInstructions::Stake { amount: _ } => {
            msg!("Fr Ixn: Stake");
            let accounts_iter = &mut accounts.iter();
            load_accounts!(
                accounts_iter,
                //
                user_info,
                user_farming_info,
                user_stake_token_info,
                _user_rewards_info,
                _user_rewards_b_info,
                farming_pool_info,
                _farming_pool_authority_info,
                _farming_pool_stake_token_info,
                farming_pool_rewards_token_info,
                farming_pool_rewards_b_token_info,
                _token_program_info,
                _sysvar_clock_info,
            );
            let farming_user = FarmingUser {
                version: 1,
                staked_amount: 0,
                rewards_debt: 0,
                rewards_debt_b: 0,
                farming_pool: *farming_pool_info.key,
                user_main: *user_info.key,
                stake_token_account: *user_stake_token_info.key,
                rewards_token_accont: *farming_pool_rewards_token_info.key,
                rewards_token_account_b: *farming_pool_rewards_b_token_info.key,
            };
            let account_length = FarmingUser::get_packed_len();
            let rent = Rent::get()?;
            let required_lamports = rent.minimum_balance(account_length);

            let seeds = &[
                user_info.key.as_ref(),
                farming_pool_info.key.as_ref(),
                user_stake_token_info.key.as_ref(),
            ];

            let (_, bump) = Pubkey::find_program_address(seeds, &program_id);

            invoke_signed(
                &system_instruction::create_account(
                    user_info.key,
                    user_farming_info.key,
                    required_lamports,
                    account_length as u64,
                    &program_id,
                ),
                &[user_info.clone(), user_farming_info.clone()],
                &[&[
                    user_info.key.as_ref(),
                    farming_pool_info.key.as_ref(),
                    user_stake_token_info.key.as_ref(),
                    &[bump],
                ]],
            )?;
            Pack::pack(farming_user, &mut user_farming_info.try_borrow_mut_data()?)?;
        }
        FarmingInstructions::UnStake { amount: _ } => {
            msg!("Fr Ixn: UnStake");
            let accounts_iter = &mut accounts.iter();
            load_accounts!(
                accounts_iter,
                //
                _user_info,
                _user_farming_info,
                _user_stake_token_info,
                _user_rewards_info,
                _user_rewards_b_info,
                _farming_pool_info,
                _farming_pool_authority_info,
                _farming_pool_stake_token_info,
                _farming_pool_rewards_token_info,
                _farming_pool_rewards_b_token_info,
                _token_program_info,
                _sysvar_clock_info,
            );
        }
        _ => unimplemented!(),
    }
    Ok(())
}

fn get_return_rate() -> FPUSDC {
    let return_rate = get_thread_local_refcell(&FRANCIUM_RETURN_RATE);
    assert_ne!(
        return_rate,
        FPUSDC::zero(),
        "Francium rate is zero. Did you call init_return_rate()?"
    );
    return_rate
}

fn deposit(amount: FPUSDC) {
    let return_rate = get_return_rate();
    let amount_appreciated = amount.checked_mul(return_rate).unwrap();
    modify_thread_local_refcell(&FRANCIUM_DEPOSIT, |x| x.checked_add(amount_appreciated).unwrap());
}

fn get_deposit() -> FPUSDC {
    get_thread_local_refcell(&FRANCIUM_DEPOSIT)
}

fn withdraw(amount: FPUSDC) {
    modify_thread_local_refcell(&FRANCIUM_DEPOSIT, |x| x.checked_sub(amount).unwrap());
}

// Local Key Helpers

fn get_thread_local_refcell<T: Copy>(key: &'static LocalKey<RefCell<T>>) -> T {
    key.with(|x| *x.borrow())
}

fn set_thread_local_refcell<T>(key: &'static LocalKey<RefCell<T>>, val: T) {
    key.with(|x| {
        let mut refm_ = x.borrow_mut();
        *refm_ = val;
    })
}

fn modify_thread_local_refcell<T: Copy>(key: &'static LocalKey<RefCell<T>>, f: impl FnOnce(T) -> T) {
    key.with(|x| {
        let mut refm_ = x.borrow_mut();
        *refm_ = f(*refm_);
    })
}
