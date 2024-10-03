use std::sync::Arc;

use anyhow::Result;
use nezha_staking::instruction;
use service::solana::{
    rpc::{SolanaRpc, SolanaRpcExt, SolanaRpcTest},
    solana_impl::SolanaImpl,
    VrfConfiguration,
};
use solana_program::{
    instruction::Instruction, native_token::LAMPORTS_PER_SOL, program_pack::Pack, pubkey::Pubkey, system_instruction,
};
use solana_sdk::{
    signature::{Keypair, Signature},
    signer::Signer,
};
use spl_associated_token_account::{get_associated_token_address, instruction::create_associated_token_account};

pub mod util;

pub struct SolanaContext {
    pub solana: SolanaImpl,
    pub usdc_mint: Arc<Keypair>,
    pub nez_mint: Arc<Keypair>,
    pub admin_keypair: Arc<Keypair>,
    pub investor_keypair: Arc<Keypair>,
    pub user_keypair: Arc<Keypair>,
}

fn random_keypair() -> Arc<Keypair> {
    Arc::new(Keypair::new())
}

pub async fn setup_solana() -> SolanaContext {
    let program_id = Pubkey::new_unique();
    let usdc_mint = random_keypair();
    let nez_mint = random_keypair();
    let admin_keypair = random_keypair();
    let super_admin_keypair = random_keypair();
    let investor_keypair = random_keypair();
    let user_keypair = random_keypair();
    let switchboard_program_id = random_keypair();
    let nezha_vrf = Pubkey::new_unique();

    let rpc = SolanaRpcTest::new(program_id, nezha_vrf).await;

    for actor in [&super_admin_keypair, &admin_keypair, &investor_keypair, &user_keypair] {
        rpc.mint_sols(actor.pubkey(), 100 * LAMPORTS_PER_SOL).await;
    }

    create_mint(
        &rpc,
        &admin_keypair,
        &usdc_mint,
        &admin_keypair.pubkey(),
        Some(&admin_keypair.pubkey()),
    )
    .await
    .unwrap();

    create_mint(
        &rpc,
        &admin_keypair,
        &nez_mint,
        &admin_keypair.pubkey(),
        Some(&admin_keypair.pubkey()),
    )
    .await
    .unwrap();

    for actor in [&admin_keypair, &investor_keypair, &user_keypair] {
        create_token_account(&rpc, &admin_keypair, &actor.pubkey(), &usdc_mint.pubkey())
            .await
            .unwrap();
        mint_tokens(
            &rpc,
            &actor.pubkey(),
            &usdc_mint.pubkey(),
            &admin_keypair,
            1_000_000_000_000,
        )
        .await
        .unwrap();
    }

    init(
        &rpc,
        &program_id,
        &super_admin_keypair,
        &admin_keypair,
        &investor_keypair,
        &usdc_mint.pubkey(),
        &nezha_vrf,
    )
    .await
    .unwrap();

    let solana = SolanaImpl {
        rpc_client: Arc::new(rpc),
        program_id,
        usdc_mint: usdc_mint.pubkey(),
        nez_mint: nez_mint.pubkey(),
        admin_keypair: admin_keypair.clone(),
        investor_keypair: investor_keypair.clone(),
        vrf_configuration: VrfConfiguration::Fake { program_id: nezha_vrf },
    };

    SolanaContext {
        solana,
        usdc_mint,
        nez_mint,
        admin_keypair,
        investor_keypair,
        user_keypair,
    }
}

pub async fn send_and_confirm_tx(rpc: &dyn SolanaRpc, signer: &Keypair, ix: Instruction) -> Result<Signature> {
    let sig = rpc.send_and_confirm_transaction(signer, &[ix]).await?;
    Ok(sig)
}

// Solana Setup Actions

pub async fn create_mint(
    rpc: &SolanaRpcTest,
    payer: &Keypair,
    mint: &Keypair,
    manager: &Pubkey,
    freeze_authority: Option<&Pubkey>,
) -> Result<Signature> {
    let rent = rpc.get_rent().await;

    let sig = rpc
        ._send_and_confirm_transaction(
            &[payer, mint],
            None,
            &[
                system_instruction::create_account(
                    &payer.pubkey(),
                    &mint.pubkey(),
                    rent.minimum_balance(spl_token::state::Mint::LEN),
                    spl_token::state::Mint::LEN as u64,
                    &spl_token::id(),
                ),
                spl_token::instruction::initialize_mint(
                    &spl_token::id(),
                    &mint.pubkey(),
                    &manager,
                    freeze_authority,
                    6,
                )?,
            ],
        )
        .await?;
    Ok(sig)
}

pub async fn init(
    rpc: &dyn SolanaRpc,
    program_id: &Pubkey,
    super_admin_keypair: &Keypair,
    admin_keypair: &Keypair,
    investor_keypair: &Keypair,
    usdc_mint: &Pubkey,
    vrf_program: &Pubkey,
) -> Result<Signature> {
    let sig = rpc
        .send_and_confirm_transaction(
            &super_admin_keypair,
            &[instruction::init(
                program_id,
                &super_admin_keypair.pubkey(),
                &admin_keypair.pubkey(),
                &investor_keypair.pubkey(),
                usdc_mint,
                vrf_program,
            )],
        )
        .await?;
    Ok(sig)
}

pub async fn create_token_account(rpc: &dyn SolanaRpc, payer: &Keypair, owner: &Pubkey, mint: &Pubkey) -> Result<()> {
    let instructions = &[create_associated_token_account(
        &payer.pubkey(),
        &owner,
        &mint,
        &spl_token::id(),
    )];
    rpc.send_and_confirm_transaction(payer, instructions).await?;
    Ok(())
}

pub async fn mint_tokens(
    rpc: &dyn SolanaRpc,
    account: &Pubkey,
    mint: &Pubkey,
    mint_authority: &Keypair,
    amount: u64,
) -> Result<()> {
    let ata = get_associated_token_address(account, mint);
    rpc.send_and_confirm_transaction(
        mint_authority,
        &[
            spl_token::instruction::mint_to(&spl_token::id(), &mint, &ata, &mint_authority.pubkey(), &[], amount)
                .unwrap(),
        ],
    )
    .await?;
    Ok(())
}
