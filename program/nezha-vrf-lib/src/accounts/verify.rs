use solana_program::account_info::AccountInfo;

use crate::error::NezhaVrfError;

use super::PDA;

pub trait VerifyPDA {
    type Error;
    fn verify(&self, account_info: &AccountInfo) -> Result<(), Self::Error>;
}

impl VerifyPDA for PDA {
    type Error = NezhaVrfError;
    fn verify(&self, account_info: &AccountInfo) -> Result<(), Self::Error> {
        self.verify_or(account_info.key, &NezhaVrfError::InvalidAccount)
    }
}
