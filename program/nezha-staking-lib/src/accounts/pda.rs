use solana_program::account_info::AccountInfo;

use crate::error::StakingError;

pub type PDA = nezha_utils::pda::PDA<super::AccountType>;

pub trait VerifyPDA {
    type Error;
    fn verify(&self, account_info: &AccountInfo) -> Result<(), Self::Error>;
}

impl VerifyPDA for PDA {
    type Error = StakingError;
    fn verify(&self, account_info: &AccountInfo) -> Result<(), Self::Error> {
        self.verify_or(account_info.key, &StakingError::InvalidAccount)
    }
}
