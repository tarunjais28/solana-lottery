use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("2beVdAd5fpgyxwspZBfJGaqTLe2sZBm1KkBxiZFc1Mjr");

pub const BALANCE_SEED: &str = "user_balance";
pub const VAULT_SEED: &str = "vault";
pub const UNBONDING_SECONDS: i64 = 0;

#[program]
pub mod staking {

    use super::*;

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let balance = &mut ctx.accounts.user_balance;
        if balance.last_deposit_ts == 0 {
            balance.owner = ctx.accounts.owner.key();
            balance.mint = ctx.accounts.token.mint;
            balance.amount = 0;
        }
        balance.amount = balance.amount.checked_add(amount).ok_or(Error::BalanceOverflow)?;
        balance.last_deposit_ts = ctx.accounts.clock.unix_timestamp;

        let accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.token.to_account_info(),
            to: ctx.accounts.vault_token.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), accounts);
        anchor_spl::token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let balance = &mut ctx.accounts.user_balance;
        if ctx.accounts.clock.unix_timestamp - balance.last_deposit_ts <= UNBONDING_SECONDS {
            return Err(Error::UnbondingPeriodNotPassed.into());
        }

        if balance.amount < amount {
            return Err(Error::NotEnoughBalance.into());
        }

        balance.amount = balance.amount.checked_sub(amount).ok_or(Error::BalanceOverflow)?;

        let accounts = anchor_spl::token::Transfer {
            from: ctx.accounts.vault_token.to_account_info(),
            to: ctx.accounts.token.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        };

        let vault_bump = Pubkey::find_program_address(&[VAULT_SEED.as_ref()], &ID).1;
        let vault_seeds: &[&[&[u8]]] = &[&[VAULT_SEED.as_ref(), &[vault_bump]]];

        let cpi_ctx = CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), accounts, vault_seeds);
        anchor_spl::token::transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init_if_needed,
        seeds = [BALANCE_SEED.as_ref(), owner.key().as_ref(), mint.key().as_ref()],
        space = UserBalance::LEN,
        payer = owner,
        bump,
    )]
    pub user_balance: Account<'info, UserBalance>,

    #[account(mut, token::mint = mint, token::authority = owner)]
    pub token: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        token::authority = vault,
        token::mint = mint,
        seeds = [VAULT_SEED.as_ref(), mint.key().as_ref()],
        bump,
        payer = owner
    )]
    pub vault_token: Account<'info, TokenAccount>,

    #[account(seeds = [VAULT_SEED.as_ref()], bump)]
    /// Vault is a PDA that owns all the vaults for specific tokens.
    pub vault: SystemAccount<'info>,

    pub clock: Sysvar<'info, Clock>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [BALANCE_SEED.as_ref(), owner.key().as_ref(), token.mint.as_ref()],
        bump,
    )]
    pub user_balance: Account<'info, UserBalance>,

    #[account(mut, token::mint = user_balance.mint, token::authority = owner)]
    pub token: Account<'info, TokenAccount>,

    #[account(mut, token::mint = user_balance.mint,seeds = [VAULT_SEED.as_ref(), user_balance.mint.as_ref()], bump )]
    /// Vault owned by the program where the token will be held.
    pub vault_token: Account<'info, TokenAccount>,

    #[account(seeds = [VAULT_SEED.as_ref()], bump)]
    /// Vault is a PDA that owns all the vaults for specific tokens.
    pub vault: SystemAccount<'info>,

    pub clock: Sysvar<'info, Clock>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[account]
/// Holds the user balance for a specific mint.
pub struct UserBalance {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub last_deposit_ts: i64,
}

impl UserBalance {
    const LEN: usize = 8 + 32 + 32 + 8 + 8;
}

#[error_code]
pub enum Error {
    #[msg("Balance overflow")]
    BalanceOverflow,
    #[msg("NotEnoughBalance")]
    NotEnoughBalance,
    #[msg("UnbondingPeriodNotPassed")]
    UnbondingPeriodNotPassed,
}
