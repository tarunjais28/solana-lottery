use borsh::BorshDeserialize;
use chrono::{Duration, Utc};
use nezha_staking_lib::{
    accounts as ac,
    error::StakingError,
    fixed_point::FPUSDC,
    francium::constants as fr_consts,
    instruction::{self, CreateEpochWinnersMetaArgs, TierWinnersMetaInput, WinnerInput, WithdrawVault},
    state::*,
};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    borsh0_10::try_from_slice_unchecked,
    compute_budget,
    instruction::{Instruction, InstructionError},
    native_token::{lamports_to_sol, sol_to_lamports},
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signature, Signer},
    transaction::{Transaction, TransactionError},
};
use spl_associated_token_account::{get_associated_token_address, instruction::create_associated_token_account};

use std::{
    env::{self, args},
    ops::Add,
    path::PathBuf,
    thread,
};
use std::{fs, str::FromStr};

fn main() {
    let args: Vec<String> = args().collect();

    let rpc_url = env::var("DEMO_SOLANA_RPC_URL").unwrap();
    let staking_program_id = env::var("SOLANA_STAKING_PROGRAM_ID")
        .map(|s| Pubkey::from_str(&s).unwrap())
        .unwrap();
    let nezha_vrf_program_id = env::var("SOLANA_VRF_PROGRAM_ID")
        .map(|s| Pubkey::from_str(&s).unwrap())
        .unwrap();
    let usdc_mint_pubkey = env::var("DEMO_SOLANA_USDC_MINT")
        .map(|s| Pubkey::from_str(&s).unwrap())
        .unwrap();

    println!("RPC: {}", rpc_url);
    let rpc = RpcClient::new(rpc_url.to_string());

    let super_admin_kp = read_keypair_file(key_path("keys/super_admin.json")).expect("failed to read keypair file");
    let super_admin_pubkey = super_admin_kp.pubkey();

    let admin_kp = read_keypair_file(key_path("keys/admin.json")).expect("failed to read keypair file");
    let admin_pubkey = admin_kp.pubkey();

    let investor_kp = read_keypair_file(key_path("keys/investor.json")).expect("failed to read keypair file");
    let investor_pubkey = investor_kp.pubkey();

    let user_kp = read_keypair_file(key_path("keys/user.json")).expect("failed to read keypair file");
    let user_pubkey = user_kp.pubkey();

    let latest_epoch_pubkey = ac::latest_epoch(&staking_program_id).pubkey;

    let command = &args[1];
    match &command[..] {
        "replenish-sols" => {
            let mut txns = Vec::new();
            txns.extend(replenish_account(&rpc, super_admin_pubkey, 1.0));
            txns.extend(replenish_account(&rpc, admin_pubkey, 1.0));
            txns.extend(replenish_account(&rpc, user_pubkey, 1.0));
            txns.extend(replenish_account(&rpc, investor_pubkey, 1.0));

            for txn in txns {
                confirm_tx(&rpc, txn);
            }
        }
        "init" => {
            let ix = instruction::init(
                &staking_program_id,
                &super_admin_pubkey,
                &admin_pubkey,
                &investor_pubkey,
                &usdc_mint_pubkey,
                &nezha_vrf_program_id,
            );
            send_tx(&rpc, &super_admin_kp, ix);
        }
        "show-latest-epoch" => {
            let latest_data = rpc.get_account_data(&latest_epoch_pubkey).unwrap();
            let latest = LatestEpoch::try_from_slice(&latest_data).unwrap();
            println!("{:#?}", &latest);
            show_epoch(&rpc, &latest.epoch);
        }
        "show-epoch" => {
            let epoch_index = args[2].parse::<u64>().unwrap();
            let epoch_pubkey = ac::epoch(&staking_program_id, epoch_index).pubkey;
            show_epoch(&rpc, &epoch_pubkey);
        }
        "show-stake" => {
            let stake_pubkey = ac::stake(&staking_program_id, &user_pubkey).pubkey;
            let stake_data = rpc.get_account_data(&stake_pubkey).unwrap();
            let stake = Stake::try_from_slice(&stake_data).unwrap();
            println!("{:#?}", &stake);
        }
        "show-stake-update-request" => {
            let stake_update_request_pubkey = ac::stake_update_request(&staking_program_id, &user_pubkey).pubkey;
            let stake_update_request_data = rpc.get_account_data(&stake_update_request_pubkey).unwrap();
            let stake_update_request = StakeUpdateRequest::try_from_slice(&stake_update_request_data).unwrap();
            println!("{:#?}", &stake_update_request);
        }
        "show-latest-epoch-winners" => {
            let latest_data = rpc.get_account_data(&latest_epoch_pubkey).unwrap();
            let latest = LatestEpoch::try_from_slice(&latest_data).unwrap();

            show_epoch_winners(&rpc, &staking_program_id, latest.index);
        }
        "show-epoch-winners" => {
            let epoch_index = args[2].parse::<u64>().expect("index should be a number");
            show_epoch_winners(&rpc, &staking_program_id, epoch_index);
        }
        "create-epoch" => {
            let latest_data = rpc.get_account_data(&latest_epoch_pubkey).unwrap();
            let latest = LatestEpoch::try_from_slice(&latest_data).unwrap();
            let index = latest.index + 1;

            let yield_split_cfg = YieldSplitCfg {
                insurance: InsuranceCfg {
                    premium: "3.0".parse().unwrap(),
                    probability: "0.000_000_000_181_576_594".parse().unwrap(),
                },
                jackpot: "100_000".parse().unwrap(),
                treasury_ratio: "0.5".parse().unwrap(),
                tier2_prize_share: 1,
                tier3_prize_share: 1,
            };

            let expected_end_date = Utc::now().add(Duration::weeks(1));

            let ix = instruction::create_epoch(
                &staking_program_id,
                &admin_pubkey,
                index,
                expected_end_date.timestamp(),
                yield_split_cfg,
            );
            send_tx(&rpc, &admin_kp, ix);
        }
        "request-stake-update" => {
            let amount = if args[2].starts_with("-") {
                let amount: FPUSDC = args[2].strip_prefix("-").unwrap().parse().unwrap();
                amount.as_usdc_i64() * -1
            } else {
                let amount: FPUSDC = args[2].parse().unwrap();
                amount.as_usdc_i64()
            };
            let user_usdc_token = get_or_create_ata(&rpc, &admin_kp, &user_pubkey, &usdc_mint_pubkey);

            if amount > 0 {
                let ix = spl_token::instruction::mint_to(
                    &spl_token::id(),
                    &usdc_mint_pubkey,
                    &user_usdc_token,
                    &admin_pubkey,
                    &[&admin_pubkey],
                    amount as _,
                )
                .unwrap();
                println!("Minting USDC");
                send_tx(&rpc, &admin_kp, ix);
            }

            let ix = instruction::request_stake_update(&staking_program_id, &user_pubkey, &user_usdc_token, amount);
            println!("Sending DepositAttempt");
            send_tx(&rpc, &user_kp, ix);
        }
        "approve-stake-update" => {
            let stake_update_request = StakeUpdateRequest::try_from_slice(
                &rpc.get_account_data(&ac::stake_update_request(&staking_program_id, &user_pubkey).pubkey)
                    .unwrap(),
            )
            .unwrap();

            let ix = instruction::approve_stake_update(
                &staking_program_id,
                &admin_pubkey,
                &user_pubkey,
                stake_update_request.amount,
            );
            send_tx(&rpc, &admin_kp, ix);
        }
        "complete-stake-update" => {
            let user_usdc_token = get_or_create_ata(&rpc, &admin_kp, &user_pubkey, &usdc_mint_pubkey);
            let ix =
                instruction::complete_stake_update(&staking_program_id, &admin_pubkey, &user_pubkey, &user_usdc_token);
            send_tx(&rpc, &admin_kp, ix);
        }
        "cancel-stake-update" => {
            let amount = if args[2].starts_with("-") {
                let amount: FPUSDC = args[2].strip_prefix("-").unwrap().parse().unwrap();
                amount.as_usdc_i64() * -1
            } else {
                let amount: FPUSDC = args[2].parse().unwrap();
                amount.as_usdc_i64()
            };

            let user_usdc_token = get_or_create_ata(&rpc, &admin_kp, &user_pubkey, &usdc_mint_pubkey);
            let ix =
                instruction::cancel_stake_update(&staking_program_id, None, &user_pubkey, &user_usdc_token, amount);
            send_tx(&rpc, &user_kp, ix);
        }
        "claim-prize" => {
            let epoch_index = args[2].parse::<u64>().expect("index should be a number");
            let tier = args[3].parse::<u8>().expect("tier should be a number");
            let page = args
                .get(4)
                .unwrap_or(&String::from("0"))
                .parse()
                .expect("page should be a number");

            let winner_index = args
                .get(5)
                .unwrap_or(&String::from("0"))
                .parse()
                .expect("winner index should be a number");

            let ix =
                instruction::claim_winning(&staking_program_id, &user_pubkey, epoch_index, page, winner_index, tier);
            println!("Sending ClaimWinning");
            send_tx(&rpc, &user_kp, ix);
        }
        "yield-withdraw" => {
            let latest_data = rpc.get_account_data(&latest_epoch_pubkey).unwrap();
            let latest = LatestEpoch::try_from_slice(&latest_data).unwrap();
            let num_tickets = 1;

            let investor_usdc_token = get_or_create_ata(&rpc, &admin_kp, &investor_pubkey, &usdc_mint_pubkey);

            let ix = instruction::yield_withdraw_by_investor(
                &staking_program_id,
                &admin_pubkey,
                &investor_usdc_token,
                latest.index,
                TicketsInfo {
                    num_tickets,
                    tickets_url: String::from("TODO"),
                    tickets_hash: Vec::new(),
                    tickets_version: 0,
                },
            );

            send_tx(&rpc, &admin_kp, ix);
        }
        "yield-deposit" => {
            let return_amount = args[2].parse::<FPUSDC>().unwrap();
            let investor_usdc_token = get_or_create_ata(&rpc, &admin_kp, &investor_pubkey, &usdc_mint_pubkey);

            mint_to(
                &rpc,
                &admin_kp,
                &usdc_mint_pubkey,
                &investor_usdc_token,
                return_amount.as_usdc(),
            );

            let latest_data = rpc.get_account_data(&latest_epoch_pubkey).unwrap();
            let latest = LatestEpoch::try_from_slice(&latest_data).unwrap();

            let ix = instruction::yield_deposit_by_investor(
                &staking_program_id,
                &investor_pubkey,
                &investor_usdc_token,
                latest.index,
                return_amount.as_usdc(),
            );

            send_tx(&rpc, &investor_kp, ix)
        }
        "publish-winners" => {
            let latest_data = rpc.get_account_data(&latest_epoch_pubkey).unwrap();
            let latest = LatestEpoch::try_from_slice(&latest_data).unwrap();
            let epoch_index = latest.index;
            let epoch: Epoch = try_from_slice_unchecked(
                &rpc.get_account_data(&ac::epoch(&staking_program_id, epoch_index).pubkey)
                    .unwrap(),
            )
            .unwrap();

            let mut winners_input = (0..10)
                .map(|i| WinnerInput {
                    index: i,
                    address: Pubkey::new_unique(),
                    tier: 1,
                    num_winning_tickets: 1,
                })
                .collect::<Vec<_>>();
            winners_input.extend((10..20).map(|i| WinnerInput {
                index: i,
                address: Pubkey::new_unique(),
                tier: 2,
                num_winning_tickets: 1,
            }));
            winners_input.extend((20..27).map(|i| WinnerInput {
                index: i,
                address: Pubkey::new_unique(),
                tier: 3,
                num_winning_tickets: 1,
            }));
            let meta_args = CreateEpochWinnersMetaArgs {
                tier1_meta: TierWinnersMetaInput {
                    total_num_winners: 10,
                    total_num_winning_tickets: 10,
                },
                tier2_meta: TierWinnersMetaInput {
                    total_num_winners: 10,
                    total_num_winning_tickets: 10,
                },
                tier3_meta: TierWinnersMetaInput {
                    total_num_winners: 7,
                    total_num_winning_tickets: 7,
                },
            };
            let create_winners_meta_ix = instruction::create_epoch_winners_meta(
                &staking_program_id,
                &admin_pubkey,
                epoch_index,
                meta_args,
                &nezha_vrf_program_id,
            );
            send_tx(&rpc, &admin_kp, create_winners_meta_ix);

            if epoch.draw_enabled.unwrap() && !winners_input.is_empty() {
                for (page_index, chunk) in winners_input.chunks(MAX_NUM_WINNERS_PER_PAGE).enumerate() {
                    let cuix = compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(1_400_000);
                    let publish_winners_ix = instruction::publish_winners(
                        &staking_program_id,
                        &admin_pubkey,
                        epoch_index,
                        page_index as u32,
                        chunk.to_vec(),
                        &nezha_vrf_program_id,
                    );
                    send_txs(&rpc, &admin_kp, &[cuix, publish_winners_ix]);
                }
            }
        }
        "faucet" => {
            let wallet = Pubkey::from_str(&args[2]).expect("unable to parse wallet addr");
            let amount: FPUSDC = args[3].parse().expect("invalid amount");

            rpc.request_airdrop(&wallet, sol_to_lamports(2.0))
                .expect("failed to request airdrop");

            mint_to(&rpc, &admin_kp, &usdc_mint_pubkey, &wallet, amount.as_usdc());
        }

        "fund-jackpot" => {
            let index = args[2].parse::<u64>().expect("index should be a number");

            let epoch_winners_meta = ac::epoch_winners_meta(&staking_program_id, index).pubkey;
            let admin_ata = get_or_create_ata(&rpc, &admin_kp, &admin_pubkey, &usdc_mint_pubkey);

            let meta = try_from_slice_unchecked::<EpochWinnersMeta>(
                &rpc.get_account_data(&epoch_winners_meta)
                    .expect("unable to get epoch winners meta data"),
            )
            .expect("unable to parse EpochWinnersMeta");

            mint_to(
                &rpc,
                &admin_kp,
                &usdc_mint_pubkey,
                &admin_pubkey,
                meta.tier1_meta.total_prize.as_usdc(),
            );

            let ix = instruction::fund_jackpot(&staking_program_id, &admin_pubkey, &admin_ata, index);

            send_tx(&rpc, &admin_kp, ix);
        }
        "francium-init" => {
            let ix = instruction::francium_init(&staking_program_id, &admin_pubkey, &fr_consts::get_mints());
            send_tx(&rpc, &admin_kp, ix);
        }
        "francium-user-deposit" => {
            let user_kp_file = &args[2];

            let user_kp = read_keypair_file(user_kp_file).expect("failed to read user keypair file");
            let user_pubkey = user_kp.pubkey();
            let user_usdc_token = get_associated_token_address(&user_pubkey, &usdc_mint_pubkey);

            let amount: FPUSDC = args[3].parse().unwrap();

            println!("Request Stake Update");
            let ix = instruction::request_stake_update(
                &staking_program_id,
                &user_pubkey,
                &user_usdc_token,
                amount.as_usdc_i64(),
            );
            send_tx(&rpc, &user_kp, ix);

            println!("Approve Stake Update");
            let ix = instruction::approve_stake_update(
                &staking_program_id,
                &admin_pubkey,
                &user_pubkey,
                amount.as_usdc_i64(),
            );
            send_tx(&rpc, &admin_kp, ix);

            println!("Complete Stake Update");
            let ix =
                instruction::complete_stake_update(&staking_program_id, &admin_pubkey, &user_pubkey, &user_usdc_token);
            send_tx(&rpc, &admin_kp, ix);
        }
        "francium-invest" => {
            let latest_data = rpc.get_account_data(&latest_epoch_pubkey).unwrap();
            let latest = LatestEpoch::try_from_slice(&latest_data).unwrap();

            let num_tickets = 1;

            let ix = instruction::francium_invest(
                &staking_program_id,
                &admin_pubkey,
                latest.index,
                TicketsInfo {
                    num_tickets,
                    tickets_url: String::from("TODO"),
                    tickets_hash: Vec::new(),
                    tickets_version: 0,
                },
                &fr_consts::get_mints(),
            );
            send_tx(&rpc, &admin_kp, ix);
        }
        "francium-withdraw" => {
            let latest_data = rpc.get_account_data(&latest_epoch_pubkey).unwrap();
            let latest = LatestEpoch::try_from_slice(&latest_data).unwrap();

            let cuix = compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(300000);
            let ix = instruction::francium_withdraw(
                &staking_program_id,
                &admin_pubkey,
                latest.index,
                &fr_consts::get_mints(),
            );

            send_txs(&rpc, &admin_kp, &[cuix, ix]);
        }
        "withdraw-treasury" => {
            withdraw_vault(
                &rpc,
                &staking_program_id,
                &admin_kp,
                &usdc_mint_pubkey,
                WithdrawVault::Treasury,
            );
        }
        "withdraw-insurance" => {
            withdraw_vault(
                &rpc,
                &staking_program_id,
                &admin_kp,
                &usdc_mint_pubkey,
                WithdrawVault::Insurance,
            );
        }
        _ => {
            eprintln!("error: invalid command");
        }
    }
}

fn withdraw_vault(rpc: &RpcClient, program_id: &Pubkey, admin_kp: &Keypair, usdc_mint: &Pubkey, vault: WithdrawVault) {
    let vault_pubkey = &vault.get_pda(program_id).pubkey;
    let balance = get_usdc_balance(rpc, &vault_pubkey);

    let admin_pubkey = admin_kp.pubkey();
    let admin_usdc = get_or_create_ata(&rpc, &admin_kp, &admin_pubkey, &usdc_mint);

    println!("Vault balance ({:#?}) {}", vault, balance);
    println!("Admin balance: {}", get_usdc_balance(rpc, &admin_usdc));

    let ix = instruction::withdraw_vault(&program_id, &admin_pubkey, vault, &admin_usdc, balance.as_usdc());
    send_txs(&rpc, &admin_kp, &[ix]);

    println!(
        "Post Vault balance ({:#?}) {}",
        vault,
        get_usdc_balance(rpc, &vault_pubkey)
    );
    println!("Post Admin balance: {}", get_usdc_balance(rpc, &admin_usdc));
}

fn get_usdc_balance(rpc: &RpcClient, usdc_account: &Pubkey) -> FPUSDC {
    let account_data = rpc.get_account_data(usdc_account).unwrap();
    let account = spl_token::state::Account::unpack(&account_data).unwrap();
    let balance = account.amount;
    FPUSDC::from_usdc(balance)
}

fn key_path(path: &str) -> PathBuf {
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path);
    fs::canonicalize(&path).expect(&format!("unable to key: {}", path).to_string())
}

fn get_or_create_ata(rpc: &RpcClient, admin_kp: &Keypair, wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
    let ata = get_associated_token_address(&wallet, &mint);

    match rpc.get_token_account(&ata) {
        Ok(_) => {}
        Err(_) => {
            println!("Creating ATA");
            let ix = create_associated_token_account(&admin_kp.pubkey(), &wallet, &mint, &spl_token::id());
            send_tx(&rpc, &admin_kp, ix);
        }
    };

    ata
}

fn mint_to(rpc: &RpcClient, admin_kp: &Keypair, mint: &Pubkey, target: &Pubkey, amount: u64) {
    let token = get_or_create_ata(rpc, admin_kp, target, mint);

    let ix = spl_token::instruction::mint_to(
        &spl_token::id(),
        &mint,
        &token,
        &admin_kp.pubkey(),
        &[&admin_kp.pubkey()],
        amount,
    )
    .unwrap();
    send_tx(&rpc, &admin_kp, ix);
}

fn replenish_account(rpc: &RpcClient, pubkey: Pubkey, amount: f64) -> Option<Signature> {
    let balance = rpc.get_balance(&pubkey).expect("unable to get balance");
    if lamports_to_sol(balance) < amount {
        let sig = rpc
            .request_airdrop(&pubkey, sol_to_lamports(amount))
            .expect("unable to request airdrop");
        Some(sig)
    } else {
        None
    }
}

fn send_tx(rpc: &RpcClient, signer_kp: &Keypair, ix: Instruction) {
    send_txs(rpc, signer_kp, &[ix])
}

fn send_txs(rpc: &RpcClient, signer_kp: &Keypair, ixns: &[Instruction]) {
    let hash = rpc.get_latest_blockhash().unwrap();
    let mut tx = Transaction::new_with_payer(ixns, Some(&signer_kp.pubkey()));
    tx.sign(&[signer_kp], hash);
    match rpc.simulate_transaction(&tx) {
        Ok(sim_result) => {
            println!("Program logs: {:#?}", sim_result.value.logs.unwrap_or_default());
            if let Some(err) = sim_result.value.err {
                if let Some(staking_error) = decode_staking_error(&err) {
                    if let StakingError::InvalidEpochStatus(_) = staking_error {
                        println!("Error (ignoring): {}", staking_error);
                        return;
                    } else {
                        println!("Error: {}", staking_error);
                    }
                } else {
                    println!("Error: {}", err);
                }
                panic!("Simulation failed");
            }
        }
        Err(err) => {
            println!("Failed to simulate txn");
            if let Some(staking_error) = err.get_transaction_error().and_then(|err| decode_staking_error(&err)) {
                println!("Error: {}", staking_error);
            } else {
                println!("Error: {}", err);
            }
            panic!("Txn simulation failed");
        }
    }
    let tx_id = match rpc.send_transaction(&tx) {
        Ok(tx_id) => tx_id,
        Err(err) => {
            println!("Failed to execute txn");
            if let Some(staking_error) = err.get_transaction_error().and_then(|err| decode_staking_error(&err)) {
                println!("Error: {}", staking_error);
            } else {
                println!("Error: {}", err);
            }
            panic!("Txn failed");
        }
    };
    confirm_tx(rpc, tx_id);
}

fn confirm_tx(rpc: &RpcClient, tx_id: Signature) {
    println!("sending {}", tx_id);
    loop {
        let confirmed = rpc.confirm_transaction(&tx_id).unwrap();
        if confirmed {
            break;
        }
        thread::sleep(std::time::Duration::from_millis(2000));
    }
    println!();
}

fn show_epoch(rpc: &RpcClient, epoch_pubkey: &Pubkey) {
    let epoch_data = rpc.get_account_data(&epoch_pubkey).unwrap();
    let epoch: Result<Epoch, _> = try_from_slice_unchecked(&epoch_data);

    match epoch {
        Ok(epoch) => println!("{epoch:#?}"),
        Err(_) => println!("epoch not found"),
    };
}

fn show_epoch_winners(rpc: &RpcClient, staking_program_id: &Pubkey, epoch_index: u64) {
    let epoch_winners_meta_pubkey = ac::epoch_winners_meta(&staking_program_id, epoch_index).pubkey;
    let epoch_winners_meta: EpochWinnersMeta =
        try_from_slice_unchecked(&rpc.get_account_data(&epoch_winners_meta_pubkey).unwrap()).unwrap();
    let epoch_winner_page_pubkeys = (0..epoch_winners_meta.total_num_pages)
        .map(|page_index| ac::epoch_winners_page(&staking_program_id, epoch_index, page_index).pubkey)
        .collect::<Vec<_>>();
    let epoch_winner_page_accounts = rpc.get_multiple_accounts(&epoch_winner_page_pubkeys).unwrap();
    let mut winners = vec![];
    for (page_index, epoch_winner_page_account) in epoch_winner_page_accounts.into_iter().enumerate() {
        if let Some(epoch_winner_page_account) = epoch_winner_page_account {
            let epoch_winner_page: EpochWinnersPage =
                try_from_slice_unchecked(&epoch_winner_page_account.data).unwrap();
            winners.extend_from_slice(&epoch_winner_page.winners);
        } else {
            println!("Page {page_index} not found");
        }
    }
    println!("Winners: {:#?}", winners);
}

pub fn decode_staking_error(err: &TransactionError) -> Option<StakingError> {
    if let TransactionError::InstructionError(_, error) = err {
        if let InstructionError::Custom(number) = error {
            return nezha_staking_lib::error::StakingError::try_from(*number).ok();
        }
    }
    None
}
