use crate::{epochs::services::*, tickets::services::*, users::services::*};
use async_graphql::{EmptySubscription, MergedObject, Schema};

#[derive(MergedObject, Default)]
pub struct Query(EpochsQuery, UsersQuery, TicketsQuery);

#[derive(MergedObject, Default)]
pub struct Mutation(EpochMutation, UserMutation, TicketMutation);

pub type NezhaSchema = Schema<Query, Mutation, EmptySubscription>;
