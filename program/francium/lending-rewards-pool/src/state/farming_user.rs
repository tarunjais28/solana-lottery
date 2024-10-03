use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::Pubkey;
use spl_math::uint::U256;

const INVALID_USER_VERSION: u8 = 0;
const CURRENT_USER_VERSION: u8 = 1;

// [1,8,8,8,32,32,32,32,32,128]
const FARMING_USER_LEN: usize = 313;
#[derive(Debug, Default, PartialEq)]
pub struct FarmingUser {
    pub version: u8,
    pub staked_amount: u64,
    pub rewards_debt: u64,
    pub rewards_debt_b: u64,
    pub farming_pool: Pubkey,
    pub user_main: Pubkey,
    pub stake_token_account: Pubkey,
    pub rewards_token_accont: Pubkey,
    pub rewards_token_account_b: Pubkey,
    // pending 128
}

impl FarmingUser {
    pub fn init(
        &mut self,
        farming_pool: &Pubkey,
        user_main: &Pubkey,
        stake_token_account: &Pubkey,
        rewards_token_accont: &Pubkey,
        rewards_token_account_b: &Pubkey,
    ) {
        self.version = CURRENT_USER_VERSION;
        self.farming_pool = farming_pool.clone();
        self.user_main = user_main.clone();
        self.stake_token_account = stake_token_account.clone();
        self.rewards_token_accont = rewards_token_accont.clone();
        self.rewards_token_account_b = rewards_token_account_b.clone();
    }

    pub fn stake(
        &mut self,
        amount: u64,
        accumulated_rewards_per_share: u128,
        accumulated_rewards_per_share_b: u128,
    ){
        self.staked_amount = self.staked_amount.checked_add(amount).unwrap();

        self.update_rewards_debt(
            accumulated_rewards_per_share,
            accumulated_rewards_per_share_b
        )
    }

    pub fn un_stake(
        &mut self,
        amount: u64,
        accumulated_rewards_per_share: u128,
        accumulated_rewards_per_share_b: u128,
    ){
        self.staked_amount = self.staked_amount.checked_sub(amount).unwrap();

        self.update_rewards_debt(
            accumulated_rewards_per_share,
            accumulated_rewards_per_share_b
        )
    }

    pub fn pending_rewards(
        & self,
        accumulated_rewards_per_share: u128,
        accumulated_rewards_per_share_b: u128,
    ) -> (u64, u64) {
        if self.staked_amount == 0 {
            return (0,0);
        }

        let pending_rewards = U256::from(self.staked_amount)
            .checked_mul(U256::from(accumulated_rewards_per_share)).unwrap()
            .checked_div(U256::from(1000_000_000_000u128)).unwrap()
            .checked_sub(U256::from(self.rewards_debt)).unwrap();

        assert!(pending_rewards.lt(&U256::from(u64::MAX)));

        let pending_rewards_b = U256::from(self.staked_amount)
            .checked_mul(U256::from(accumulated_rewards_per_share_b)).unwrap()
            .checked_div(U256::from(1000_000_000_000u128)).unwrap()
            .checked_sub(U256::from(self.rewards_debt_b)).unwrap();

        assert!(pending_rewards_b.lt(&U256::from(u64::MAX)));

        (pending_rewards.as_u64(), pending_rewards_b.as_u64())
    }

    pub fn update_rewards_debt(
        &mut self,
        accumulated_rewards_per_share: u128,
        accumulated_rewards_per_share_b: u128,
    ) {
        let rewards_debt = U256::from(self.staked_amount)
            .checked_mul(U256::from(accumulated_rewards_per_share)).unwrap()
            .checked_div(U256::from(1000_000_000_000u128)).unwrap();

        assert!(rewards_debt.lt(&U256::from(u64::MAX)));

        self.rewards_debt = rewards_debt.as_u64();

        let rewards_debt_b = U256::from(self.staked_amount)
            .checked_mul(U256::from(accumulated_rewards_per_share_b)).unwrap()
            .checked_div(U256::from(1000_000_000_000u128)).unwrap();

        assert!(rewards_debt_b.lt(&U256::from(u64::MAX)));

        self.rewards_debt_b = rewards_debt_b.as_u64();
    }
}

impl Sealed for FarmingUser {}
impl IsInitialized for FarmingUser {
    fn is_initialized(&self) -> bool {
        self.version != INVALID_USER_VERSION
    }
}

impl Pack for FarmingUser {
    const LEN: usize = FARMING_USER_LEN;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, FARMING_USER_LEN];

        let (
            version,
            staked_amount,
            rewards_debt,
            rewards_debt_b,
            farming_pool,
            user_main,
            stake_token_account,
            rewards_token_account,
            rewards_token_account_b,
            _padding
        ) =  mut_array_refs![
            output,
            1,8,8,8,32,32,32,32,32,128
        ];

        version[0] = self.version;
        staked_amount.copy_from_slice(&self.staked_amount.to_le_bytes());
        rewards_debt.copy_from_slice(&self.rewards_debt.to_le_bytes());
        rewards_debt_b.copy_from_slice(&self.rewards_debt_b.to_le_bytes());
        farming_pool.copy_from_slice(self.farming_pool.as_ref());
        user_main.copy_from_slice(self.user_main.as_ref());
        stake_token_account.copy_from_slice(self.stake_token_account.as_ref());
        rewards_token_account.copy_from_slice(self.rewards_token_accont.as_ref());
        rewards_token_account_b.copy_from_slice(self.rewards_token_account_b.as_ref());
    }

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, FARMING_USER_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
            let (
            version,
            staked_amount,
            rewards_debt,
            rewards_debt_b,
            farming_pool,
            user_main,
            stake_token_account,
            rewards_token_account,
            rewards_token_account_b,
            _padding
        ) = array_refs![
            input,
            1,8,8,8,32,32,32,32,32,128
        ];

        Ok(Self {
            version: version[0],
            staked_amount: u64::from_le_bytes(*staked_amount),
            rewards_debt: u64::from_le_bytes(*rewards_debt),
            rewards_debt_b: u64::from_le_bytes(*rewards_debt_b),
            farming_pool: Pubkey::new_from_array(*farming_pool),
            user_main: Pubkey::new_from_array(*user_main),
            stake_token_account: Pubkey::new_from_array(*stake_token_account),
            rewards_token_accont: Pubkey::new_from_array(*rewards_token_account),
            rewards_token_account_b: Pubkey::new_from_array(*rewards_token_account_b)
        })
    }
}
