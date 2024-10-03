use super::returns;
use crate::fixed_point::FixedPoint;
use crate::state::*;

use crate::fixed_point::test_utils::fp;

#[test]
fn investor_return() {
    #[derive(Clone)]
    struct TestInput {
        name: &'static str,
        yield_split_cfg: YieldSplitCfg,
        tickets_issued: u64,
        cumulative_return_rate: f64,
        total_invested: f64,
        investor_return: f64,
        carryover_insurance: f64,
    }

    #[derive(Default)]
    struct TestOutput {
        total_returned: Option<f64>,
        cumulative_return_rate: Option<f64>,
        draw_enabled: Option<bool>,
        yield_split_insurance: Option<f64>,
        yield_split_treasury: Option<f64>,
        yield_split_tier2_prize: Option<f64>,
        yield_split_tier3_prize: Option<f64>,
        carryover_insurance: Option<f64>,
    }

    let base_test_input = TestInput {
        name: "",
        yield_split_cfg: YieldSplitCfg {
            jackpot: fp(20.0),
            insurance: InsuranceCfg {
                premium: fp(1.0),
                probability: fp(0.5),
            },
            treasury_ratio: fp(0.5),
            tier2_prize_share: 3,
            tier3_prize_share: 1,
        },
        tickets_issued: 5,
        cumulative_return_rate: 1.0f64,
        total_invested: 0.0f64,
        investor_return: 0.0f64,
        carryover_insurance: 0.0,
    };

    let tests = [
        (
            TestInput {
                name: "happy path",
                total_invested: 100.0f64,
                investor_return: 200.0f64,
                ..base_test_input.clone()
            },
            TestOutput {
                total_returned: Some(200.0),
                cumulative_return_rate: Some(1.0),
                draw_enabled: Some(true),
                yield_split_insurance: Some(50.0),
                yield_split_treasury: Some(25.0),
                yield_split_tier2_prize: Some(18.75),
                yield_split_tier3_prize: Some(6.25),
                carryover_insurance: Some(0.0),
                ..Default::default()
            },
        ),
        (
            TestInput {
                name: "yield covers insurance only",
                total_invested: 100.0f64,
                investor_return: 150.0f64,
                ..base_test_input.clone()
            },
            TestOutput {
                cumulative_return_rate: Some(1.0),
                draw_enabled: Some(true),
                yield_split_insurance: Some(50.0),
                yield_split_treasury: Some(0.0),
                yield_split_tier2_prize: Some(0.0),
                yield_split_tier3_prize: Some(0.0),
                ..Default::default()
            },
        ),
        (
            TestInput {
                name: "yield doesn't cover insurance",
                total_invested: 100.0f64,
                investor_return: 140.0f64,
                ..base_test_input.clone()
            },
            TestOutput {
                cumulative_return_rate: Some(1.0),
                draw_enabled: Some(false),
                yield_split_insurance: Some(40.0),
                yield_split_treasury: Some(0.0),
                yield_split_tier2_prize: Some(0.0),
                yield_split_tier3_prize: Some(0.0),
                carryover_insurance: Some(40.0),
                ..Default::default()
            },
        ),
        (
            TestInput {
                name: "yield + carryover covers insurance",
                total_invested: 100.0f64,
                investor_return: 130.0f64,
                carryover_insurance: 40.0,
                ..base_test_input.clone()
            },
            TestOutput {
                cumulative_return_rate: Some(1.0),
                draw_enabled: Some(true),
                yield_split_insurance: Some(10.0),
                yield_split_treasury: Some(10.0),
                yield_split_tier2_prize: Some(7.5),
                yield_split_tier3_prize: Some(2.5),
                carryover_insurance: Some(0.0),
                ..Default::default()
            },
        ),
        (
            TestInput {
                name: "carryover alone covers insurance",
                total_invested: 100.0f64,
                investor_return: 120.0f64,
                carryover_insurance: 100.0,
                ..base_test_input.clone()
            },
            TestOutput {
                cumulative_return_rate: Some(1.0),
                draw_enabled: Some(true),
                yield_split_insurance: Some(0.0),
                yield_split_treasury: Some(10.0),
                yield_split_tier2_prize: Some(7.5),
                yield_split_tier3_prize: Some(2.5),
                carryover_insurance: Some(50.0),
                ..Default::default()
            },
        ),
        (
            TestInput {
                name: "loss scenario",
                total_invested: 100.0f64,
                investor_return: 90.0f64,
                ..base_test_input.clone()
            },
            TestOutput {
                cumulative_return_rate: Some(0.9),
                draw_enabled: Some(false),
                yield_split_insurance: Some(0.0),
                yield_split_treasury: Some(0.0),
                yield_split_tier2_prize: Some(0.0),
                yield_split_tier3_prize: Some(0.0),
                carryover_insurance: Some(0.0),
                ..Default::default()
            },
        ),
        (
            TestInput {
                name: "loss scenario with insurance carryover",
                total_invested: 100.0f64,
                investor_return: 90.0f64,
                carryover_insurance: 60.0f64,
                ..base_test_input.clone()
            },
            TestOutput {
                cumulative_return_rate: Some(0.9),
                draw_enabled: Some(true),
                yield_split_insurance: Some(0.0),
                yield_split_treasury: Some(0.0),
                yield_split_tier2_prize: Some(0.0),
                yield_split_tier3_prize: Some(0.0),
                carryover_insurance: Some(10.0),
                ..Default::default()
            },
        ),
    ];

    // Test runner
    for (input, output) in tests {
        println!("Test: {}", input.name);

        let returns_info = returns::distribute_returns(
            fp(input.investor_return),
            fp(input.total_invested),
            CumulativeReturnRate::new(fp(input.cumulative_return_rate)).unwrap(),
            PendingFunds {
                insurance: fp(input.carryover_insurance),
                tier2_prize: FixedPoint::zero(),
                tier3_prize: FixedPoint::zero(),
            },
            returns::YieldSplitCfgInternal {
                insurance_amount: input
                    .yield_split_cfg
                    .insurance
                    .calculate_amount(input.tickets_issued, input.yield_split_cfg.jackpot)
                    .unwrap(),
                treasury_ratio: input.yield_split_cfg.treasury_ratio,
                tier2_prize_share: input.yield_split_cfg.tier2_prize_share,
                tier3_prize_share: input.yield_split_cfg.tier3_prize_share,
            },
        )
        .unwrap();

        if let Some(total_returned) = output.total_returned {
            assert_eq!(returns_info.returns.total, fp(total_returned));
        }

        if let Some(cumulative_return_rate) = output.cumulative_return_rate {
            assert_eq!(
                returns_info.cumulative_return_rate,
                CumulativeReturnRate::new(fp(cumulative_return_rate)).unwrap()
            );
        }

        if let Some(draw_enabled) = output.draw_enabled {
            assert_eq!(returns_info.draw_enabled, draw_enabled);
        }

        if let Some(yield_split_insurance) = output.yield_split_insurance {
            assert_eq!(returns_info.returns.insurance, fp(yield_split_insurance));
        }

        if let Some(yield_split_treasury) = output.yield_split_treasury {
            assert_eq!(returns_info.returns.treasury, fp(yield_split_treasury));
        }

        if let Some(yield_split_tier2_prize) = output.yield_split_tier2_prize {
            assert_eq!(returns_info.returns.tier2_prize, fp(yield_split_tier2_prize));
        }

        if let Some(yield_split_tier3_prize) = output.yield_split_tier3_prize {
            assert_eq!(returns_info.returns.tier3_prize, fp(yield_split_tier3_prize));
        }

        if let Some(carryover_insurance) = output.carryover_insurance {
            assert_eq!(returns_info.pending_funds.insurance, fp(carryover_insurance));
        }
    }
}
