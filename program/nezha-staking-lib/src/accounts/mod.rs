//! Definitions of all PDAs used by the contract.

use solana_program::pubkey::Pubkey;

#[cfg(test)]
mod tests;

mod account_type;
pub use account_type::AccountType;

mod pda;
pub use pda::VerifyPDA;
pub use pda::PDA;

use nezha_utils::seeds;

pub const PREFIX: &str = "staking";

/// [`crate::state::LatestEpoch`] account.
pub fn latest_epoch(program_id: &Pubkey) -> PDA {
    PDA::new(program_id, seeds!(PREFIX, "LATEST_EPOCH"), AccountType::LatestEpoch)
}

/// [`crate::state::Epoch`] account.
pub fn epoch(program_id: &Pubkey, index: u64) -> PDA {
    PDA::new(program_id, seeds!(PREFIX, "EPOCH", index), AccountType::Epoch)
}

/// [`crate::state::Stake`] account.
pub fn stake(program_id: &Pubkey, owner: &Pubkey) -> PDA {
    PDA::new(program_id, seeds!(PREFIX, "STAKE", *owner), AccountType::Stake)
}

/// [`crate::state::StakeUpdateRequest`] account.
pub fn stake_update_request(program_id: &Pubkey, owner: &Pubkey) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "STAKE_UPDATE_REQUEST", *owner),
        AccountType::StakeUpdateRequest,
    )
}

/// Vault authority is the authority of all vault token accounts.
pub fn vault_authority(program_id: &Pubkey) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "VAULT_AUTHORITY"),
        AccountType::VaultAuthority,
    )
}

/// USDC token account where all the deposits are held.
pub fn deposit_vault(program_id: &Pubkey) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "VAULT", "DEPOSIT"),
        AccountType::DepositVault,
    )
}

/// USDC token account where the treasury funds are held.
pub fn treasury_vault(program_id: &Pubkey) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "VAULT", "TREASURY"),
        AccountType::TreasuryVault,
    )
}

/// USDC token account where the insurance funds are held.
pub fn insurance_vault(program_id: &Pubkey) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "VAULT", "INSURANCE"),
        AccountType::InsuranceVault,
    )
}

/// USDC token account where the prize money is held.
pub fn prize_vault(program_id: &Pubkey, tier: u8) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "VAULT", "PRIZE", tier),
        match tier {
            1 => AccountType::Tier1PrizeVault,
            2 => AccountType::Tier2PrizeVault,
            3 => AccountType::Tier3PrizeVault,
            _ => unreachable!(),
        },
    )
}

/// USDC token account where the deposits which are pending, are held.
pub fn pending_deposit_vault(program_id: &Pubkey) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "VAULT", "PENDING_DEPOSIT"),
        AccountType::PendingDepositVault,
    )
}

/// Francium authority is the authority of francium related accounts.
pub fn francium_authority(program_id: &Pubkey) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "FRANCIUM_AUTHORITY"),
        AccountType::VaultAuthority,
    )
}

/// [`crate::state::EpochWinnersMeta`] account.
pub fn epoch_winners_meta(program_id: &Pubkey, epoch_index: u64) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "EPOCH_WINNERS_META", epoch_index),
        AccountType::EpochWinnersMeta,
    )
}

/// [`crate::state::EpochWinnersPage`] account.
pub fn epoch_winners_page(program_id: &Pubkey, epoch_index: u64, page_index: u32) -> PDA {
    PDA::new(
        program_id,
        seeds!(PREFIX, "EPOCH_WINNERS_PAGE", epoch_index, page_index),
        AccountType::EpochWinnersPage,
    )
}
