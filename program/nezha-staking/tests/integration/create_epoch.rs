use std::time::{SystemTime, UNIX_EPOCH};

use crate::{accounts::Accounts, setup::setup_test_runtime};
use anyhow::Result;
use nezha_staking_lib::{
    fixed_point::{test_utils::fp, FPUSDC},
    instruction,
    state::{InsuranceCfg, YieldSplitCfg},
};
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64
}

fn one_day_from_now() -> i64 {
    now() + 60 * 60 * 24
}

#[tokio::test]
async fn works() -> Result<()> {
    let accounts = Accounts::new();
    let mut runtime = setup_test_runtime(&accounts).await?;

    let epoch_index = 1;
    let expected_end_at = one_day_from_now();
    let yield_split_cfg = YieldSplitCfg {
        jackpot: fp("100_000.0"),
        insurance: InsuranceCfg {
            premium: fp("3.0"),
            probability: fp("0.000001"),
        },
        treasury_ratio: fp("0.5"),
        tier2_prize_share: 2,
        tier3_prize_share: 1,
    };

    runtime
        .send_ixns(
            &[instruction::create_epoch(
                &accounts.program_id,
                &accounts.admin.pubkey(),
                epoch_index,
                expected_end_at,
                yield_split_cfg,
            )],
            &[&accounts.admin],
        )
        .await?;
    Ok(())
}

#[tokio::test]
async fn with_invalid_values() -> Result<()> {
    let accounts = Accounts::new();
    let mut runtime = setup_test_runtime(&accounts).await?;

    let one_day_from_now = || now() + 60 * 60 * 24;

    // 30 digits + 6 digits decimal
    let thirty_digits_fpusdc: FPUSDC = fp("1_000_000_000_000_000_000_000_000_000_000.0");

    let correct_yield_split_cfg = YieldSplitCfg {
        jackpot: fp("100_000.0"),
        insurance: InsuranceCfg {
            premium: fp("3.0"),
            probability: fp("0.000001"),
        },
        treasury_ratio: fp("0.5"),
        tier2_prize_share: 2,
        tier3_prize_share: 1,
    };

    for (name, expected_end_at, yield_split_cfg) in [
        (
            "end in past",
            {
                now() - 10 // 10secs ago to account for any drift
            },
            YieldSplitCfg {
                jackpot: fp("100_000.0"),
                insurance: InsuranceCfg {
                    premium: fp("3.0"),
                    probability: fp("0.000001"),
                },
                treasury_ratio: fp("0.5"),
                tier2_prize_share: 2,
                tier3_prize_share: 1,
            },
        ),
        (
            "jackpot is zero",
            one_day_from_now(),
            YieldSplitCfg {
                jackpot: fp("0.0"),
                ..correct_yield_split_cfg.clone()
            },
        ),
        (
            "jackpot is too high",
            one_day_from_now(),
            YieldSplitCfg {
                jackpot: thirty_digits_fpusdc,
                ..correct_yield_split_cfg.clone()
            },
        ),
        (
            "premium is zero",
            one_day_from_now(),
            YieldSplitCfg {
                insurance: InsuranceCfg {
                    premium: fp("0.0"),
                    ..correct_yield_split_cfg.insurance.clone()
                },
                ..correct_yield_split_cfg.clone()
            },
        ),
        (
            "premium is too high",
            one_day_from_now(),
            YieldSplitCfg {
                insurance: InsuranceCfg {
                    premium: thirty_digits_fpusdc,
                    ..correct_yield_split_cfg.insurance.clone()
                },
                ..correct_yield_split_cfg.clone()
            },
        ),
        (
            "probability is zero",
            one_day_from_now(),
            YieldSplitCfg {
                insurance: InsuranceCfg {
                    probability: fp("0.0"),
                    ..correct_yield_split_cfg.insurance.clone()
                },
                ..correct_yield_split_cfg.clone()
            },
        ),
        (
            "probability is too high",
            one_day_from_now(),
            YieldSplitCfg {
                insurance: InsuranceCfg {
                    // this test value is somewhat arbitrary.
                    // the value should be much lower than this.
                    // but for the sake of having some tests, lets say one in 1000 people is too
                    // high.
                    probability: fp("0.001"),
                    ..correct_yield_split_cfg.insurance.clone()
                },
                ..correct_yield_split_cfg.clone()
            },
        ),
        (
            "treasury ratio is too high",
            one_day_from_now(),
            YieldSplitCfg {
                treasury_ratio: fp("1.1"),
                ..correct_yield_split_cfg.clone()
            },
        ),
        (
            "tier2 share is zero",
            one_day_from_now(),
            YieldSplitCfg {
                tier2_prize_share: 0,
                ..correct_yield_split_cfg.clone()
            },
        ),
        (
            "tier3 share is zero",
            one_day_from_now(),
            YieldSplitCfg {
                tier3_prize_share: 0,
                ..correct_yield_split_cfg.clone()
            },
        ),
    ] {
        let epoch_index = 1;
        println!(">> test: {}", name);
        let res = runtime
            .send_ixns(
                &[instruction::create_epoch(
                    &accounts.program_id,
                    &accounts.admin.pubkey(),
                    epoch_index,
                    expected_end_at,
                    yield_split_cfg,
                )],
                &[&accounts.admin],
            )
            .await;
        assert!(res.is_err())
    }

    Ok(())
}
