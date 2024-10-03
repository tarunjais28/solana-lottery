use solana_sdk::pubkey::Pubkey;

pub fn user_balance_address(program_id: &Pubkey, user_pubkey: &Pubkey, token_mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            staking::BALANCE_SEED.as_ref(),
            user_pubkey.as_ref(),
            token_mint.as_ref(),
        ],
        program_id,
    )
    .0
}

pub fn vault_token_address(program_id: &Pubkey, token_mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[staking::VAULT_SEED.as_ref(), token_mint.as_ref()], program_id).0
}

pub fn vault_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[staking::VAULT_SEED.as_ref()], program_id).0
}
