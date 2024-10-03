#[cfg(test)]

mod tests {
    use crate::state::farming_pool::FarmingPool;
    use crate::state::farming_user::FarmingUser;

    #[test]
    fn farming_pool_update_pool_rewards_b_test() {
        let mut pool = FarmingPool{
            version: 0,
            is_dual_rewards: 0,
            admin: Default::default(),
            token_program_id: Default::default(),
            pool_authority: Default::default(),
            staked_token_mint: Default::default(),
            staked_token_account: Default::default(),
            rewards_token_mint: Default::default(),
            rewards_token_account: Default::default(),
            rewards_token_mint_b: Default::default(),
            rewards_token_account_b: Default::default(),
            pool_stake_cap: 10000000000000000,
            user_stake_cap: 10000000000000000,
            rewards_start_slot: 10000000,
            rewards_end_slot: 12322131,
            rewards_per_day: 21348978129,
            rewards_start_slot_b: 10000000,
            rewards_end_slot_b: 12322131,
            rewards_per_day_b: 21348978129,
            total_staked_amount: 265676815236,
            last_update_slot: 11000000,
            accumulated_rewards_per_share: 231784612783467128,
            accumulated_rewards_per_share_b: 0
        };

        let currentSlot = 11111111;
        pool.update_pool_rewards_b(currentSlot);

        println!("{}, {}",
                 pool.accumulated_rewards_per_share_b,
                 41335837462u128
        );

        assert!(pool.accumulated_rewards_per_share_b == 41335837462u128);

        let currentSlot = 11211111;
        pool.update_pool_rewards_b(currentSlot);

        println!("{}, {}",
                 pool.accumulated_rewards_per_share_b,
                 78538128380u128
        );

        assert!(pool.accumulated_rewards_per_share_b == 78538128380u128);

        let currentSlot = 11311111;
        pool.update_pool_rewards_b(currentSlot);

        println!("{}, {}",
                 pool.accumulated_rewards_per_share_b,
                 115740419298u128
        );

        assert!(pool.accumulated_rewards_per_share_b == 115740419298u128);


        let currentSlot = 11411111;
        pool.update_pool_rewards_b(currentSlot);

        println!("{}, {}",
                 pool.accumulated_rewards_per_share_b,
                 152942710216u128
        );

        assert!(pool.accumulated_rewards_per_share_b == 152942710216u128);
    }

    #[test]
    fn farming_pool_update_pool_rewards_test() {
        let mut pool = FarmingPool{
            version: 0,
            is_dual_rewards: 0,
            admin: Default::default(),
            token_program_id: Default::default(),
            pool_authority: Default::default(),
            staked_token_mint: Default::default(),
            staked_token_account: Default::default(),
            rewards_token_mint: Default::default(),
            rewards_token_account: Default::default(),
            rewards_token_mint_b: Default::default(),
            rewards_token_account_b: Default::default(),
            pool_stake_cap: 10000000000000000,
            user_stake_cap: 10000000000000000,
            rewards_start_slot: 10000000,
            rewards_end_slot: 12322131,
            rewards_per_day: 21348978129,
            rewards_start_slot_b: 10000000,
            rewards_end_slot_b: 12322131,
            rewards_per_day_b: 21348978129,
            total_staked_amount: 265676815236,
            last_update_slot: 11000000,
            accumulated_rewards_per_share: 0,
            accumulated_rewards_per_share_b: 0
        };

        let currentSlot = 11111111;
        pool.update_pool_rewards(currentSlot);

        println!("{}, {}",
                 pool.accumulated_rewards_per_share,
                 41335837462u128
        );

        assert!(pool.accumulated_rewards_per_share == 41335837462u128);

        let currentSlot = 11211111;
        pool.update_pool_rewards(currentSlot);

        println!("{}, {}",
                 pool.accumulated_rewards_per_share,
                 78538128380u128
        );

        assert!(pool.accumulated_rewards_per_share == 78538128380u128);

        let currentSlot = 11311111;
        pool.update_pool_rewards(currentSlot);

        println!("{}, {}",
                 pool.accumulated_rewards_per_share,
                 115740419298u128
        );

        assert!(pool.accumulated_rewards_per_share == 115740419298u128);


        let currentSlot = 11411111;
        pool.update_pool_rewards(currentSlot);

        println!("{}, {}",
                 pool.accumulated_rewards_per_share,
                 152942710216u128
        );

        assert!(pool.accumulated_rewards_per_share == 152942710216u128);
    }

    #[test]
    fn farming_user_update_debt_test() {
        let mut user = FarmingUser{
            version: 0,
            staked_amount: 12345678,
            rewards_debt: 1234541335837462u64,
            rewards_debt_b: 1234541335837462u64,
            farming_pool: Default::default(),
            user_main: Default::default(),
            stake_token_account: Default::default(),
            rewards_token_accont: Default::default(),
            rewards_token_account_b: Default::default()
        };
        user.update_rewards_debt(
            41335837462u128,
            41335837462u128
        );

        assert!(
            user.rewards_debt == 510318 &&
            user.rewards_debt_b == 510318
        );

        user.update_rewards_debt(
            115740419298u128,
            115740419298u128
        );

        assert!(
            user.rewards_debt == 1428893 &&
            user.rewards_debt_b == 1428893
        );

        user.update_rewards_debt(
            152942710216u128,
            152942710216u128
        );

        assert!(
            user.rewards_debt == 1888181 &&
            user.rewards_debt_b == 1888181
        );
    }

    #[test]
    fn farming_user_pending_rewards_test() {
        let mut user = FarmingUser{
            version: 0,
            staked_amount: 12345678,
            rewards_debt: 1000u64,
            rewards_debt_b: 1000u64,
            farming_pool: Default::default(),
            user_main: Default::default(),
            stake_token_account: Default::default(),
            rewards_token_accont: Default::default(),
            rewards_token_account_b: Default::default()
        };


        let (r0,r1) = user.pending_rewards(
            41335837462u128,
            41335837462u128
        );

        assert!(
            r0 == 509318 &&
            r1== 509318
        );

    }
}