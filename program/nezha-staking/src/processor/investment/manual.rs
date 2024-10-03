//! Manual investor.

#[cfg(not(feature = "dev"))]
pub use disabled::{process_yield_deposit_by_investor, process_yield_withdraw_by_investor};
#[cfg(feature = "dev")]
pub use enabled::{process_yield_deposit_by_investor, process_yield_withdraw_by_investor};

// Using nested modules here to avoid unused imports warning by moving the imports to the section
// that uses them.

#[cfg(feature = "dev")]
mod enabled {
    use borsh::BorshDeserialize;
    use nezha_utils::load_accounts;
    use solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_pack::Pack, pubkey::Pubkey,
    };

    use crate::{
        accounts as ac,
        accounts::VerifyPDA,
        fixed_point::FPUSDC,
        state::{LatestEpoch, TicketsInfo},
        utils::*,
    };

    pub fn process_yield_withdraw_by_investor<'a>(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'a>],
        tickets_info: TicketsInfo,
    ) -> ProgramResult {
        msg!("Ixn: Investor withdraw");

        let account_info_iter = &mut accounts.iter();

        load_accounts!(
            account_info_iter,
            //
            admin_info,
            investor_usdc_info,
            epoch_info,
            latest_epoch_info,
            vault_authority_info,
            deposit_vault_info,
            token_program_info
        );

        check_token_program(token_program_info)?;

        ac::latest_epoch(program_id).verify(latest_epoch_info)?;
        let latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.data.borrow())?;

        check_admin(admin_info, &latest_epoch)?;

        // send all usdc token from vault to investor for yield
        let vault_usdc_amount =
            FPUSDC::from_usdc(spl_token::state::Account::unpack(&deposit_vault_info.data.borrow())?.amount);

        let destination_account_info = investor_usdc_info;

        super::super::invest(
            program_id,
            deposit_vault_info,
            vault_authority_info,
            destination_account_info,
            latest_epoch_info,
            epoch_info,
            token_program_info,
            tickets_info,
            vault_usdc_amount,
        )?;

        Ok(())
    }

    pub fn process_yield_deposit_by_investor<'a>(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'a>],
        return_amount: u64,
    ) -> ProgramResult {
        msg!("Ixn: Return by investor");

        let account_info_iter = &mut accounts.iter();

        load_accounts!(
            account_info_iter,
            //
            investor_info,
            investor_usdc_info,
            epoch_info,
            latest_epoch_info,
            //
            deposit_vault_info,
            treasury_vault_info,
            insurance_vault_info,
            tier2_prize_vault_info,
            tier3_prize_vault_info,
            //
            token_program_info
        );

        check_token_program(token_program_info)?;

        ac::latest_epoch(program_id).verify(latest_epoch_info)?;
        let latest_epoch = LatestEpoch::try_from_slice(&latest_epoch_info.data.borrow())?;

        check_investor(investor_info, &latest_epoch)?;

        let source_account_info = investor_usdc_info;
        let source_authority_info = investor_info;

        super::super::withdraw(
            program_id,
            latest_epoch_info,
            epoch_info,
            source_account_info,
            source_authority_info,
            None,
            deposit_vault_info,
            treasury_vault_info,
            insurance_vault_info,
            tier2_prize_vault_info,
            tier3_prize_vault_info,
            token_program_info,
            return_amount,
        )?;

        Ok(())
    }
}

#[cfg(not(feature = "dev"))]
mod disabled {
    use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

    use crate::{error::*, state::TicketsInfo};

    pub fn process_yield_withdraw_by_investor<'a>(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo<'a>],
        _tickets_info: TicketsInfo,
    ) -> ProgramResult {
        msg!("Ixn: Investor withdraw");
        Err(StakingError::RemovedInstruction.into())
    }

    pub fn process_yield_deposit_by_investor<'a>(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo<'a>],
        _return_amount: u64,
    ) -> ProgramResult {
        msg!("Ixn: Return by investor");
        Err(StakingError::RemovedInstruction.into())
    }
}
