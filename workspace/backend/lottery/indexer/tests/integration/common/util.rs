use anyhow::Result;
use indexer::indexer::util::{send_and_confirm_transaction, SolanaProgramContext};
use nezha_staking::instruction;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{native_token::LAMPORTS_PER_SOL, pubkey::Pubkey, signature::Keypair, signer::Signer};
use spl_associated_token_account::{get_associated_token_address, instruction::create_associated_token_account};

pub async fn airdrop_usdc(context: &SolanaProgramContext, wallet: &Pubkey, amount: u64) -> Result<()> {
    let admin_pubkey = context.admin_keypair.pubkey();
    let ata_pubkey = get_associated_token_address(wallet, &context.usdc_mint_pubkey);
    if let Err(_) = context.rpc_client.get_account(&ata_pubkey).await {
        let instruction =
            create_associated_token_account(&admin_pubkey, wallet, &context.usdc_mint_pubkey, &spl_token::id());
        send_and_confirm_transaction(&context.rpc_client, instruction, &context.admin_keypair, &admin_pubkey).await?;
    }
    let instruction = spl_token::instruction::mint_to(
        &spl_token::id(),
        &context.usdc_mint_pubkey,
        &ata_pubkey,
        &admin_pubkey,
        &[&admin_pubkey],
        amount,
    )?;
    send_and_confirm_transaction(&context.rpc_client, instruction, &context.admin_keypair, &admin_pubkey).await?;
    Ok(())
}

pub async fn airdrop_sol(rpc_client: &RpcClient, wallet: &Pubkey) -> Result<()> {
    let signature = rpc_client.request_airdrop(&wallet, LAMPORTS_PER_SOL).await?;
    rpc_client.confirm_transaction(&signature).await?;
    Ok(())
}

pub async fn attempt_deposit(context: &SolanaProgramContext, owner_keypair: &Keypair, amount: u64) -> Result<()> {
    let owner_pubkey = owner_keypair.pubkey();

    let owner_usdc_token_pubkey = get_associated_token_address(&owner_pubkey, &context.usdc_mint_pubkey);
    let instruction = instruction::request_stake_update(
        &context.staking_program_id,
        &owner_pubkey,
        &owner_usdc_token_pubkey,
        amount as _,
    );
    send_and_confirm_transaction(&context.rpc_client, instruction, owner_keypair, &owner_pubkey).await?;
    Ok(())
}
