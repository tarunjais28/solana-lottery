use anyhow::{Context, Result};
use nezha_staking_lib::{
    fixed_point::test_utils::fp,
    instruction::{CreateEpochWinnersMetaArgs, TierWinnersMetaInput, WinnerInput},
    state::MAX_NUM_WINNERS_PER_PAGE,
};
use solana_program::pubkey::Pubkey;
use solana_program_test::tokio;

use crate::{
    accounts::Accounts,
    actions::{
        approve_stake_update, complete_stake_update, create_epoch, create_epoch_winners_meta, get_latest_epoch,
        publish_epoch_winners_page, random_yield_split_cfg, request_stake_update, set_winning_combination,
        yield_deposit_by_investor, yield_withdraw_by_investor, StakeUpdateOp,
    },
    setup::setup_test_runtime,
};

use nezha_testing::solana_test_runtime::SolanaTestRuntime;

async fn setup() -> Result<(Accounts, Box<dyn SolanaTestRuntime>)> {
    let accounts = Accounts::new();
    let processor = setup_test_runtime(&accounts).await?;
    Ok((accounts, processor))
}

async fn progress_epoch(accounts: &Accounts, processor: &mut dyn SolanaTestRuntime) -> Result<()> {
    create_epoch(accounts, random_yield_split_cfg(), processor).await?;

    let deposit_amount = 100.0;
    // It's okay if this is less than the number of winners as it's not enforced.
    let num_tickets_issued = 1;

    request_stake_update(StakeUpdateOp::Deposit, fp(deposit_amount), accounts, processor).await?;
    approve_stake_update(accounts, processor, StakeUpdateOp::Deposit, fp(deposit_amount)).await?;
    complete_stake_update(accounts, processor).await?;

    yield_withdraw_by_investor(num_tickets_issued, accounts, processor).await?;
    yield_deposit_by_investor(fp(200.0), accounts, processor).await?;

    let latest_epoch = get_latest_epoch(accounts, processor).await?;
    set_winning_combination(latest_epoch.index, [0u8; 6], accounts, processor).await?;

    Ok(())
}

fn generate_winners(
    tier1: u32,
    tier1_tickets: u32,
    tier2: u32,
    tier2_tickets: u32,
    tier3: u32,
    tier3_tickets: u32,
) -> Vec<WinnerInput> {
    let mut winners = Vec::new();

    for i in 0..tier1 {
        winners.push(WinnerInput {
            index: i,
            address: Pubkey::new_unique(),
            tier: 1,
            num_winning_tickets: tier1_tickets,
        });
    }

    for i in tier1..tier1 + tier2 {
        winners.push(WinnerInput {
            index: i,
            address: Pubkey::new_unique(),
            tier: 2,
            num_winning_tickets: tier2_tickets,
        });
    }

    for i in tier1 + tier2..tier1 + tier2 + tier3 {
        winners.push(WinnerInput {
            index: i,
            address: Pubkey::new_unique(),
            tier: 3,
            num_winning_tickets: tier3_tickets,
        });
    }

    winners
}

#[tokio::test]
async fn publish_winners_happy_path() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    progress_epoch(&accounts, processor.as_mut()).await?;

    let winners = generate_winners(25, 1, 25, 2, 50, 3);

    let meta_args = CreateEpochWinnersMetaArgs {
        tier1_meta: TierWinnersMetaInput {
            total_num_winners: 25,
            total_num_winning_tickets: 25 * 1,
        },
        tier2_meta: TierWinnersMetaInput {
            total_num_winners: 25,
            total_num_winning_tickets: 25 * 2,
        },
        tier3_meta: TierWinnersMetaInput {
            total_num_winners: 50,
            total_num_winning_tickets: 50 * 3,
        },
    };
    create_epoch_winners_meta(&meta_args, &accounts, processor.as_mut()).await?;

    for (i, chunk) in winners.chunks(MAX_NUM_WINNERS_PER_PAGE).enumerate() {
        publish_epoch_winners_page(i as _, &chunk, &accounts, processor.as_mut()).await?;
    }

    Ok(())
}

#[tokio::test]
async fn publish_winners_larger_chunks() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    progress_epoch(&accounts, processor.as_mut()).await?;

    let winners = generate_winners(25, 1, 25, 2, 55, 3);

    let meta_args = CreateEpochWinnersMetaArgs {
        tier1_meta: TierWinnersMetaInput {
            total_num_winners: 25,
            total_num_winning_tickets: 25 * 1,
        },
        tier2_meta: TierWinnersMetaInput {
            total_num_winners: 25,
            total_num_winning_tickets: 25 * 2,
        },
        tier3_meta: TierWinnersMetaInput {
            total_num_winners: 55,
            total_num_winning_tickets: 55 * 3,
        },
    };

    create_epoch_winners_meta(&meta_args, &accounts, processor.as_mut()).await?;

    for (i, chunk) in winners.chunks(MAX_NUM_WINNERS_PER_PAGE * 2).enumerate() {
        let res = publish_epoch_winners_page(i as _, &chunk, &accounts, processor.as_mut()).await;
        assert!(res.is_err());
    }

    Ok(())
}

#[tokio::test]
async fn publish_winners_smaller_chunks() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    progress_epoch(&accounts, processor.as_mut()).await?;

    let winners = generate_winners(25, 1, 25, 2, 50, 3);

    let meta_args = CreateEpochWinnersMetaArgs {
        tier1_meta: TierWinnersMetaInput {
            total_num_winners: 25,
            total_num_winning_tickets: 25 * 1,
        },
        tier2_meta: TierWinnersMetaInput {
            total_num_winners: 25,
            total_num_winning_tickets: 25 * 2,
        },
        tier3_meta: TierWinnersMetaInput {
            total_num_winners: 50,
            total_num_winning_tickets: 50 * 3,
        },
    };

    create_epoch_winners_meta(&meta_args, &accounts, processor.as_mut()).await?;

    for (i, chunk) in winners.chunks(MAX_NUM_WINNERS_PER_PAGE / 2).enumerate() {
        let res = publish_epoch_winners_page(i as _, &chunk, &accounts, processor.as_mut()).await;
        assert!(res.is_err());
    }

    Ok(())
}

#[tokio::test]
async fn publish_wrong_page() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    progress_epoch(&accounts, processor.as_mut()).await?;

    let winners = generate_winners(25, 1, 25, 2, 50, 3);

    let meta_args = CreateEpochWinnersMetaArgs {
        tier1_meta: TierWinnersMetaInput {
            total_num_winners: 25,
            total_num_winning_tickets: 25 * 1,
        },
        tier2_meta: TierWinnersMetaInput {
            total_num_winners: 25,
            total_num_winning_tickets: 25 * 2,
        },
        tier3_meta: TierWinnersMetaInput {
            total_num_winners: 50,
            total_num_winning_tickets: 50 * 3,
        },
    };

    create_epoch_winners_meta(&meta_args, &accounts, processor.as_mut()).await?;

    publish_epoch_winners_page(0, &winners[0..MAX_NUM_WINNERS_PER_PAGE], &accounts, processor.as_mut()).await?;

    let res = publish_epoch_winners_page(1, &winners[0..MAX_NUM_WINNERS_PER_PAGE], &accounts, processor.as_mut()).await;
    assert!(res.is_err());

    Ok(())
}

#[tokio::test]
async fn republish_the_same_page() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    progress_epoch(&accounts, processor.as_mut()).await?;

    let winners = generate_winners(25, 1, 25, 2, 50, 3);

    let meta_args = CreateEpochWinnersMetaArgs {
        tier1_meta: TierWinnersMetaInput {
            total_num_winners: 25,
            total_num_winning_tickets: 25 * 1,
        },
        tier2_meta: TierWinnersMetaInput {
            total_num_winners: 25,
            total_num_winning_tickets: 25 * 2,
        },
        tier3_meta: TierWinnersMetaInput {
            total_num_winners: 50,
            total_num_winning_tickets: 50 * 3,
        },
    };

    create_epoch_winners_meta(&meta_args, &accounts, processor.as_mut()).await?;

    publish_epoch_winners_page(0, &winners[0..MAX_NUM_WINNERS_PER_PAGE], &accounts, processor.as_mut())
        .await
        .context("Page 0")?;
    publish_epoch_winners_page(
        1,
        &winners[MAX_NUM_WINNERS_PER_PAGE..MAX_NUM_WINNERS_PER_PAGE * 2],
        &accounts,
        processor.as_mut(),
    )
    .await
    .context("Page 1")?;

    let res = publish_epoch_winners_page(0, &winners[0..MAX_NUM_WINNERS_PER_PAGE], &accounts, processor.as_mut())
        .await
        .context("Republish Page 0");
    assert!(res.is_err());

    create_epoch_winners_meta(&meta_args, &accounts, processor.as_mut()).await?;
    publish_epoch_winners_page(0, &winners[0..MAX_NUM_WINNERS_PER_PAGE], &accounts, processor.as_mut())
        .await
        .context("Republish Page 0")?;

    Ok(())
}

#[tokio::test]
async fn cant_change_winners_once_published() -> Result<()> {
    let (accounts, mut processor) = setup().await?;

    progress_epoch(&accounts, processor.as_mut()).await?;

    let winners = generate_winners(25, 1, 25, 2, 50, 3);

    let meta_args = CreateEpochWinnersMetaArgs {
        tier1_meta: TierWinnersMetaInput {
            total_num_winners: 25,
            total_num_winning_tickets: 25 * 1,
        },
        tier2_meta: TierWinnersMetaInput {
            total_num_winners: 25,
            total_num_winning_tickets: 25 * 2,
        },
        tier3_meta: TierWinnersMetaInput {
            total_num_winners: 50,
            total_num_winning_tickets: 50 * 3,
        },
    };
    create_epoch_winners_meta(&meta_args, &accounts, processor.as_mut()).await?;

    for (i, chunk) in winners.chunks(MAX_NUM_WINNERS_PER_PAGE).enumerate() {
        publish_epoch_winners_page(i as _, &chunk, &accounts, processor.as_mut()).await?;
    }

    let res = create_epoch_winners_meta(&meta_args, &accounts, processor.as_mut()).await;
    assert!(res.is_err());

    Ok(())
}
