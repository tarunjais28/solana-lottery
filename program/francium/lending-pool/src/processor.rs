use std::convert::TryInto;
use flux_aggregator::solana_program::program_option::COption;
use arrayref::array_ref;

use solana_program::{
    pubkey::Pubkey,
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::{IsInitialized, Pack},
    program::{
        invoke_signed,
        invoke,
    },
    rent::Rent,
    sysvar::Sysvar,
    system_instruction,
    msg,
    program_error::ProgramError,
    clock::Clock,
};

use spl_token::{
  state::{
      Mint,
      Account
  }
};

use crate::{
    state::{
        LendingMarket,
        InitLendingMarketParams,
        LendingPool,
        InitLendingPoolParams,
        ReserveLiquidity,
        NewReserveLiquidityParams,
        LiquidityShares,
        NewLiquiditShareParams,
        LastUpdate
    },
    instruction::{
        LendingInstruction,
    },
    math::Decimal,
    error::LendingError,
};
use crate::state::{PROGRAM_VERSION, CreditToken, NewCreditParams, InterestRateModel};
use spl_token::solana_program::instruction::AccountMeta;

// lyfRaydiumProgram: 2nAAsYdXF3eTQzaeUQS3fr4o782dDg8L28mX39Wr5j8N [26,109,49,46,37,249,111,110,12,238,136,132,15,184,246,34,26,153,205,150,37,119,138,203,101,119,55,103,86,156,150,99]
const LYF_RAYDIUM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([26,109,49,46,37,249,111,110,12,238,136,132,15,184,246,34,26,153,205,150,37,119,138,203,101,119,55,103,86,156,150,99]);

// lyfOrcaProgram: DmzAmomATKpNp2rCBfYLS7CSwQqeQTsgRYJA1oSSAJaP [189,210,112,120,69,245,75,128,115,31,40,18,197,141,136,210,251,252,81,120,129,129,197,186,23,219,175,12,225,241,196,12]
const LYF_ORCA_PROGRAM_ID: Pubkey = Pubkey::new_from_array([189,210,112,120,69,245,75,128,115,31,40,18,197,141,136,210,251,252,81,120,129,129,197,186,23,219,175,12,225,241,196,12]);

/// Program state handler.
pub struct Processor {}

impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
            Ok(())
    }
}
