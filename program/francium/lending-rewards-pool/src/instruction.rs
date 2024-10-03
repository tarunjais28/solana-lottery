use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::pubkey::Pubkey;
use solana_program::program_error::ProgramError;
use crate::error::FarmingError::InstructionUnpackError;
use crate::instruction::FarmingInstructions::UpdateRewardsConfig;
use std::mem::size_of;
use borsh::{
    BorshSerialize,
    BorshDeserialize
};
use std::convert::TryInto;


#[derive(BorshSerialize, BorshDeserialize,Debug, Default, PartialEq)]
pub struct  FarmingPoolConfig {
    pub is_dual_rewards: u8,
    // rewards config
    pub pool_stake_cap: u64,
    pub user_stake_cap: u64,

    // rewards a
    pub rewards_start_slot: u64,
    pub rewards_end_slot: u64,
    pub rewards_per_day: u64,

    // rewards b
    pub rewards_start_slot_b: u64,
    pub rewards_end_slot_b: u64,
    pub rewards_per_day_b: u64,
}

#[derive(Debug, PartialEq)]
pub enum FarmingInstructions {
    // 0
    InitFarmingPool{
        config: FarmingPoolConfig
    },
    // 1
    /// Init farming user info account
    ///   0. `[signer]` User's wallet address
    ///   1. `[writable]` User's farming info account, PDA account
    ///             Pubkey::find_program_address(
    ///                    &[
    ///                        user_main_account.key.as_ref(),
    ///                        farming_pool_account.key.as_ref(),
    ///                        user_stake_token_account.key.as_ref()
    ///                    ],
    ///                    program_id,
    ///              );
    ///   2. `[writable]`  Farming pool account
    ///   3. `[writable]` User's token account to stake into the farming pool
    ///   4. `[writable]` User's rewards token account to receive rewards token
    ///   5. `[writable]` User's rewards token account b to receive rewards token b
    ///   6. `[]` System Program
    ///   7  `[]` Rent sysvar.
    InitFarmingUser,
    // 2
    UpdateRewardsConfig{
        config: FarmingPoolConfig
    },

    // 3
    /// Stake tokens to farming pool
    ///
    ///   0. `[signer]` User's wallet address
    ///   1. `[writable]` User's farming info account
    ///   2. `[writable]` User's token account to stake into the farming pool
    ///   3. `[writable]` User's rewards token account to receive rewards token
    ///   4. `[writable]` User's rewards token account b to receive rewards token b
    ///   5. `[writable]`  Farming pool account
    ///   6. `[]`  Farming pool's authority
    ///   7. `[writable]` Farming pool's stake token account to receive user staked token
    ///   8. `[writable]` Farming pool's rewards token account
    ///   9. `[writable]` Farming pool's rewards token account b
    ///   10. `[]` Token program id
    ///   11. `[]` Clock sysvar.
    Stake { amount: u64 },

    // 4
    /// UnStake tokens from farming pool
    ///
    ///   0. `[signer]` User's wallet address
    ///   1. `[writable]` User's farming info account
    ///   2. `[writable]` User's token account to stake into the farming pool
    ///   3. `[writable]` User's rewards token account to receive rewards token
    ///   4. `[writable]` User's rewards token account b to receive rewards token b
    ///   5. `[writable]`  Farming pool account
    ///   6. `[]`  Farming pool's authority
    ///   7. `[writable]` Farming pool's stake token account to receive user staked token
    ///   8. `[writable]` Farming pool's rewards token account
    ///   9. `[writable]` Farming pool's rewards token account b
    ///   10. `[]` Token program id
    ///   11. `[]` Clock sysvar.
    UnStake { amount: u64 },

    // 5
    UpdateRewardsToken,
    // 6
    UpdateRewardsTokenB,

    // 7
    ExtendRewards { end_slot: u64},

    // 8
    UpdateRewardsPerDay { rewards_per_day: u64},

    // 9
    AdminCloseEmptyAccount,
}

impl FarmingInstructions {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input.split_first().ok_or(InstructionUnpackError)?;

        Ok(match tag {
            0 => {
                let config = FarmingPoolConfig::try_from_slice(rest).unwrap();
                Self::InitFarmingPool{config}
            },
            1 => {
                Self::InitFarmingUser
            },
            2 => {
                let config = FarmingPoolConfig::try_from_slice(rest).unwrap();

                Self::UpdateRewardsConfig{config}
            },
            3=> {
                let amount_le_bytes = array_ref![rest, 0, 8];
                let amount = u64::from_le_bytes(*amount_le_bytes);
                Self::Stake {
                    amount
                }
            }
            4 => {
                let amount_le_bytes = array_ref![rest, 0, 8];
                let amount = u64::from_le_bytes(*amount_le_bytes);
                Self::UnStake {
                    amount
                }
            }
            5 => {
                Self::UpdateRewardsToken
            },
            6 => {
                Self::UpdateRewardsTokenB
            },
            7 => {
                let amount_le_bytes = array_ref![rest, 0, 8];
                let end_slot = u64::from_le_bytes(*amount_le_bytes);
                Self::ExtendRewards {
                    end_slot
                }
            }
            8 => {
                let amount_le_bytes = array_ref![rest, 0, 8];
                let rewards_per_day = u64::from_le_bytes(*amount_le_bytes);
                Self::UpdateRewardsPerDay {
                    rewards_per_day
                }
            }
            9 => {
                Self::AdminCloseEmptyAccount
            }
            _ => {
               return Err(InstructionUnpackError.into());
            }
        })
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());

        match &self {
            &Self::InitFarmingPool{config} => {
                buf.push(0);
                let param_bytes = config.try_to_vec().unwrap();
                buf.extend_from_slice(param_bytes.as_ref());
            }
            &Self::InitFarmingUser => {
                buf.push(1);
            }
            &Self::UpdateRewardsConfig{config} => {
                buf.push(2);
                let param_bytes = config.try_to_vec().unwrap();
                buf.extend_from_slice(param_bytes.as_ref());
            }
            &Self::Stake {amount} => {
                buf.push(3);
                let param_bytes = amount.try_to_vec().unwrap();
                buf.extend_from_slice(param_bytes.as_ref());
            }
            &Self::UnStake {amount} => {
                buf.push(4);
                let param_bytes = amount.try_to_vec().unwrap();
                buf.extend_from_slice(param_bytes.as_ref());
            }
            &Self::UpdateRewardsToken => {
                buf.push(5);
            }
            &Self::UpdateRewardsTokenB => {
                buf.push(6);
            }
            &Self::ExtendRewards {end_slot} => {
                buf.push(7);
                let param_bytes = end_slot.try_to_vec().unwrap();
                buf.extend_from_slice(param_bytes.as_ref());
            }
            &Self::UpdateRewardsPerDay {rewards_per_day} => {
                buf.push(8);
                let param_bytes = rewards_per_day.try_to_vec().unwrap();
                buf.extend_from_slice(param_bytes.as_ref());
            }
            &Self::AdminCloseEmptyAccount => {
                buf.push(9);
            }
        }
        buf
    }
}
