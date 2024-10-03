use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

#[derive(Debug)]
pub struct Accounts {
    pub program_id: Pubkey,
    pub super_admin: Keypair,
    pub admin: Keypair,
    pub random1: Keypair,
    pub random2: Keypair,
    pub switchboard_queue: Pubkey,
    pub switchboard_queue_authority: Pubkey,
    pub switchboard_queue_mint: Pubkey,
    pub switchboard_queue_data_buffer: Pubkey,
    pub nezha_staking_program_id: Pubkey,
}

impl Accounts {
    pub fn new() -> Self {
        Accounts {
            program_id: Pubkey::new_unique(),
            super_admin: Keypair::new(),
            admin: Keypair::new(),
            random1: Keypair::new(),
            random2: Keypair::new(),
            switchboard_queue: Pubkey::new_unique(),
            switchboard_queue_authority: Pubkey::new_unique(),
            switchboard_queue_mint: spl_token::native_mint::ID,
            switchboard_queue_data_buffer: Pubkey::new_unique(),
            nezha_staking_program_id: Pubkey::new_unique(),
        }
    }
}
