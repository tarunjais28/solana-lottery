use solana_program::{program_error::ProgramError, system_instruction::SystemError};
use spl_token::error::TokenError;

use super::{AccountType, InvalidConstant, NezhaVrfError, SignatureType};

#[test]
fn error_round_trip() {
    let errors = [
        NezhaVrfError::InvalidInstruction,
        NezhaVrfError::MissingSignature(SignatureType::Admin),
        NezhaVrfError::InvalidConstant(InvalidConstant::SuperAdminKey),
        NezhaVrfError::InvalidAccount(AccountType::SwitchboardVrfLite),
        NezhaVrfError::InvalidAccount(AccountType::SwitchboardAuthority),
        NezhaVrfError::TokenProgramError(TokenError::InvalidInstruction),
        NezhaVrfError::SystemProgramError(SystemError::InvalidProgramId),
    ];

    for error in errors.clone() {
        let x: u32 = (error.clone()).into();
        let error_ = NezhaVrfError::try_from(x);
        assert_eq!(Ok(error.clone()), error_);
    }

    for error in errors {
        let p: ProgramError = error.clone().into();
        let x: u64 = p.into();
        let p_: ProgramError = x.into();
        if let ProgramError::Custom(x_) = p_ {
            let error_: NezhaVrfError = x_.into();
            assert_eq!(error_, error);
        } else {
            panic!("Did not encode/decode into a custom variant");
        }
    }
}
