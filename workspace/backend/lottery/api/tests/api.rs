// mod common;

// use crate::common::{
//     epochs::InMemoryEpochService,
//     stake::{InMemoryDepositRepository, InMemoryStakeRepository, InMemoryStakeService},
//     tickets::InMemoryTicketRepository,
// };
// use anyhow::Result;
// use chrono::DateTime;
// use rand::{prelude::StdRng, SeedableRng};
// use serde_json::{json, Value};
// use service::{
//     epoch::EpochManager,
//     model::epoch::Epoch,
//     stake::{DepositRepository, Stake, StakeRepository, StakeService},
//     tickets::{ConstantTicketPriceCalculator, DefaultTicketService, Ticket, TicketRepository, TicketService},
// };
// use solana_sdk::pubkey::Pubkey;
// use std::{error::Error, str::FromStr};
// use test_case::test_case;

// /// Tests integration of *service layer* and *api layer*.
// ///
// /// # Test cases
// /// 1. Lists all epochs
// #[test_case(r#"
// query {
//   epochs {
//     index
//     start
//     end
//     prize {
//       tier
//       amount
//     }
//     canCollect
//   }
// }
// "#, json!(
// {
//   "epochs": [{
//     "index": 0,
//     "start": "2022-01-01T00:00:00+00:00",
//     "end": "2022-01-07T00:00:00+00:00",
//     "prize": [],
//     "canCollect": false
//   }]
// }
// ))]

// /// 2. Gets a single epoch
// #[test_case(r#"
// query {
//   epoch(index: 0) {
//     index
//     start
//     end
//     prize { tier amount winners }
//     canCollect
//   }
// }
// "#, json!(
// {
//   "epoch": {
//     "index": 0,
//     "start": "2022-01-01T00:00:00+00:00",
//     "end": "2022-01-07T00:00:00+00:00",
//     "prize": [],
//     "canCollect": false
//   }
// }
// ))]

// /// 3. Lists tickets of wallet and epoch
// #[test_case(r#"
// query {
//   ticket(wallet: "11111111111111111111111111111111", epochIndex: 0) {
//     epochIndex
//     arweaveUrl
//     sequences
//   }
// }
// "#, json!(
// {
//   "ticket": {
//     "epochIndex": 0,
//     "arweaveUrl": None::<String>,
//     "sequences": [],
//   }
// }
// ))]

// /// 4. Lists balances of wallet
// #[test_case(r#"
// query {
//   balances(wallet: "11111111111111111111111111111111") { amount currency }
// }
// "#, json!(
// {
//   "balances": [{
//     "amount": "0",
//     "currency": "USDC",
//   }]
// }
// ))]
// #[tokio::test]
// async fn test_integration_of_schema_with_service_layer(
//     query: &str,
//     expected_response_data: Value,
// ) -> Result<(), Box<dyn Error>> {
//     let mut epoch_service = InMemoryEpochService::default();
//     let epoch = Epoch {
//         id: Default::default(),
//         index: 0,
//         start: DateTime::from_str("2022-01-01T00:00:00Z")?,
//         end: DateTime::from_str("2022-01-07T00:00:00Z")?,
//         prize: serde_json::from_str("[]")?,
//         can_collect: false,
//     };
//     epoch_service.add(epoch);

//     let mut ticket_repository = InMemoryTicketRepository::default();
//     let ticket = Ticket {
//         wallet: Pubkey::default().to_string(),
//         epoch_index: 0,
//         arweave_url: None,
//         sequences: vec![],
//     };
//     ticket_repository.add(ticket);

//     let mut stake_repository = InMemoryStakeRepository::default();
//     let stake = Stake {
//         is_initialized: false,
//         owner: Default::default(),
//         amount: 0,
//         effective_epoch_index: 1,
//     };
//     stake_repository.add(stake);

//     let rng = StdRng::from_entropy();
//     let ticket_repository: Box<dyn TicketRepository> = Box::new(ticket_repository);
//     let ticket_price_calc = ConstantTicketPriceCalculator::new(5);
//     let ticket_service: Box<dyn TicketService> = Box::new(DefaultTicketService::new(
//         rng,
//         ticket_repository,
//         Box::new(ticket_price_calc),
//         Box::new(stake_repository.clone()),
//     ));
//     let stake_repository: Box<dyn StakeRepository> = Box::new(stake_repository);
//     let deposit_repository = InMemoryDepositRepository::default();
//     let deposit_repository: Box<dyn DepositRepository> = Box::new(deposit_repository);
//     let stake_service: Box<dyn StakeService> =
//         Box::new(InMemoryStakeService::new(stake_repository, deposit_repository));
//     let epoch_service: Box<dyn EpochManager> = Box::new(epoch_service);
//     let ctx = common::setup();

//     let schema = api::load_schema(ctx, epoch_service, ticket_service, stake_service);

//     let actual_response_data = schema.execute(query).await.data.into_json()?;

//     assert_eq!(expected_response_data, actual_response_data);
//     Ok(())
// }
