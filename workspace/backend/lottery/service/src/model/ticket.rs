use thiserror::Error;

use crate::tickets::Ticket;

pub const SEQUENCE_LENGTH: usize = 6;

#[derive(Error, Debug)]
pub enum TicketError {
    #[error("empty prefix")]
    EmptyPrefix,

    #[error("prefix length exceeded, maximum allowed = 6, supplied prefix length = {0}")]
    PrefixLengthExceeded(usize),
}

#[derive(Debug, Clone)]
pub struct TicketsWithCount {
    pub tickets: Vec<Ticket>,
    pub count: usize,
}
