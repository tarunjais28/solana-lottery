use solana_program::pubkey::Pubkey;
use static_pubkey::static_pubkey;

// Programs
pub const LENDING_PROGRAM_ID: Pubkey = static_pubkey!("FC81tbGt6JWRXidaWYFXxGnTk4VgobhJHATvTRVMqgWj");
pub const LENDING_REWARDS_PROGRAM_ID: Pubkey = static_pubkey!("3Katmm9dhvLQijAvomteYMo6rfVbY5NaCRNq9ZBqBgr6");

// Lending
pub const LENDING_MARKET_INFO: Pubkey = static_pubkey!("4XNif294wbrxj6tJ8K5Rg7SuaEACnu9s2L27i28MQB6E");
pub const LENDING_MARKET_AUTHORITY: Pubkey = static_pubkey!("sCDiYj7X7JmXg5fVq2nqED2q1Wqjo7PnqMgH3casMem");
pub const LENDING_POOL_INFO: Pubkey = static_pubkey!("Hx6LbkMHe69DYawhPyVNs8Apa6tyfogfzQV6a7XkwBUU");
pub const LENDING_POOL_USDC_ACCOUNT: Pubkey = static_pubkey!("CFp9kt8z3Epb1QSiEp3xA44KbSwuJxhFR3wQoerFqYS9");

// Rewards
pub const FARMING_POOL: Pubkey = static_pubkey!("8Eq2XZRQe2EjYiNmu7Lhgb2xVHqJ5wFvcVU6yH3CUn34");
pub const FARMING_POOL_AUTHORITY: Pubkey = static_pubkey!("4NWwKzVvEfKCsMeauE4cZHRR9K91FsFauxnrW6pK8H2E");
pub const FARMING_POOL_SHARE_TOKEN_ACCOUNT: Pubkey = static_pubkey!("3yNu5pg2DhtaxZbAwgUSsVnemqMn1WqxnBn6tgKGj7R2");
pub const FARMING_POOL_REWARDS_TOKEN_ACCOUNT: Pubkey = static_pubkey!("34R2ZVwg6uvJWFYjQ2LrrKFFaZ7CgsyZbMKwvfjxkvip");
pub const FARMING_POOL_REWARDS_TOKEN_B_ACCOUNT: Pubkey = static_pubkey!("FGAh5YjdcyzQ841skvGQGWyejK3uPiwpEdtMncJqe7f9");

#[derive(Clone)]
pub struct Mints {
    pub usdc_mint: Pubkey,
    pub share_token_mint: Pubkey,
    pub rewards_token_mint: Pubkey,
    pub rewards_token_b_mint: Pubkey,
}

const _MINTS: Mints = Mints {
    usdc_mint: static_pubkey!("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
    share_token_mint: static_pubkey!("62fDf5daUJ9jBz8Xtj6Bmw1bh1DvHn8AG4L9hMmxCzpu"),
    rewards_token_mint: static_pubkey!("Hjibp1cn2bSk1dkTdpbxez3YAiBGTLjzc8xZ8LbCCUHS"),
    rewards_token_b_mint: static_pubkey!("EgiD69Uhf8t13CRPKz1btmtHj7SogeEjyPHfnT4d13XN"),
};

#[cfg(target_os = "solana")]
pub fn get_mints() -> Mints {
    _MINTS.clone()
}

#[cfg(not(target_os = "solana"))]
use std::cell::RefCell;

#[cfg(not(target_os = "solana"))]
thread_local! {
    static MINTS: RefCell<Mints> = RefCell::new(_MINTS.clone());
}

#[cfg(not(target_os = "solana"))]
pub fn get_mints() -> Mints {
    MINTS.with(|mints| mints.borrow().clone())
}

#[cfg(not(target_os = "solana"))]
pub fn set_mints(mints_: Mints) {
    MINTS.with(|mints| {
        let mut refm_ = mints.borrow_mut();
        *refm_ = mints_;
    });
}
