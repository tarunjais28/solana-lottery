use solana_program::system_instruction::SystemError;
use spl_token::error::TokenError;

use crate::state::EpochStatus;

use super::{AccountType, InvalidConstant, SignatureType, StakingError};

#[test]
fn error_round_trip() {
    let errors = [
        StakingError::InvalidInstruction,
        StakingError::YieldNotWithdrawn,
        StakingError::UnknownError(1),
        StakingError::MissingSignature(SignatureType::Owner),
        StakingError::InvalidConstant(InvalidConstant::InvestorKey),
        StakingError::InvalidAccount(AccountType::DepositVault),
        StakingError::InvalidAccount(AccountType::Tier1PrizeVault),
        StakingError::InvalidEpochStatus(EpochStatus::Ended),
        StakingError::TokenProgramError(TokenError::InvalidInstruction),
        StakingError::SystemProgramError(SystemError::InvalidProgramId),
    ];

    for error in errors {
        let x: u32 = (error.clone()).into();
        let error_ = x.try_into();
        assert_eq!(Ok(error), error_);
    }
}
