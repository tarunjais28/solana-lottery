use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

use crate::state::farming_user::FarmingUser;
use solana_program::clock::{
    Slot, DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY,
};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::Pubkey;
use solana_program::account_info::AccountInfo;
use std::cmp::{max, min};
use spl_math::uint::U256;
use crate::instruction::FarmingPoolConfig;

const INVALID_POOL_VERSION: u8 = 0;
const CURRENT_POOL_VERSION: u8 = 1;
const SLOTS_PER_DAY: u64 = DEFAULT_TICKS_PER_SECOND * SECONDS_PER_DAY / DEFAULT_TICKS_PER_SLOT;

// [1,1,32,32,32,32,32,32,32,32,32,8,8,8,8,8,8,8,8,8,8,16,16,128]
const FARMING_POOL_LEN: usize = 530;
#[derive(Debug, Default, PartialEq)]
pub struct FarmingPool {
    pub version: u8,
    pub is_dual_rewards: u8,
    pub admin: Pubkey,
    pub token_program_id: Pubkey,
    pub pool_authority: Pubkey,

    // staked_token
    pub staked_token_mint: Pubkey,
    pub staked_token_account: Pubkey,

    // reward_token
    pub rewards_token_mint: Pubkey,
    pub rewards_token_account: Pubkey,

    // reward_token_b
    pub rewards_token_mint_b: Pubkey,
    pub rewards_token_account_b: Pubkey,

    // rewards config
    pub pool_stake_cap: u64,
    pub user_stake_cap: u64,
    // rewards a
    pub rewards_start_slot: Slot,
    pub rewards_end_slot: Slot,
    pub rewards_per_day: u64,

    // rewards b
    pub rewards_start_slot_b: Slot,
    pub rewards_end_slot_b: Slot,
    pub rewards_per_day_b: u64,

    pub total_staked_amount: u64,
    pub last_update_slot: Slot,

    pub accumulated_rewards_per_share: u128,
    pub accumulated_rewards_per_share_b: u128,
    // padding 128
}

impl FarmingPool {
    pub fn init(
        &mut self,
        admin: &Pubkey,
        token_program_id: &Pubkey,
        pool_authority: &Pubkey,
        staked_token_mint: &Pubkey,
        staked_token_account: &Pubkey,
        rewards_token_mint: &Pubkey,
        rewards_token_account: &Pubkey,
        rewards_token_mint_b: &Pubkey,
        rewards_token_account_b: &Pubkey,
        config: &FarmingPoolConfig
    ) {
        self.version = CURRENT_POOL_VERSION;

        self.admin = admin.clone();
        self.token_program_id = token_program_id.clone();
        self.pool_authority = pool_authority.clone();

        self.staked_token_mint = staked_token_mint.clone();
        self.staked_token_account = staked_token_account.clone();

        self.rewards_token_mint = rewards_token_mint.clone();
        self.rewards_token_account = rewards_token_account.clone();

        self.rewards_token_mint_b = rewards_token_mint_b.clone();
        self.rewards_token_account_b = rewards_token_account_b.clone();

        self.is_dual_rewards = config.is_dual_rewards;

        self.pool_stake_cap = config.pool_stake_cap;
        self.user_stake_cap = config.user_stake_cap;

        self.rewards_start_slot = config.rewards_start_slot;
        self.rewards_end_slot = config.rewards_end_slot;
        self.rewards_per_day = config.rewards_per_day;

        self.rewards_start_slot_b = config.rewards_start_slot_b;
        self.rewards_end_slot_b = config.rewards_end_slot_b;
        self.rewards_per_day_b = config.rewards_per_day_b;
    }

    pub fn update_rewards_config(
        &mut self,
        config: &FarmingPoolConfig
    ) {
        self.is_dual_rewards = config.is_dual_rewards;

        self.pool_stake_cap = config.pool_stake_cap;
        self.user_stake_cap = config.user_stake_cap;

        self.rewards_start_slot = config.rewards_start_slot;
        self.rewards_end_slot = config.rewards_end_slot;
        self.rewards_per_day = config.rewards_per_day;

        self.rewards_start_slot_b = config.rewards_start_slot_b;
        self.rewards_end_slot_b = config.rewards_end_slot_b;
        self.rewards_per_day_b = config.rewards_per_day_b;
    }

    pub fn total_rewards(&self) -> (u64, u64){
        (
            (self.rewards_end_slot as u128)
                .checked_sub(self.rewards_start_slot as u128).unwrap()
                .checked_mul(self.rewards_per_day as u128).unwrap()
                .checked_div(SLOTS_PER_DAY as u128).unwrap()
                 as u64
            ,
            (self.rewards_end_slot_b as u128)
                .checked_sub(self.rewards_start_slot_b as u128).unwrap()
                .checked_mul(self.rewards_per_day_b as u128).unwrap()
                .checked_div(SLOTS_PER_DAY as u128).unwrap()
                as u64
        )
    }

    pub fn extend_rewards(&self, end_slot: u64) -> u64{
        (end_slot as u128)
            .checked_sub(self.rewards_end_slot as u128).unwrap()
            .checked_mul(self.rewards_per_day as u128).unwrap()
            .checked_div(SLOTS_PER_DAY as u128).unwrap()
            as u64
    }


    pub fn update_pool_rewards_b(&mut self, current_slot: Slot) {
        if self.last_update_slot >= current_slot  {
            return;
        }

        if current_slot < self.rewards_start_slot_b
            || self.last_update_slot > self.rewards_end_slot_b
            || self.total_staked_amount == 0 {
            self.last_update_slot = current_slot;
            return;
        }

        let slot_start = max(self.rewards_start_slot_b, self.last_update_slot);

        let slot_end = min(self.rewards_end_slot_b, current_slot);

        let slots_elapsed = slot_end.checked_sub(slot_start).unwrap();
        self.last_update_slot = current_slot;

        self.accumulated_rewards_per_share_b =
            U256::from(self.accumulated_rewards_per_share_b)
                .checked_add(
                    U256::from(slots_elapsed)
                        .checked_mul(U256::from(self.rewards_per_day_b)).unwrap()
                        .checked_mul(U256::from(1000_000_000_000u128)).unwrap()
                        .checked_div(U256::from(SLOTS_PER_DAY)).unwrap()
                        .checked_div(U256::from(self.total_staked_amount)).unwrap(),
                ).unwrap().as_u128();
    }

    pub fn update_pool_rewards(&mut self, current_slot: Slot) {
        if self.last_update_slot >= current_slot {
            return;
        }

        if current_slot < self.rewards_start_slot
            || self.last_update_slot > self.rewards_end_slot
            || self.total_staked_amount == 0{

            self.last_update_slot = current_slot;
            return;
        }

        let slot_start = max(self.rewards_start_slot, self.last_update_slot);
        let slot_end = min(self.rewards_end_slot, current_slot);

        let slots_elapsed = slot_end.checked_sub(slot_start).unwrap();
        self.last_update_slot = current_slot;

        self.accumulated_rewards_per_share =
            U256::from(self.accumulated_rewards_per_share)
                .checked_add(
                    U256::from(slots_elapsed)
                        .checked_mul(U256::from(self.rewards_per_day)).unwrap()
                        .checked_mul(U256::from(1000_000_000_000u128)).unwrap()
                        .checked_div(U256::from(SLOTS_PER_DAY)).unwrap()
                        .checked_div(U256::from(self.total_staked_amount)).unwrap(),
                ).unwrap().as_u128();
    }

    pub fn update_pool(
        &mut self, current_slot: Slot
    ){
        self.update_pool_rewards(current_slot);

        if self.is_dual_rewards() {
            self.update_pool_rewards_b(current_slot);
        }
    }

    pub fn is_dual_rewards(&self) -> bool{
        self.is_dual_rewards == 1
    }

    pub fn pending_rewards(&mut self, current_slot: Slot, user: &FarmingUser) -> (u64,u64) {
        self.update_pool(current_slot);

        user.pending_rewards(
            self.accumulated_rewards_per_share,
            self.accumulated_rewards_per_share_b
        )
    }

    pub fn stake(&mut self, user: &mut FarmingUser, current_slot: Slot, amount: u64) -> (u64,u64) {
        let pending_rewards = self.pending_rewards(current_slot, user);

        user.stake(
            amount,
            self.accumulated_rewards_per_share,
            self.accumulated_rewards_per_share_b
        );

        self.total_staked_amount = self.total_staked_amount.checked_add(amount).unwrap();

        pending_rewards
    }

    pub fn un_stake(&mut self, user: &mut FarmingUser, current_slot: Slot, amount: u64) -> (u64,u64)  {
        let pending_rewards = self.pending_rewards(current_slot, user);

        user.un_stake(
            amount,
            self.accumulated_rewards_per_share,
            self.accumulated_rewards_per_share_b
        );

        self.total_staked_amount = self.total_staked_amount.checked_sub(amount).unwrap();

        pending_rewards
    }
}

impl Sealed for FarmingPool {}
impl IsInitialized for FarmingPool {
    fn is_initialized(&self) -> bool {
        self.version != INVALID_POOL_VERSION
    }
}

impl Pack for FarmingPool {
    const LEN: usize = FARMING_POOL_LEN;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, FARMING_POOL_LEN];

        let (
            version,
            is_dual_rewards,
            admin,
            pool_authority,
            token_program_id,
            staked_token_mint,
            staked_token_account,
            rewards_token_mint,
            rewards_token_account,
            rewards_token_mint_b,
            rewards_token_account_b,
            pool_stake_cap,
            user_stake_cap,
            rewards_start_slot,
            rewards_end_slot,
            rewards_per_day,
            rewards_start_slot_b,
            rewards_end_slot_b,
            rewards_per_day_b,
            total_staked_amount,
            last_update_slot,
            accumulated_rewards_per_share,
            accumulated_rewards_per_share_b,
            _padding,
        ) = mut_array_refs![output, 1,1,32,32,32,32,32,32,32,32,32,8,8,8,8,8,8,8,8,8,8,16,16,128];

        version[0] = self.version;
        is_dual_rewards[0] = self.is_dual_rewards;
        admin.copy_from_slice(self.admin.as_ref());
        pool_authority.copy_from_slice(self.pool_authority.as_ref());
        token_program_id.copy_from_slice(self.token_program_id.as_ref());
        staked_token_mint.copy_from_slice(self.staked_token_mint.as_ref());
        staked_token_account.copy_from_slice(self.staked_token_account.as_ref());
        rewards_token_mint.copy_from_slice(self.rewards_token_mint.as_ref());
        rewards_token_account.copy_from_slice(self.rewards_token_account.as_ref());
        rewards_token_mint_b.copy_from_slice(self.rewards_token_mint_b.as_ref());
        rewards_token_account_b.copy_from_slice(self.rewards_token_account_b.as_ref());
        pool_stake_cap.copy_from_slice(&self.pool_stake_cap.to_le_bytes());
        user_stake_cap.copy_from_slice(&self.user_stake_cap.to_le_bytes());
        rewards_start_slot.copy_from_slice(&self.rewards_start_slot.to_le_bytes());
        rewards_end_slot.copy_from_slice(&self.rewards_end_slot.to_le_bytes());
        rewards_per_day.copy_from_slice(&self.rewards_per_day.to_le_bytes());
        rewards_start_slot_b.copy_from_slice(&self.rewards_start_slot_b.to_le_bytes());
        rewards_end_slot_b.copy_from_slice(&self.rewards_end_slot_b.to_le_bytes());
        rewards_per_day_b.copy_from_slice(&self.rewards_per_day_b.to_le_bytes());

        total_staked_amount.copy_from_slice(&self.total_staked_amount.to_le_bytes());
        accumulated_rewards_per_share
            .copy_from_slice(&self.accumulated_rewards_per_share.to_le_bytes());
        accumulated_rewards_per_share_b
            .copy_from_slice(&self.accumulated_rewards_per_share_b.to_le_bytes());
        last_update_slot.copy_from_slice(&self.last_update_slot.to_le_bytes());
    }

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, FARMING_POOL_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            is_dual_rewards,
            admin,
            pool_authority,
            token_program_id,
            staked_token_mint,
            staked_token_account,
            rewards_token_mint,
            rewards_token_account,
            rewards_token_mint_b,
            rewards_token_account_b,
            pool_stake_cap,
            user_stake_cap,
            rewards_start_slot,
            rewards_end_slot,
            rewards_per_day,
            rewards_start_slot_b,
            rewards_end_slot_b,
            rewards_per_day_b,
            total_staked_amount,
            last_update_slot,
            accumulated_rewards_per_share,
            accumulated_rewards_per_share_b,
            _padding,
        ) = array_refs![input, 1,1,32,32,32,32,32,32,32,32,32,8,8,8,8,8,8,8,8,8,8,16,16,128];

        Ok(Self {
            version: version[0],
            is_dual_rewards: is_dual_rewards[0],
            admin: Pubkey::new_from_array(*admin),
            pool_authority: Pubkey::new_from_array(*pool_authority),
            token_program_id: Pubkey::new_from_array(*token_program_id),
            staked_token_mint: Pubkey::new_from_array(*staked_token_mint),
            staked_token_account: Pubkey::new_from_array(*staked_token_account),
            rewards_token_mint: Pubkey::new_from_array(*rewards_token_mint),
            rewards_token_account: Pubkey::new_from_array(*rewards_token_account),
            rewards_token_mint_b : Pubkey::new_from_array(*rewards_token_mint_b),
            rewards_token_account_b: Pubkey::new_from_array(*rewards_token_account_b),
            pool_stake_cap: u64::from_le_bytes(*pool_stake_cap),
            user_stake_cap: u64::from_le_bytes(*user_stake_cap),
            rewards_start_slot: Slot::from_le_bytes(*rewards_start_slot),
            rewards_end_slot: Slot::from_le_bytes(*rewards_end_slot),
            rewards_per_day: u64::from_le_bytes(*rewards_per_day),
            rewards_start_slot_b: Slot::from_le_bytes(*rewards_start_slot_b),
            rewards_end_slot_b: Slot::from_le_bytes(*rewards_end_slot_b),
            rewards_per_day_b: u64::from_le_bytes(*rewards_per_day_b),
            total_staked_amount: u64::from_le_bytes(*total_staked_amount),
            last_update_slot: Slot::from_le_bytes(*last_update_slot),
            accumulated_rewards_per_share: u128::from_le_bytes(*accumulated_rewards_per_share),
            accumulated_rewards_per_share_b: u128::from_le_bytes(*accumulated_rewards_per_share_b),
        })
    }
}
