use anyhow::Result;
use async_trait::async_trait;
use rand::{prelude::StdRng, SeedableRng};
use serde_json::json;
use service::{
    stake::{deposit::HappyApprover, stake::DefaultStakeService, Stake},
    tickets::{ConstantTicketPriceCalculator, DefaultTicketService, Ticket, TicketRepository},
};
use solana_sdk::pubkey::Pubkey;
use std::sync::RwLock;

use super::{
    epochs::InMemoryEpochService,
    stake::{InMemoryDepositRepository, InMemoryStakeRepository},
};

#[tokio::test]
async fn ticket_representation() {
    let mut rng = StdRng::from_seed([0u8; 32]);
    let pk = Pubkey::new_rand();
    let mut stake_repo = Box::new(InMemoryStakeRepository::default());
    stake_repo.add(Stake {
        is_initialized: true,
        owner: pk.clone(),
        amount: 1000,
        effective_epoch_index: 1,
    });

    let mut ticket_repo = InMemoryTicketRepository::default();
    ticket_repo.add(Ticket::generate(&mut rng, pk.to_string(), 1, 2).expect("unable to generate ticket"));

    let rng = StdRng::from_entropy();
    let ticket_service = DefaultTicketService::new(
        rng,
        Box::new(ticket_repo),
        Box::new(ConstantTicketPriceCalculator::new(100)),
        Box::new(InMemoryStakeRepository::default()),
    );

    let ctx = super::setup();

    let schema = api::load_schema(
        ctx,
        Box::new(InMemoryEpochService::default()),
        Box::new(ticket_service),
        Box::new(DefaultStakeService::new(
            stake_repo.clone(),
            Box::new(InMemoryDepositRepository::default()),
            Box::new(HappyApprover::default()),
        )),
    );

    let rsp = schema
        .execute(format!(
            r#"
        query {{
            ticket(wallet:"{}", epochIndex: 1) {{
              wallet
              epochIndex
              sequences
              arweaveUrl
            }}
          }}
    "#,
            pk.to_string()
        ))
        .await
        .data
        .into_json()
        .expect("unable to convert to json");

    let expected = json!(
        {
            "ticket": {
              "epochIndex": 1,
              "arweaveUrl": None::<String>,
              "sequences": [
                [20, 26, 41, 12, 27, 2],
                [22, 18, 46, 41, 5, 7]
              ],
              "wallet": pk.to_string(),
            }
          }
    );

    assert_eq!(rsp, expected);
}

#[derive(Default)]
pub struct InMemoryTicketRepository {
    mem: RwLock<Vec<Ticket>>,
}

impl InMemoryTicketRepository {
    pub fn add(&mut self, entry: Ticket) {
        self.mem.write().unwrap().push(entry)
    }
}

#[async_trait]
impl TicketRepository for InMemoryTicketRepository {
    async fn by_wallet_and_epoch_index(&self, wallet: &Pubkey, index: u64) -> Result<Option<Ticket>> {
        Ok(self
            .mem
            .read()
            .unwrap()
            .iter()
            .find(|&v| v.wallet == wallet.to_string() && v.epoch_index == index)
            .map(|v| v.clone()))
    }

    async fn by_wallets_and_epoch_index(&self, wallets: Vec<&str>, index: u64) -> Result<Vec<Ticket>> {
        Ok(self
            .mem
            .read()
            .unwrap()
            .iter()
            .filter(|&v| wallets.contains(&v.wallet.as_str()) && v.epoch_index == index)
            .map(|v| v.clone())
            .collect())
    }

    async fn all(&self) -> Result<Vec<Ticket>> {
        Ok(self.mem.read().unwrap().clone())
    }

    async fn distinct_sequences_by_epoch_index(&self, index: u64) -> Result<Vec<Sequence>> {
        todo!()
    }

    async fn count_sequences_by_epoch_index_and_prefix(&self, index: u64, prefix: &[u8]) -> Result<u64> {
        todo!()
    }

    async fn create(&self, ticket: Ticket) -> Result<Ticket> {
        self.mem.write().unwrap().push(ticket.clone());
        Ok(ticket)
    }

    async fn update_arweave_url(&self, wallet: &Pubkey, index: u64, arweave_url: String) -> Result<Option<()>> {
        Ok(self
            .mem
            .write()
            .unwrap()
            .iter_mut()
            .find(|v| v.wallet == wallet.to_string() && v.epoch_index == index)
            .map(|v| v.arweave_url = Some(arweave_url)))
    }
}
