use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use nezha_staking::{
    fixed_point::test_utils::fp,
    instruction::{self, CreateEpochWinnersMetaArgs, TierWinnersMetaInput, WinnerInput},
    state::YieldSplitCfg,
};
use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_sdk::{signature::Keypair, signer::Signer};
use spl_associated_token_account::get_associated_token_address;
use std::ops::Add;

use service::{
    epoch::FPUSDC,
    model::epoch::EpochStatus,
    solana::{solana_impl::SolanaImpl, InsuranceCfg, Solana},
};

use super::{send_and_confirm_tx, SolanaContext};

pub async fn progress_latest_epoch_to_status(ctx: &SolanaContext, status: EpochStatus) -> Result<()> {
    let solana = &ctx.solana;
    loop {
        let latest = solana.get_latest_epoch().await?;
        if status == latest.status {
            break;
        }
        let epoch_index = latest.index;

        match latest.status.into() {
            EpochStatus::Running => {
                perform_deposit(solana, &ctx.user_keypair, 1_000_000).await?;
                withdraw_yield(solana, latest.index).await?;
            }
            EpochStatus::Yielding => {
                deposit_yield(solana, latest.index, None).await?;
            }
            EpochStatus::Finalising => {
                assert_eq!(latest.index, epoch_index);
                publish_winning_combination(solana, latest.index, [1, 2, 3, 4, 5, 5]).await?;

                let ltest = solana.get_latest_epoch().await.unwrap().index;
                println!("### status: {:?} vs {:?}", ltest, latest.index);

                let winners_input = vec![WinnerInput {
                    index: 0,
                    address: ctx.user_keypair.pubkey(),
                    tier: 2,
                    num_winning_tickets: 1,
                }];
                println!("### 1");
                publish_winners(solana, latest.index, true, &winners_input).await?;
                println!("### 2");
            }
            EpochStatus::Ended => {
                let expected_end_date = Utc::now().add(Duration::hours(1));
                let yield_split_cfg = YieldSplitCfg {
                    insurance: InsuranceCfg {
                        premium: fp("2.0"),
                        probability: fp("0.0001"),
                    },
                    jackpot: "1_000".parse().unwrap(),
                    treasury_ratio: fp("0.5"),
                    tier2_prize_share: 7,
                    tier3_prize_share: 3,
                };

                create_epoch(solana, epoch_index + 1, expected_end_date, yield_split_cfg).await?;
            }
        }
    }

    Ok(())
}

pub async fn perform_deposit(solana: &SolanaImpl, user_keypair: &Keypair, amount: u64) -> Result<()> {
    let admin_pubkey = solana.admin_keypair().pubkey();
    let user_pubkey = user_keypair.pubkey();
    let owner_usdc_token_pubkey = get_associated_token_address(&user_pubkey, &solana.usdc_mint());

    solana.rpc_client.request_airdrop(user_pubkey, LAMPORTS_PER_SOL).await?;

    println!("Attempt deposit");
    let ix = instruction::request_stake_update(&solana.program_id, &user_pubkey, &owner_usdc_token_pubkey, amount as _);
    send_and_confirm_tx(&*solana.rpc_client, user_keypair, ix).await?;

    println!("Approve deposit");
    let ix = instruction::approve_stake_update(&solana.program_id, &admin_pubkey, &user_pubkey, amount as _);
    send_and_confirm_tx(&*solana.rpc_client, &solana.admin_keypair, ix).await?;

    println!("Complete deposit");
    let ix = instruction::complete_stake_update(
        &solana.program_id,
        &admin_pubkey,
        &user_pubkey,
        &owner_usdc_token_pubkey,
    );
    send_and_confirm_tx(&*solana.rpc_client, &solana.admin_keypair, ix).await?;
    Ok(())
}

pub async fn attempt_deposit(solana: &SolanaImpl, user_keypair: &Keypair, amount: u64) -> Result<()> {
    let user_pubkey = user_keypair.pubkey();
    let owner_usdc_token_pubkey = get_associated_token_address(&user_pubkey, &solana.usdc_mint());

    println!("Attempt deposit");
    let ix = instruction::request_stake_update(&solana.program_id, &user_pubkey, &owner_usdc_token_pubkey, amount as _);
    send_and_confirm_tx(&*solana.rpc_client, user_keypair, ix).await?;

    Ok(())
}

pub async fn withdraw_yield(solana: &dyn Solana, epoch_index: u64) -> Result<()> {
    println!("Withdrawing yield");
    solana.enter_investment_fake(epoch_index, 1).await?;
    Ok(())
}

pub async fn deposit_yield(solana: &dyn Solana, epoch_index: u64, return_amount: Option<FPUSDC>) -> Result<()> {
    let epoch = solana.get_epoch_by_index(epoch_index).await?;

    // if return_amount is set, use that, otherwise use 200% of total_invested
    let amount = match return_amount {
        Some(amount) => amount,
        None => {
            let tvl = epoch.total_invested.expect("total_invested not set");
            let amount = tvl
                .checked_mul(200u8.into())
                .ok_or_else(|| anyhow!("overflow"))?
                .checked_div(100u8.into())
                .ok_or_else(|| anyhow!("overflow"))?;
            amount
        }
    };

    println!("Depositing yield: {}", amount);
    solana.exit_investment_fake(epoch_index, amount).await?;
    Ok(())
}

pub async fn publish_winning_combination(
    solana: &dyn Solana,
    epoch_index: u64,
    winning_combination: [u8; 6],
) -> Result<()> {
    // let epoch = solana.get_epoch_by_index(epoch_index).await?;

    // let winning = nezha_vrf_lib::accounts::nezha_vrf_request(&solana.nezha_vrf_program_id(), epoch_index);
    solana
        .set_winning_combination_fake(epoch_index, &winning_combination)
        .await?;

    // match epoch.winning_combination {
    //     Some(_) => {}
    //     None => {
    //         println!("Publishing winning combination: {winning_combination:?}");
    //         solana
    //             .publish_winning_combination(epoch_index, winning_combination)
    //             .await?;
    //     }
    // }

    Ok(())
}

pub async fn publish_winners(
    solana: &dyn Solana,
    epoch_index: u64,
    draw_enabled: bool,
    winners_input: &[WinnerInput],
) -> Result<()> {
    let mut tier1_total_num_winners = 0;
    let mut tier1_total_num_winning_tickets = 0;
    let mut tier2_total_num_winners = 0;
    let mut tier2_total_num_winning_tickets = 0;
    let mut tier3_total_num_winners = 0;
    let mut tier3_total_num_winning_tickets = 0;
    for winner_input in winners_input {
        match winner_input.tier {
            1 => {
                tier1_total_num_winners += 1;
                tier1_total_num_winning_tickets += winner_input.num_winning_tickets;
            }
            2 => {
                tier2_total_num_winners += 1;
                tier2_total_num_winning_tickets += winner_input.num_winning_tickets;
            }
            3 => {
                tier3_total_num_winners += 1;
                tier3_total_num_winning_tickets += winner_input.num_winning_tickets;
            }
            _ => return Err(anyhow!("Invalid tier: {}", winner_input.tier)),
        }
    }
    let tier1_meta = TierWinnersMetaInput {
        total_num_winners: tier1_total_num_winners,
        total_num_winning_tickets: tier1_total_num_winning_tickets,
    };
    let tier2_meta = TierWinnersMetaInput {
        total_num_winners: tier2_total_num_winners,
        total_num_winning_tickets: tier2_total_num_winning_tickets,
    };
    let tier3_meta = TierWinnersMetaInput {
        total_num_winners: tier3_total_num_winners,
        total_num_winning_tickets: tier3_total_num_winning_tickets,
    };
    let meta_args = CreateEpochWinnersMetaArgs {
        tier1_meta,
        tier2_meta,
        tier3_meta,
    };

    solana
        .publish_winners(epoch_index, draw_enabled, &meta_args, winners_input)
        .await?;
    Ok(())
}

pub async fn create_epoch(
    solana: &dyn Solana,
    epoch_index: u64,
    expected_end_at: DateTime<Utc>,
    yield_split_cfg: YieldSplitCfg,
) -> Result<()> {
    println!("Creating epoch");
    solana
        .create_epoch(epoch_index, expected_end_at, yield_split_cfg)
        .await?;
    Ok(())
}
