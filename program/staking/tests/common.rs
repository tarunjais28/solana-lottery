use solana_program::native_token::LAMPORTS_PER_SOL;
use solana_program_test::ProgramTest;
use solana_sdk::{signature::Keypair, signer::Signer};

pub fn setup_keypair(program_test: &mut ProgramTest, sol_balance: u64) -> Keypair {
    let kp = Keypair::new();

    program_test.add_account(
        kp.pubkey(),
        solana_sdk::account::Account::new(sol_balance * LAMPORTS_PER_SOL, 0, &solana_sdk::system_program::ID),
    );

    kp
}
