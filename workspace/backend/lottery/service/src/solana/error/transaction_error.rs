use num_traits::cast::FromPrimitive;

use nezha_staking::error::StakingError;
use solana_program::{
    instruction::{Instruction, InstructionError},
    pubkey::Pubkey,
    system_instruction::SystemError,
    system_program,
};
use solana_sdk::transaction::TransactionError;
use spl_token::error::TokenError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TransactionErrorParsed {
    #[error("Staking Error: {0}")]
    StakingError(StakingError),
    #[error("System Error: {0}")]
    SystemError(SystemError),
    #[error("Token Error: {0}")]
    TokenError(TokenError),
}

impl TransactionErrorParsed {
    pub fn from_instructions(
        ixs: &[Instruction],
        staking_program_id: Pubkey,
        error: &TransactionError,
    ) -> Option<Self> {
        if let TransactionError::InstructionError(idx, ix_error) = error {
            let idx = *idx;
            if let InstructionError::Custom(err_code) = ix_error {
                let err_code = *err_code;
                let ix = &ixs[idx as usize];
                if ix.program_id == staking_program_id {
                    let e = StakingError::from(err_code);
                    return Some(TransactionErrorParsed::StakingError(e));
                } else if ix.program_id == system_program::id() {
                    let e = SystemError::from_u32(err_code)?;
                    return Some(TransactionErrorParsed::SystemError(e));
                } else if ix.program_id == spl_token::id() {
                    let e = TokenError::from_u32(err_code)?;
                    return Some(TransactionErrorParsed::TokenError(e));
                }
            }
        }
        None
    }
}
