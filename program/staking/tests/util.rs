use anchor_client::solana_sdk::{
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
};

use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::system_instruction;
use solana_program_test::*;
use solana_sdk::{instruction::Instruction, program_pack::Pack, sysvar, transaction::Transaction};
use staking::BALANCE_SEED;

pub async fn mint_to(context: &mut ProgramTestContext, mint: &Pubkey, to: &Pubkey, amount: u64, decimals: u8) {
    let tx = Transaction::new_signed_with_payer(
        &[spl_token::instruction::mint_to(
            &spl_token::id(),
            mint,
            to,
            &context.payer.pubkey(),
            &[],
            amount * 10u64.pow(decimals as u32),
        )
        .unwrap()],
        Some(&context.payer.pubkey()),
        &[&context.payer],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();
}

pub async fn create_token_account(
    context: &mut ProgramTestContext,
    account: &Keypair,
    mint: &Pubkey,
    manager: &Pubkey,
) {
    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &account.pubkey(),
                rent.minimum_balance(spl_token::state::Account::LEN),
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(&spl_token::id(), &account.pubkey(), mint, manager).unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, &account],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();
}

pub async fn create_mint(
    context: &mut ProgramTestContext,
    mint: &Keypair,
    authority: &Pubkey,
    freeze_authority: Option<&Pubkey>,
    decimals: u8,
) {
    let rent = context.banks_client.get_rent().await.unwrap();

    let tx = Transaction::new_signed_with_payer(
        &[
            system_instruction::create_account(
                &context.payer.pubkey(),
                &mint.pubkey(),
                rent.minimum_balance(spl_token::state::Mint::LEN),
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint.pubkey(),
                authority,
                freeze_authority,
                decimals,
            )
            .unwrap(),
        ],
        Some(&context.payer.pubkey()),
        &[&context.payer, mint],
        context.last_blockhash,
    );

    context.banks_client.process_transaction(tx).await.unwrap();
}

#[derive(Debug)]
pub struct Deposit {
    pub owner: Keypair,
    pub mint: Keypair,
    pub token: Pubkey,
    pub user_balance: Pubkey,
    pub vault: Pubkey,
    pub vault_token: Pubkey,
    pub program_id: Pubkey,
    pub amount: u64,
}

pub async fn deposit(ctx: &mut ProgramTestContext, owner: Keypair, amount: u64) -> Deposit {
    let mint = Keypair::new();
    let decimals = 6;
    let program_id = staking::id();
    let token = Keypair::new();

    create_mint(ctx, &mint, &ctx.payer.pubkey(), None, decimals).await;
    create_token_account(ctx, &token, &mint.pubkey(), &owner.pubkey()).await;
    mint_to(ctx, &mint.pubkey(), &token.pubkey(), amount * 2, decimals).await;

    let user_balance = Pubkey::find_program_address(
        &[BALANCE_SEED.as_ref(), owner.pubkey().as_ref(), mint.pubkey().as_ref()],
        &program_id,
    )
    .0;

    let vault_token =
        Pubkey::find_program_address(&[staking::VAULT_SEED.as_ref(), mint.pubkey().as_ref()], &program_id).0;
    let vault = Pubkey::find_program_address(&[staking::VAULT_SEED.as_ref()], &program_id).0;

    let init_accounts = staking::accounts::Deposit {
        owner: owner.pubkey(),
        user_balance,
        token: token.pubkey(),
        mint: mint.pubkey(),
        vault,
        vault_token: vault_token.clone(),
        clock: sysvar::clock::id(),
        rent: sysvar::rent::id(),
        system_program: solana_program::system_program::ID,
        token_program: anchor_spl::token::ID,
    };

    let deposit_args = staking::instruction::Deposit { amount };

    let tx = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id,
            accounts: init_accounts.to_account_metas(None),
            data: deposit_args.data(),
        }],
        Some(&owner.pubkey()),
        &[&owner],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await.unwrap();

    Deposit {
        owner,
        mint,
        user_balance,
        vault,
        vault_token,
        program_id,
        amount,
        token: token.pubkey(),
    }
}

pub async fn withdraw(ctx: &mut ProgramTestContext, deposit: &Deposit, amount: u64) -> anyhow::Result<()> {
    let Deposit {
        owner,
        mint,
        token,
        user_balance: _,
        vault: _,
        vault_token: _,
        program_id,
        amount: _,
    } = deposit;

    let user_balance = Pubkey::find_program_address(
        &[BALANCE_SEED.as_ref(), owner.pubkey().as_ref(), mint.pubkey().as_ref()],
        &program_id,
    )
    .0;

    let vault_token =
        Pubkey::find_program_address(&[staking::VAULT_SEED.as_ref(), mint.pubkey().as_ref()], &program_id).0;
    let vault = Pubkey::find_program_address(&[staking::VAULT_SEED.as_ref()], &program_id).0;

    let init_accounts = staking::accounts::Withdraw {
        owner: owner.pubkey(),
        user_balance,
        token: token.clone(),
        vault,
        vault_token: vault_token.clone(),
        clock: sysvar::clock::id(),
        system_program: solana_program::system_program::ID,
        token_program: anchor_spl::token::ID,
    };

    let deposit_args = staking::instruction::Withdraw { amount };

    let tx = Transaction::new_signed_with_payer(
        &[Instruction {
            program_id: *program_id,
            accounts: init_accounts.to_account_metas(None),
            data: deposit_args.data(),
        }],
        Some(&owner.pubkey()),
        &[owner],
        ctx.last_blockhash,
    );

    ctx.banks_client.process_transaction(tx).await.unwrap();

    Ok(())
}
