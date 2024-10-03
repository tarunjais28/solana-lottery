use std::{fmt::Display, str::FromStr, sync::Arc};

use anchor_lang::AccountDeserialize;
use anyhow::{anyhow, Result};
use clap::{command, crate_description, crate_version, Arg, ArgAction, ColorChoice, Command};
use instruction::{deposit_instruction, withdraw_instruction};
use solana_clap_utils::input_validators::normalize_to_url_if_moniker;
use solana_client::rpc_client::RpcClient;

use solana_sdk::{
    commitment_config::CommitmentConfig,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{read_keypair_file, Signature},
    signer::{keypair::Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::{amount_to_ui_amount, ui_amount_to_amount};
use staking::UserBalance;
use url::Url;
use util::user_balance_address;

mod instruction;
mod util;

#[derive(Debug)]
pub enum NezCommand {
    Deposit,
    Withdraw,
    StakedBalance,
    WalletBalance,
}

impl Display for NezCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NezCommand::Deposit => write!(f, "deposit"),
            NezCommand::Withdraw => write!(f, "withdraw"),
            NezCommand::StakedBalance => write!(f, "staked-balance"),
            NezCommand::WalletBalance => write!(f, "wallet-balance"),
        }
    }
}

impl FromStr for NezCommand {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "deposit" => Ok(NezCommand::Deposit),
            "withdraw" => Ok(NezCommand::Withdraw),
            "staked-balance" => Ok(NezCommand::StakedBalance),
            "wallet-balance" => Ok(NezCommand::WalletBalance),
            _ => Err(anyhow!("Invalid command")),
        }
    }
}

struct NezContext {
    rpc_client: Arc<RpcClient>,
    program_id: Pubkey,
    token_mint: Pubkey,
    token_decimals: u8,
}

fn parse_commitment(commitment_level: &str) -> Result<CommitmentConfig> {
    match commitment_level {
        "processed" => Ok(CommitmentConfig::processed()),
        "confirmed" => Ok(CommitmentConfig::confirmed()),
        "finalized" => Ok(CommitmentConfig::finalized()),
        _ => Err(anyhow!("Invalid commitment level")),
    }
}

fn parse_json_rpc_url(url: &str) -> Result<Url> {
    let url = normalize_to_url_if_moniker(url);
    let url = Url::parse(&url)?;

    if url.has_host() {
        Ok(url)
    } else {
        Err(anyhow!("no host provided"))
    }
}

fn main() -> Result<()> {
    let app = command!()
        .color(ColorChoice::Always)
        .about(crate_description!())
        .version(crate_version!())
        .arg(
            Arg::new("json_rpc_url")
                .short('u')
                .long("url")
                .env("DEMO_SOLANA_RPC_URL")
                .action(ArgAction::Set)
                .value_name("URL_OR_MONIKER")
                .value_parser(parse_json_rpc_url)
                .global(true)
                .help(
                    "URL for Solana's JSON RPC or moniker (or their first letter): \
                [mainnet-beta, testnet, devnet, localhost]",
                ),
        )
        .arg(
            Arg::new("keypair")
                .short('k')
                .long("keypair")
                .action(ArgAction::Set)
                .value_name("KEYPAIR")
                .global(true)
                .help("Filepath or URL to a keypair"),
        )
        .arg(
            Arg::new("commitment")
                .long("commitment")
                .action(ArgAction::Set)
                .value_name("COMMITMENT_LEVEL")
                .value_parser(parse_commitment)
                .hide_possible_values(true)
                .global(true)
                .help("Return information at the selected commitment level [processed, confirmed, finalized]"),
        )
        .arg(
            Arg::new("token_mint")
                .short('t')
                .long("token-mint")
                .env("SOLANA_NEZ_MINT")
                .action(ArgAction::Set)
                .value_name("TOKEN_MINT")
                .value_parser(Pubkey::from_str)
                .global(true)
                .help("Token to deposit"),
        )
        .arg(
            Arg::new("program_id")
                .short('p')
                .long("program-id")
                .env("SOLANA_NEZ_PROGRAM_ID")
                .action(ArgAction::Set)
                .value_name("PROGRAM_ID")
                .value_parser(Pubkey::from_str)
                .global(true)
                .help("Program ID"),
        )
        .subcommand(
            Command::new(NezCommand::Deposit.to_string())
                .about("Deposit NEZ token into NEZ staking contract")
                .arg(
                    Arg::new("amount")
                        .action(ArgAction::Set)
                        .value_name("AMOUNT")
                        .value_parser(|s: &str| s.parse::<f64>().map_err(|e| e.to_string()))
                        .index(1)
                        .required(true)
                        .help("Amount to deposit"),
                ),
        )
        .subcommand(
            Command::new(NezCommand::Withdraw.to_string())
                .about("Withdraw NEZ token from NEZ staking contract")
                .arg(
                    Arg::new("amount")
                        .action(ArgAction::Set)
                        .value_name("AMOUNT")
                        .value_parser(|s: &str| s.parse::<f64>().map_err(|e| e.to_string()))
                        .index(1)
                        .required(true)
                        .help("Amount to withdraw"),
                ),
        )
        .subcommand(
            Command::new(NezCommand::StakedBalance.to_string())
                .about("Show staked user balance")
                .alias("b")
                .arg(
                    Arg::new("user")
                        .action(ArgAction::Set)
                        .value_name("USER_ADDRESS")
                        .value_parser(Pubkey::from_str)
                        .index(1)
                        .help(
                            "Address of the owner of the stake account. \
                            Defaults to the keypair address.",
                        ),
                ),
        )
        .subcommand(
            Command::new(NezCommand::WalletBalance.to_string())
                .about("Show user wallet balance")
                .alias("w")
                .arg(
                    Arg::new("user")
                        .action(ArgAction::Set)
                        .value_name("USER_ADDRESS")
                        .value_parser(Pubkey::from_str)
                        .index(1)
                        .help(
                            "Address of the owner of the stake account. \
                            Defaults to the keypair address.",
                        ),
                ),
        );
    let matches = app.get_matches();
    let json_rpc_url = matches
        .get_one::<Url>("json_rpc_url")
        .ok_or_else(|| anyhow!("No JSON RPC URL provided"))?;
    let commitment_config = matches
        .get_one::<CommitmentConfig>("commitment")
        .cloned()
        .unwrap_or(CommitmentConfig::finalized());
    let user_keypair = match matches.get_one::<String>("keypair") {
        Some(keypair_path) => Some(Arc::new(
            read_keypair_file(keypair_path).map_err(|e| anyhow!(format!("{e}")))?,
        )),
        None => None,
    };
    let token_mint = matches.get_one::<Pubkey>("token_mint").ok_or_else(|| {
        anyhow!("No token mint provided. Use --token-mint or set NEZ_TOKEN_MINT environment variable")
    })?;
    let program_id = matches.get_one::<Pubkey>("program_id").ok_or_else(|| {
        anyhow!("No program ID provided. Use --program-id or set NEZ_PROGRAM_ID environment variable")
    })?;

    let rpc_client = Arc::new(RpcClient::new_with_commitment(json_rpc_url, commitment_config));
    let token_decimals = spl_token::state::Mint::unpack(&rpc_client.get_account_data(token_mint)?)?.decimals;
    let context = NezContext {
        rpc_client,
        program_id: *program_id,
        token_mint: *token_mint,
        token_decimals,
    };

    if let Some((sub_command, sub_matches)) = matches.subcommand() {
        match NezCommand::from_str(sub_command)? {
            NezCommand::Deposit => {
                let user_keypair = user_keypair.ok_or_else(|| anyhow!("No keypair provided"))?;
                let ui_amount = sub_matches
                    .get_one::<f64>("amount")
                    .ok_or_else(|| anyhow!("No amount provided"))?;
                let amount = ui_amount_to_amount(*ui_amount, context.token_decimals);
                let signature = deposit(&context, &user_keypair, amount)?;
                println!("Deposited {ui_amount} NEZ. Signature: {signature}");
                let balance = staked_balance(&context, &user_keypair.pubkey())?;
                println!("Staked balance: {balance}");
            }
            NezCommand::Withdraw => {
                let user_keypair = user_keypair.ok_or_else(|| anyhow!("No keypair provided"))?;
                let ui_amount = sub_matches
                    .get_one::<f64>("amount")
                    .ok_or_else(|| anyhow!("No amount provided"))?;
                let amount = ui_amount_to_amount(*ui_amount, context.token_decimals);
                let signature = withdraw(&context, &user_keypair, amount)?;
                println!("Withdrew {ui_amount} NEZ. Signature: {signature}");
                let balance = staked_balance(&context, &user_keypair.pubkey())?;
                println!("Staked balance: {balance}");
            }
            NezCommand::StakedBalance => {
                let user_pubkey = match sub_matches.get_one::<Pubkey>("user") {
                    Some(user_pubkey) => *user_pubkey,
                    None => user_keypair.ok_or_else(|| anyhow!("No keypair provided"))?.pubkey(),
                };
                let balance = staked_balance(&context, &user_pubkey)?;
                println!("Staked balance: {balance}");
            }
            NezCommand::WalletBalance => {
                let user_pubkey = match sub_matches.get_one::<Pubkey>("user") {
                    Some(user_pubkey) => *user_pubkey,
                    None => user_keypair.ok_or_else(|| anyhow!("No keypair provided"))?.pubkey(),
                };
                let balance = wallet_balance(&context, &user_pubkey)?;
                println!("Wallet balance: {balance}");
            }
        }
    }
    Ok(())
}

fn staked_balance(context: &NezContext, user_pubkey: &Pubkey) -> Result<f64> {
    let user_balance_pubkey = user_balance_address(&context.program_id, user_pubkey, &context.token_mint);
    let user_balance_data = context.rpc_client.get_account_data(&user_balance_pubkey)?;
    let balance = UserBalance::try_deserialize(&mut user_balance_data.as_ref())?;
    let amount = balance.amount;
    let ui_amount = amount_to_ui_amount(amount, context.token_decimals);
    Ok(ui_amount)
}

fn wallet_balance(context: &NezContext, user_pubkey: &Pubkey) -> Result<f64> {
    let user_ata = get_associated_token_address(user_pubkey, &context.token_mint);
    let amount = spl_token::state::Account::unpack(&context.rpc_client.get_account_data(&user_ata)?)?.amount;
    let ui_amount = amount_to_ui_amount(amount, context.token_decimals);
    Ok(ui_amount)
}

fn deposit(context: &NezContext, user_keypair: &Keypair, amount: u64) -> Result<Signature> {
    let user_pubkey = user_keypair.pubkey();
    let deposit_instruction = deposit_instruction(context.program_id, user_pubkey, context.token_mint, amount);
    let transaction = Transaction::new_signed_with_payer(
        &[deposit_instruction],
        Some(&user_pubkey),
        &[user_keypair],
        context.rpc_client.get_latest_blockhash()?,
    );
    let signature = context
        .rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)?;
    Ok(signature)
}

fn withdraw(context: &NezContext, user_keypair: &Keypair, amount: u64) -> Result<Signature> {
    let user_pubkey = user_keypair.pubkey();
    let withdraw_instruction = withdraw_instruction(context.program_id, user_pubkey, context.token_mint, amount);
    let transaction = Transaction::new_signed_with_payer(
        &[withdraw_instruction],
        Some(&user_pubkey),
        &[user_keypair],
        context.rpc_client.get_latest_blockhash()?,
    );
    let signature = context
        .rpc_client
        .send_and_confirm_transaction_with_spinner(&transaction)?;
    Ok(signature)
}
