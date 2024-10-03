use solana_program::pubkey::Pubkey;
use spl_associated_token_account as ata;

use super::constants::Mints;

pub fn share_token_ata(francium_authority: &Pubkey, mints: &Mints) -> Pubkey {
    ata::get_associated_token_address(francium_authority, &mints.share_token_mint)
}

pub fn rewards_token_ata(francium_authority: &Pubkey, mints: &Mints) -> Pubkey {
    ata::get_associated_token_address(francium_authority, &mints.rewards_token_mint)
}

pub fn rewards_token_b_ata(francium_authority: &Pubkey, mints: &Mints) -> Pubkey {
    ata::get_associated_token_address(francium_authority, &mints.rewards_token_b_mint)
}

pub fn farming_info(francium_authority: &Pubkey, mints: &Mints) -> Pubkey {
    Pubkey::find_program_address(
        &[
            francium_authority.as_ref(),
            super::constants::FARMING_POOL.as_ref(),
            share_token_ata(francium_authority, mints).as_ref(),
        ],
        &super::constants::LENDING_REWARDS_PROGRAM_ID,
    )
    .0
}
