mod account_not_found;
pub use account_not_found::*;

mod transaction_error;
pub use transaction_error::*;

use std::error::Error;

use nezha_staking::state::AccountType;
use solana_sdk::{signature::Signature, transaction::TransactionError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SolanaError {
    // Blanket wrap other errors
    #[error("Unexpected Error: {context}; {}", optional_box_error(source))]
    UnexpectedError {
        context: String,
        #[source]
        source: Option<Box<dyn Error + Send + Sync>>,
    },

    // Transaction errors
    #[error("Transaction simulation failed: ({}) {error}", optional_error(error_parsed))]
    TransactionSimulationFailed {
        error: TransactionError,
        error_parsed: Option<TransactionErrorParsed>,
        logs: Vec<String>,
    },
    #[error(
        "Transaction failed to confirm ({signature}): ({}) {error}",
        optional_error(error_parsed)
    )]
    TransactionFailedToConfirm {
        signature: Signature,
        error: TransactionError,
        error_parsed: Option<TransactionErrorParsed>,
    },
    #[error("Transaction failed to confirm due to expired blockhash ({signature})")]
    TransactionBlockhashExpired { signature: Signature },

    // Specific errors
    #[error("Account not found: {0}")]
    AccountNotFound(AccountNotFound),
}

pub trait ToSolanaError<T>
where
    Self: Sized,
{
    fn with_context<S: Into<String>>(self, context_fn: impl FnOnce() -> S) -> Result<T, SolanaError>;
    fn context<S: Into<String>>(self, context: S) -> Result<T, SolanaError> {
        self.with_context(|| context)
    }
}

impl SolanaError {
    pub fn is_account_not_found(&self, account_type: AccountType) -> bool {
        match self {
            Self::AccountNotFound(inner) => inner.is_account(account_type),
            Self::UnexpectedError {
                source: Some(inner), ..
            } => {
                if let Some(inner) = inner.downcast_ref::<Self>() {
                    inner.is_account_not_found(account_type)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

impl<T, E> ToSolanaError<T> for Result<T, E>
where
    E: Into<Box<dyn Error + Send + Sync>>,
{
    fn with_context<S: Into<String>>(self, context_fn: impl FnOnce() -> S) -> Result<T, SolanaError> {
        self.map_err(|e| SolanaError::UnexpectedError {
            context: context_fn().into(),
            source: Some(e.into()),
        })
    }
}

impl<T> ToSolanaError<T> for Option<T> {
    fn with_context<S: Into<String>>(self, context_fn: impl FnOnce() -> S) -> Result<T, SolanaError> {
        self.ok_or_else(|| SolanaError::UnexpectedError {
            context: context_fn().into(),
            source: None,
        })
    }
}

impl From<AccountNotFound> for SolanaError {
    fn from(e: AccountNotFound) -> Self {
        Self::AccountNotFound(e)
    }
}

fn optional_error(e: &Option<impl Error>) -> String {
    match e {
        None => "-".into(),
        Some(e) => format!("{}", e),
    }
}

fn optional_box_error(e: &Option<Box<dyn Error + Send + Sync>>) -> String {
    match e {
        None => "-".into(),
        Some(e) => format!("{}", e),
    }
}
