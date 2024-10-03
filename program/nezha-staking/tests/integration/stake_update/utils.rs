use nezha_testing::solana_test_runtime::SolanaTestRuntime;

use super::*;

pub async fn setup() -> Result<(Accounts, Box<dyn SolanaTestRuntime>)> {
    let accounts = Accounts::new();
    let mut processor = setup_test_runtime(&accounts).await?;

    progress_epoch_till(EpochStatus::Running, &accounts, processor.as_mut()).await?;
    Ok((accounts, processor))
}

#[derive(Default)]
pub struct AssertBalances {
    pub stake: Option<FPUSDC>,
    pub pending_deposit_vault: Option<FPUSDC>,
    pub deposit_vault: Option<FPUSDC>,
}

impl AssertBalances {
    pub fn deposit_complete(amount: FPUSDC) -> AssertBalances {
        AssertBalances {
            stake: Some(amount),
            pending_deposit_vault: Some(FPUSDC::zero()),
            deposit_vault: Some(amount),
        }
    }

    pub fn deposit_pending(amount: FPUSDC) -> AssertBalances {
        AssertBalances {
            stake: Some(FPUSDC::zero()),
            pending_deposit_vault: Some(amount),
            deposit_vault: Some(FPUSDC::zero()),
        }
    }

    pub fn deposit_cancelled() -> AssertBalances {
        AssertBalances {
            stake: Some(FPUSDC::zero()),
            pending_deposit_vault: Some(FPUSDC::zero()),
            deposit_vault: Some(FPUSDC::zero()),
        }
    }
}

pub async fn assert_balances(
    balances: AssertBalances,
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    if let Some(b) = balances.stake {
        let stake = get_owner_stake_balance(&accounts, processor).await?;
        assert_eq!(stake, b);
    }

    if let Some(b) = balances.pending_deposit_vault {
        let pending_vault_balance =
            get_usdc_balance_by_account(&ac::pending_deposit_vault(&accounts.program_id), processor).await?;
        assert_eq!(pending_vault_balance, b);
    }

    if let Some(b) = balances.deposit_vault {
        let deposit_vault_balance =
            get_usdc_balance_by_account(&ac::deposit_vault(&accounts.program_id), processor).await?;
        assert_eq!(deposit_vault_balance, b);
    }
    Ok(())
}

pub async fn run_stake_update(
    op: StakeUpdateOp,
    amount: FPUSDC,
    accounts: &Accounts,
    processor: &mut dyn SolanaTestRuntime,
) -> Result<()> {
    request_stake_update(op, amount, accounts, processor).await?;
    if op == StakeUpdateOp::Deposit {
        approve_stake_update(accounts, processor, op, amount).await?;
    }
    complete_stake_update(accounts, processor).await?;
    Ok(())
}
