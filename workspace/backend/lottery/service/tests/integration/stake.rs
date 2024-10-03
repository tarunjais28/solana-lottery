use crate::common::{self, util::progress_latest_epoch_to_status};
use anyhow::Result;
use async_trait::async_trait;
use rand::Rng;
use service::{
    model::{
        stake_update::StakeUpdate,
        transaction::{Transaction, TransactionId},
    },
    stake::{DefaultStakeService, StakeService, StakeUpdateRepository, TransactionDecoder},
};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use spl_token::ui_amount_to_amount;
use std::sync::RwLock;

#[derive(Default)]
pub struct InMemoryStakeUpdateRepository {
    mem: RwLock<Vec<StakeUpdate>>,
}

#[async_trait]
impl StakeUpdateRepository for InMemoryStakeUpdateRepository {
    async fn by_transaction_id(&self, transaction_id: &TransactionId) -> Result<Option<StakeUpdate>> {
        let stake_updates = self.mem.read().unwrap();
        let transaction_id = Some(transaction_id.to_owned());
        Ok(stake_updates
            .iter()
            .find(|d| d.transaction_id == transaction_id)
            .cloned())
    }

    async fn by_wallet(&self, wallet: &Pubkey) -> Result<Vec<StakeUpdate>> {
        let stake_updates = self.mem.read().unwrap();
        Ok(stake_updates.iter().filter(|d| &d.owner == wallet).cloned().collect())
    }

    async fn store(&self, stake_update: &StakeUpdate) -> Result<StakeUpdate> {
        self.mem.write().unwrap().push(stake_update.clone());
        Ok(stake_update.clone())
    }
}

#[derive(Default)]
pub struct MockTransactionDecoder;

#[async_trait]
impl TransactionDecoder for MockTransactionDecoder {
    async fn by_wallet(&self, _wallet: &Pubkey, _before: Option<TransactionId>) -> Result<Vec<Transaction>> {
        todo!()
    }

    async fn deposits(&self, _before: Option<TransactionId>) -> Result<Vec<Transaction>> {
        todo!()
    }
}

#[tokio::test]
async fn all_stakes() {
    let ctx = common::setup_solana().await;

    progress_latest_epoch_to_status(&ctx, nezha_staking::state::EpochStatus::Running)
        .await
        .expect("Can't move epoch to Running");

    let solana = ctx.solana;

    let stake_svc = DefaultStakeService::new(
        Box::new(solana.clone()),
        Box::new(InMemoryStakeUpdateRepository::default()),
    );

    let user_keypair = ctx.user_keypair;
    let mut rng = rand::thread_rng();
    let amount = ui_amount_to_amount(rng.gen_range(1.0..100.0), 6);
    common::util::perform_deposit(&solana, &user_keypair, amount)
        .await
        .expect("could not deposit stake");
    let stakes = stake_svc.all().await.expect("unable to get stakes");
    assert_eq!(false, stakes.is_empty());

    for stake in stakes.into_iter() {
        let s = stake_svc
            .by_wallet(stake.owner.to_string().as_str())
            .await
            .expect("unable to get stake from chain");

        assert_eq!(s.expect("user is supposed to have at least one stake"), stake);
    }
}

#[tokio::test]
async fn all_stake_update_requests() -> Result<()> {
    let ctx = common::setup_solana().await;
    let solana = ctx.solana;

    let stake_svc = DefaultStakeService::new(
        Box::new(solana.clone()),
        Box::new(InMemoryStakeUpdateRepository::default()),
    );

    let user_keypair = ctx.user_keypair;
    common::util::attempt_deposit(&solana, &user_keypair, 10).await?;

    let stake_updates = stake_svc
        .all_stake_update_requests()
        .await
        .expect("unable to get stake_updates");
    assert_eq!(stake_updates.len(), 1);
    assert_eq!(stake_updates[0].owner, user_keypair.pubkey());

    Ok(())
}
