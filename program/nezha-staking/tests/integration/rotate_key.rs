use crate::{accounts::Accounts, actions, setup::setup_test_runtime_without_init};
use anyhow::Result;
use nezha_staking_lib::{
    fixed_point::test_utils::fp,
    instruction,
    state::{InsuranceCfg, YieldSplitCfg},
};
use solana_program_test::tokio;
use solana_sdk::signer::Signer;

#[tokio::test]
async fn works() -> Result<()> {
    let mut accounts = Accounts::new();
    let mut processor = setup_test_runtime_without_init(&accounts).await?;

    // first init
    actions::init(&accounts, processor.as_mut()).await?;

    processor
        .send_ixns(
            &[instruction::rotate_key(
                &accounts.program_id,
                &accounts.super_admin.pubkey(),
                instruction::RotateKeyType::Admin,
                &accounts.random1.pubkey(),
            )],
            &[&accounts.super_admin],
        )
        .await?;

    let yield_split_cfg = YieldSplitCfg {
        jackpot: fp("100_000.0"),
        insurance: InsuranceCfg {
            premium: fp("3.0"),
            probability: fp("0.0000000001"),
        },
        treasury_ratio: fp("0.5"),
        tier2_prize_share: 2,
        tier3_prize_share: 1,
    };
    let res = actions::create_epoch(&accounts, yield_split_cfg.clone(), processor.as_mut()).await;
    assert!(res.is_err());
    std::mem::swap(&mut accounts.admin, &mut accounts.random1);

    actions::create_epoch(&accounts, yield_split_cfg, processor.as_mut()).await?;

    Ok(())
}
