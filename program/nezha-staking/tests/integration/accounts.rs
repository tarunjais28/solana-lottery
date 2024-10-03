use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

#[derive(Debug)]
pub struct Accounts {
    pub program_id: Pubkey,
    pub super_admin: Keypair,
    pub admin: Keypair,
    pub owner: Keypair,
    pub investor: Keypair,
    pub usdc_mint: Keypair,
    pub random1: Keypair,
    pub random2: Keypair,
    pub nezha_vrf_program_id: Pubkey,
}

impl Accounts {
    pub fn new() -> Self {
        Accounts {
            program_id: Pubkey::new_unique(),
            super_admin: Keypair::new(),
            admin: Keypair::new(),
            owner: Keypair::new(),
            investor: Keypair::new(),
            usdc_mint: Keypair::new(),
            random1: Keypair::new(),
            random2: Keypair::new(),
            nezha_vrf_program_id: Pubkey::new_unique(),
        }
    }
}
