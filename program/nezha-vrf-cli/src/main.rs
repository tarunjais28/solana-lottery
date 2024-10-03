use ::borsh::BorshDeserialize;
use chrono::{Local, NaiveDateTime, TimeZone};
use solana_client::rpc_client::RpcClient;
use solana_program::borsh0_10;
use solana_sdk::{
    instruction::{Instruction, InstructionError},
    native_token::{lamports_to_sol, sol_to_lamports},
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signature, Signer},
    transaction::{Transaction, TransactionError},
};

use nezha_vrf_lib::{
    accounts as ac,
    error::NezhaVrfError,
    instruction,
    state::{NezhaVrfProgramState, NezhaVrfRequest, NezhaVrfRequestStatus},
    switchboard,
};

use nezha_staking_lib::accounts as staking_ac;

use std::{
    env::{self, args},
    path::PathBuf,
    thread,
};
use std::{fs, str::FromStr};

fn main() {
    let args: Vec<String> = args().collect();

    let rpc_url = env::var("DEMO_SOLANA_RPC_URL").unwrap();
    let nezha_staking_program_id = env::var("SOLANA_STAKING_PROGRAM_ID")
        .map(|s| Pubkey::from_str(&s).unwrap())
        .unwrap();
    let nezha_vrf_program_id = env::var("SOLANA_VRF_PROGRAM_ID")
        .map(|s| Pubkey::from_str(&s).unwrap())
        .unwrap();

    println!("RPC: {}", rpc_url);
    let rpc = RpcClient::new(rpc_url.to_string());

    let super_admin_kp = read_keypair_file(key_path("keys/super_admin.json")).expect("failed to read keypair file");
    let super_admin_pubkey = super_admin_kp.pubkey();

    let admin_kp = read_keypair_file(key_path("keys/admin.json")).expect("failed to read keypair file");
    let admin_pubkey = admin_kp.pubkey();

    let switchboard_program_id = switchboard::SWITCHBOARD_PROGRAM_ID;
    let switchboard_queue = switchboard::SWITCHBOARD_LABS_DEVNET_PERMISSIONLESS_QUEUE;

    let switchboard_queue_data = rpc.get_account_data(&switchboard_queue).unwrap();
    let switchboard_queue_account =
        switchboard::deserialize_oracle_queue_account_data(&switchboard_queue_data).unwrap();

    let (sb_state, _bump) = switchboard::get_program_state_pda(&switchboard_program_id);
    let sb_state_data = rpc.get_account_data(&sb_state).unwrap();
    let sb_state_account = switchboard::deserialize_sb_state(&sb_state_data).unwrap();

    let switchboard_queue_mint = sb_state_account.dao_mint;
    println!("Mint 1: {}", switchboard_queue_mint);
    let switchboard_queue_authority = switchboard_queue_account.authority;
    let switchboard_queue_data_buffer = switchboard_queue_account.data_buffer;

    let nezha_vrf_program_state = ac::nezha_vrf_program_state(&nezha_vrf_program_id).pubkey;

    let command = &args[1];
    match &command[..] {
        "replenish-sols" => {
            let mut txns = Vec::new();
            txns.extend(replenish_account(&rpc, super_admin_pubkey, 1.0));
            txns.extend(replenish_account(&rpc, admin_pubkey, 1.0));

            for txn in txns {
                confirm_tx(&rpc, txn);
            }
        }
        "init" => {
            let ix = instruction::init(
                &nezha_vrf_program_id,
                &super_admin_pubkey,
                &admin_pubkey,
                &switchboard_program_id,
                &switchboard_queue,
                &switchboard_queue_authority,
                &switchboard_queue_mint,
                &nezha_staking_program_id,
            );
            send_txs(&rpc, &admin_kp, &[&admin_kp, &super_admin_kp], &[ix]);
        }
        "show-program-state" => {
            show_account::<NezhaVrfProgramState>(&rpc, &nezha_vrf_program_state);
        }
        "show-request" => {
            let epoch_index = args[2].parse::<u64>().unwrap();
            let request_pubkey = ac::nezha_vrf_request(&nezha_vrf_program_id, epoch_index).pubkey;
            show_account::<NezhaVrfRequest>(&rpc, &request_pubkey);
        }
        "show-vrf-lite" => {
            let epoch_index = args[2].parse::<u64>().unwrap();
            let request_pubkey = ac::nezha_vrf_request(&nezha_vrf_program_id, epoch_index).pubkey;
            show_account::<NezhaVrfRequest>(&rpc, &request_pubkey);
        }
        "wait-for-request" => {
            let epoch_index = args[2].parse::<u64>().unwrap();
            let request_pubkey = ac::nezha_vrf_request(&nezha_vrf_program_id, epoch_index).pubkey;
            let start = Local::now();
            println!("Start: {:?}", start);
            loop {
                let now = Local::now();
                let duration = now.signed_duration_since(start);
                print!("\r {}m {}s", duration.num_minutes(), duration.num_seconds() % 60);
                let request = get_account::<NezhaVrfRequest>(&rpc, &request_pubkey);
                if request.status != NezhaVrfRequestStatus::Waiting {
                    println!();
                    println!("{:#?}", request);
                    let request_start = NaiveDateTime::from_timestamp_opt(request.request_start, 0).unwrap();
                    let request_end = NaiveDateTime::from_timestamp_opt(request.request_end.unwrap(), 0).unwrap();
                    let request_start = Local.from_utc_datetime(&request_start);
                    let request_end = Local.from_utc_datetime(&request_end);
                    let dur = request_end - request_start;
                    println!(
                        "Request started at: {}.\n Ended at: {}.\n Duration: {}m {}s",
                        request_start,
                        request_end,
                        dur.num_hours(),
                        dur.num_seconds() % 60
                    );
                    break;
                }
            }
        }
        "request-vrf" => {
            let latest_epoch = &staking_ac::latest_epoch(&nezha_staking_program_id);
            let epoch_index = args[2].parse::<u64>().unwrap();

            println!(
                "keys: {:?}",
                [
                    &nezha_vrf_program_id,
                    &admin_pubkey,
                    &switchboard_program_id,
                    &switchboard_queue,
                    &switchboard_queue_authority,
                    &switchboard_queue_mint,
                    &switchboard_queue_data_buffer,
                ]
            );

            let ix = instruction::request_vrf(
                &nezha_vrf_program_id,
                &admin_pubkey,
                &switchboard_program_id,
                &switchboard_queue,
                &switchboard_queue_authority,
                &switchboard_queue_mint,
                &switchboard_queue_data_buffer,
                latest_epoch,
                epoch_index,
            );
            send_txs(&rpc, &admin_kp, &[&admin_kp], &[ix]);
        }
        _ => {
            eprintln!("error: invalid command");
        }
    }
}

fn key_path(path: &str) -> PathBuf {
    let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), path);
    fs::canonicalize(&path).expect(&format!("unable to key: {}", path).to_string())
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

fn send_txs(rpc: &RpcClient, payer: &Keypair, signers_kp: &[&Keypair], ixns: &[Instruction]) {
    let hash = rpc.get_latest_blockhash().unwrap();
    let mut tx = Transaction::new_with_payer(ixns, Some(&payer.pubkey()));
    tx.sign(&signers_kp.to_vec(), hash);
    match rpc.simulate_transaction(&tx) {
        Ok(sim_result) => {
            println!("Program logs: {:#?}", sim_result.value.logs.unwrap_or_default());
            if let Some(err) = sim_result.value.err {
                if let Some(nezha_vrf_error) = decode_nezha_vrf_error(&err) {
                    println!("Error: {}", nezha_vrf_error);
                } else {
                    println!("Error: {}", err);
                }
                panic!("Simulation failed");
            }
        }
        Err(err) => {
            println!("Failed to simulate txn");
            if let Some(nezha_vrf_error) = err.get_transaction_error().and_then(|err| decode_nezha_vrf_error(&err)) {
                println!("Error: {}", nezha_vrf_error);
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
            if let Some(nezha_vrf_error) = err.get_transaction_error().and_then(|err| decode_nezha_vrf_error(&err)) {
                println!("Error: {}", nezha_vrf_error);
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

fn get_account<T>(rpc: &RpcClient, pubkey: &Pubkey) -> T
where
    T: BorshDeserialize,
{
    let data = rpc.get_account_data(&pubkey).unwrap();
    let account: T = borsh0_10::try_from_slice_unchecked(&data).unwrap();
    account
}

fn show_account<T>(rpc: &RpcClient, pubkey: &Pubkey)
where
    T: BorshDeserialize + std::fmt::Debug,
{
    let account: T = get_account(rpc, pubkey);
    println!("{:#?}", &account);
}

pub fn decode_nezha_vrf_error(err: &TransactionError) -> Option<NezhaVrfError> {
    if let TransactionError::InstructionError(_, error) = err {
        if let InstructionError::Custom(number) = error {
            return NezhaVrfError::try_from(*number).ok();
        }
    }
    None
}
