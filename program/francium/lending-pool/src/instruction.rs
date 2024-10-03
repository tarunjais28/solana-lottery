use crate::error::{LendingError};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    msg,
};
use std::convert::{TryInto, TryFrom};
use std::mem::size_of;
use solana_program::account_info::AccountInfo;

use borsh::{BorshSerialize, BorshDeserialize};
use solana_program::instruction::InstructionError::IncorrectProgramId;
use solana_program::pubkey::PUBKEY_BYTES;
use crate::state::{PROGRAM_VERSION, InterestRateModel};

/// Minimum number of multisignature signers (min N)
pub const MIN_SIGNERS: usize = 1;
/// Maximum number of multisignature signers (max N)
pub const MAX_SIGNERS: usize = 11;

#[derive(Debug, PartialEq)]
pub enum LendingInstruction {

    // 0
    /// Initializes a lending market.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` lending market account - uninitialized.
    ///   1. `[]` Rent sysvar.
    ///   2. `[]` Token program id.
    InitLendingMarket {
        /// Owner authority which can add new reserves
        owner: Pubkey,
    },

    // 2
    /// Initializes a new lending pool.
    ///
    /// Accounts expected by this instruction:
    ///   0. `[writable]` user's tkn account for this lending pool
    ///   1. `[writable]` user's share account for this lending pool
    ///   2. `[writable]` lending pool info account - uninitialized.
    ///   3. `[writable]` token mint of this lending pool
    ///   4. `[writable]` token account owned by this lending pool's authority(PDA)
    ///   5. `[writable]` fee receiver of this lending pool, share account owned by market-owner
    ///   6. `[writable]` share mint of this lending pool
    ///   7. `[writable]` share account owned by this lending pool's authority(PDA)
    ///   8. `[writable]` credit mint of this lending pool
    ///   9. `[writable]` credit account owned by this lending pool's authority(PDA)
    ///   10. `[writable]` lending market account this lending pool belong to
    ///   11. `[]` lending market's authority (PDA)
    ///   12. `[signer]` lending market's owner
    ///   13. `[signer]` user's token transfer authority
    ///   14  `[]` Clock sysvar.
    ///   15  `[]` Rent sysvar.
    ///   16  `[]` Token program id.
    InitLendingPool {
        /// Initial amount of liquidity to deposit into the new reserve
        liquidity_amount: u64,
    },

    // 3
    /// Update interest rate model
    ///   0. `[]` lending market info account
    ///   1. `[signer]` lending market owner
    ///   2. `[writable]` lending pool info account
    UpdateInterestModel {
        interest_model: InterestRateModel
    },

    // 4
    /// Deposit liquidity into a lending pool in exchange for share token. Share token represents a share
    /// of the reserve liquidity pool.
    ///
    /// Accounts expected by this instruction:
    ///    0. `[writable]` Source liquidity token account of user.
    ///    1. `[writable]` Destination share token account of user.
    ///    2. `[writable]` Lending pool info account
    ///    3. `[writable]` Liquidity token account of lending pool
    ///    4. `[witable]` Share token mint
    ///    5. `[]` Lending market account
    ///    6. `[]` Lending market authority
    ///    7. `[signer]` User transfer authority
    ///    8. `[]` Clock sysvar.
    ///    9. `[]` Token program id.
    DepositToLendingPool {
        /// Amount of liquidity to deposit in exchange for share tokens
        liquidity_amount: u64,
    },

    // 5
    /// Withdraw from lending pool by redeem share in exchange for liquidity.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` share token account of user.
    ///                     $authority can transfer $share_amount.
    ///   1.  `[writable]` Destination liquidity token account of user.
    ///   2. `[writable]` Lending pool account
    ///   3. `[writable]` share mint account
    ///   4. `[writable]` Liquidity token account owned by lending pool's authority(PDA)
    ///   5. `[]` Lending market account
    ///   6. `[]` Lending market's authority (PDA)
    ///   7. `[signer]` User transfer authority ($authority).
    ///   8. `[]` Clock sysvar.
    ///   9. `[]` Token program id.
    WithdrawFromLendingPool {
        /// Amount of share tokens to redeem in exchange for liquidity
        share_amount: u64,
    },

    // 6
    /// Withdraw from lending pool by redeem share in exchange for liquidity.
    /// **Use the liquidity amount as param, while not the share_amount**
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` share token account of user.
    ///                     $authority can transfer $share_amount.
    ///   1.  `[writable]` Destination liquidity token account of user.
    ///   2. `[writable]` Lending pool account
    ///   3. `[writable]` share mint account
    ///   4. `[writable]` Liquidity token account owned by lending pool's authority(PDA)
    ///   5. `[]` Lending market account
    ///   6. `[]` Lending market's authority (PDA)
    ///   7. `[signer]` User transfer authority ($authority).
    ///   8. `[]` Clock sysvar.
    ///   9. `[]` Token program id.
    WithdrawFromLendingPool2 {
        /// Amount of share tokens to redeem in exchange for liquidity
        liquidity_amount: u64,
    },

    /// 7 mint credit for borrower
    MintCredit{
        credit_amount: u64,
    },
    /// 8 burn borrower's credit
    BurnCredit{
        credit_amount: u64,
    },

    // 10
    /// Borrow liquidity from a reserve by depositing collateral tokens. Requires a refreshed
    /// strategy and reserve.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Source borrow reserve liquidity supply SPL Token account.
    ///   1. `[writable]` Destination liquidity token account.
    ///                     Minted by borrow reserve liquidity mint.
    ///   2. `[writable]` Borrow reserve account - refreshed.
    ///   3. `[writable]` Borrow reserve liquidity fee receiver account.
    ///                     Must be the fee account specified at InitReserve.
    ///   5. `[]` lending market account.
    ///   6. `[]` Derived lending market authority.
    ///   7. `[signer]` Strategy's authority
    ///   8. `[]` Clock sysvar.
    ///   9. `[]` Token program id.
    BorrowFromLendingPool {
        /// Amount of liquidity to borrow - u64::MAX for 100% of borrowing power
        tkn_amount: u64,
    },

    // 11
    /// Repay borrowed liquidity to a reserve. Requires a refreshed strategy and reserve.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` Source liquidity token account.
    ///                     Minted by repay reserve liquidity mint.
    ///                     $authority can transfer $liquidity_amount.
    ///   1. `[writable]` Destination repay reserve liquidity supply SPL Token account.
    ///   2. `[writable]` Repay reserve account - refreshed.
    ///   3. `[writable]` Strategy account - refreshed.
    ///   4. `[]` lending market account.
    ///   5. `[signer]` User transfer authority ($authority).
    ///   6. `[]` Clock sysvar.
    ///   7. `[]` Token program id.
    RepayToLendingPool {
        /// Amount of liquidity to repay - u64::MAX for 100% of borrowed amount
        tkn_amount: u64,
    },

    // 12
    /// update lending pool
    UpdateLendingPool,

    // 13
    /// adminWithdrawReserve
    /// withdraw the interest reserve by admin
    AdminWithdrawReserve,

    // 14
    /// AdminReduceReserve
    /// reduce reserve, and give the reserve to all depositors
    AdminReduceReserve(u64),

    // 15
    /// TransferMarketOwner
    TransferMarketOwner,

    // 16
    /// SetFeeReceiver
    SetFeeReceiver,

    // 17
    /// update lending pool 2
    UpdateLendingPool2,

}

impl LendingInstruction {
    /// Unpacks a byte buffer into a [lendingInstruction](enum.lendingInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        msg!("lendingInstruction: beginning of uppack");
        let (&tag, rest) = input
            .split_first()
            .ok_or(LendingError::InstructionUnpackError)?;
        Ok(match tag {
            0 => {
                let (owner, _rest) = Self::unpack_owner_pubkey(rest)?;
                Self::InitLendingMarket { owner }
            }
            2 => {
                // msg!("Rest length before uppack {}", rest.len());
                let (liquidity_amount, rest) = Self::unpack_u64(rest)?;
                Self::InitLendingPool {
                    liquidity_amount,
                }
            }
            3 => {
                let interest_model = InterestRateModel::unpack_from_slice(rest);

                Self::UpdateInterestModel {
                    interest_model
                }
            }
            4 => {
                let (liquidity_amount, _rest) = Self::unpack_u64(rest)?;
                Self::DepositToLendingPool { liquidity_amount }
            }
            5 => {
                let (collateral_amount, _rest) = Self::unpack_u64(rest)?;
                Self::WithdrawFromLendingPool { share_amount: collateral_amount }
            }

            6 => {
                let (liquidity_amount, _rest) = Self::unpack_u64(rest)?;
                Self::WithdrawFromLendingPool2 { liquidity_amount }
            }

            7 => {
                let (credit_amount, _rest) = Self::unpack_u64(rest)?;
                Self::MintCredit { credit_amount }
            }
            8 => {
                let (credit_amount, _rest) = Self::unpack_u64(rest)?;
                Self::BurnCredit { credit_amount }
            }
            10 => {
                let (liquidity_amount, _rest) = Self::unpack_u64(rest)?;
                Self::BorrowFromLendingPool { tkn_amount: liquidity_amount }
            }
            11 => {
                let (liquidity_amount, _rest) = Self::unpack_u64(rest)?;
                Self::RepayToLendingPool { tkn_amount: liquidity_amount }
            }
            12 => {
                Self::UpdateLendingPool
            }
            13 => {
                Self::AdminWithdrawReserve
            }
            14 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::AdminReduceReserve(amount)
            }
            15 => {
                Self::TransferMarketOwner
            }
            16 => {
                Self::SetFeeReceiver
            }
            17 => {
                Self::UpdateLendingPool2
            }
            _ => {
                msg!("Instruction cannot be unpacked");
                return Err(LendingError::InstructionUnpackError.into());
            }
        })
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() < 8 {
            msg!("u64 cannot be unpacked");
            return Err(LendingError::InstructionUnpackError.into());
        }
        let (amount, rest) = input.split_at(8);
        let amount = amount
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(LendingError::InstructionUnpackError)?;
        Ok((amount, rest))
    }

    fn unpack_u8(input: &[u8]) -> Result<(u8, &[u8]), ProgramError> {
        if input.is_empty() {
            msg!("u8 cannot be unpacked");
            return Err(LendingError::InstructionUnpackError.into());
        }
        let (amount, rest) = input.split_at(1);
        let amount = amount
            .get(..1)
            .and_then(|slice| slice.try_into().ok())
            .map(u8::from_le_bytes)
            .ok_or(LendingError::InstructionUnpackError)?;
        Ok((amount, rest))
    }

    fn unpack_owner_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), ProgramError> {
        if input.len() < PUBKEY_BYTES {
            msg!("Pubkey cannot be unpacked");
            return Err(LendingError::InstructionUnpackError.into());
        }
        let (key, rest) = input.split_at(PUBKEY_BYTES);
        let pk = Pubkey::try_from(key).map_err(|_| LendingError::InvalidAccountOwner)?;
        Ok((pk, rest))
    }

    /// Packs a [lendingInstruction](enum.lendingInstruction.html) into a byte buffer.
    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match self {
            Self::InitLendingMarket { owner } => {
                buf.push(0);
                buf.extend_from_slice(owner.as_ref());
            }
            Self::InitLendingPool {
                liquidity_amount,
            } => {
                buf.push(2);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
            Self::UpdateInterestModel {
                interest_model
            } => {
                buf.push(3);
                buf.extend_from_slice(&interest_model.to_bytes())
            }
            Self::DepositToLendingPool { liquidity_amount } => {
                buf.push(4);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
            Self::WithdrawFromLendingPool { share_amount: collateral_amount } => {
                buf.push(5);
                buf.extend_from_slice(&collateral_amount.to_le_bytes());
            }

            Self::WithdrawFromLendingPool2 { liquidity_amount } => {
                buf.push(6);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }

            Self::MintCredit { credit_amount } => {
                buf.push(7);
                buf.extend_from_slice(&credit_amount.to_le_bytes());
            }
            Self::BurnCredit { credit_amount } => {
                buf.push(8);
                buf.extend_from_slice(&credit_amount.to_le_bytes());
            }
            Self::BorrowFromLendingPool { tkn_amount: liquidity_amount } => {
                buf.push(10);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
            Self::RepayToLendingPool { tkn_amount: liquidity_amount } => {
                buf.push(11);
                buf.extend_from_slice(&liquidity_amount.to_le_bytes());
            }
            Self::UpdateLendingPool => {
                buf.push(12);
            }
            Self::AdminWithdrawReserve => {
                buf.push(13);
            }
            Self::AdminReduceReserve(amount) => {
                buf.push(14);
                buf.extend_from_slice(&amount.to_le_bytes());
            }
            Self::TransferMarketOwner => {
                buf.push(15)
            }
            Self::SetFeeReceiver => {
                buf.push(16)
            }
            Self::UpdateLendingPool2 => {
                buf.push(17)
            }
        }
        buf
    }
}

