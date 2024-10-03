use solana_program::pubkey::Pubkey;

pub struct Token {
    pub token_program_id: Pubkey,
    pub token_mint: Pubkey,
}

impl Token {
    pub fn balance_of<'a>() {}

    pub fn transfer_to() {}

    pub fn mint() {}
}
