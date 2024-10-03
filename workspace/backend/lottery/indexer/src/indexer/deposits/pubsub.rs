use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use borsh::BorshDeserialize;
use futures::stream::{BoxStream, StreamExt};
use nezha_staking::fixed_point::FPUSDC;
use nezha_staking::instruction::StakingInstruction;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_client::rpc_response::{Response, RpcLogsResponse};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_transaction_status::{EncodedTransaction, UiCompiledInstruction, UiMessage, UiTransactionEncoding};

use crate::nezha_api::NezhaAPI;

type ArcNezhaAPI = Arc<dyn NezhaAPI + Send + Sync>;
type RpcResp = Response<RpcLogsResponse>;

pub struct SolanaPubsub {
    pub rpc_client: Arc<RpcClient>,
    pub nezha_api: ArcNezhaAPI,
    pub program_id: Pubkey,
    pub last_transaction_id: Option<String>,
}

const MAX_RETRIES: usize = 20;
const RETRY_DELAY: Duration = Duration::from_secs(2);

const STABILIZE_DELAY: Duration = Duration::from_secs(1);

impl SolanaPubsub {
    pub fn new(program_id: Pubkey, rpc_client: Arc<RpcClient>, nezha_api: ArcNezhaAPI) -> Self {
        Self {
            rpc_client,
            nezha_api,
            program_id,
            last_transaction_id: None,
        }
    }

    pub async fn run_single(
        &mut self,
        epoch_running: Arc<AtomicBool>,
        receiver: &mut BoxStream<'_, RpcResp>,
    ) -> Result<()> {
        let msg = receiver
            .next()
            .await
            .context("Pubsub: Failed to get message from WS stream")?;

        tokio::time::sleep(STABILIZE_DELAY).await;

        let signature_str = msg.value.signature;

        if self.last_transaction_id.as_ref() == Some(&signature_str) {
            log::info!("Pubsub: Received duplicate transaction {}. Ignoring.", signature_str);
            return Ok(());
        }

        if let Some(err) = msg.value.err {
            // getTransaction() RPC returns null for failed transaction
            // So it's better to catch them early and skip them
            log::info!("Pubsub: Skipping failed transaction: {}. {}", signature_str, err);
            return Ok(());
        }

        log::info!("Pubsub: Received transaction {}", signature_str);

        let signature = Signature::from_str(&signature_str)
            .with_context(|| format!("Failed to parse signature {}", signature_str))?;

        let mut transaction = None;
        for i in 0..MAX_RETRIES {
            if i > 0 {
                log::info!("Retry {}/{}", i, MAX_RETRIES);
            }
            match self
                .rpc_client
                .get_transaction(&signature, UiTransactionEncoding::Json)
                .await
                .with_context(|| format!("Failed to get transaction details {}", signature_str))
            {
                Ok(txn) => {
                    transaction = Some(txn);
                    break;
                }
                Err(e) => {
                    log::error!("Pubsub error: {}. {}", e, e.root_cause());
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }

        let transaction =
            transaction.ok_or_else(|| anyhow!("Failed to get transaction details even after {} tries", MAX_RETRIES))?;

        let (account_keys, instructions) = extract_instructions(self.program_id, transaction)
            .with_context(|| format!("Failed to extract instructions from transaction {}", signature_str))?;

        for instruction in instructions {
            tokio::spawn(handle_instruction_with_retry(
                self.nezha_api.clone(),
                epoch_running.clone(),
                instruction,
                account_keys.clone(),
            ));
        }

        // Processing was successful. Store it so that we can skip it WS duplicates it.
        self.last_transaction_id = Some(signature_str);

        Ok(())
    }

    pub async fn run(&mut self, cancelled: Arc<AtomicBool>, config: &SolanaPubsubConfig) -> Result<()> {
        let mut pubsub_client = None;
        let mut stream = config
            .connect(&mut pubsub_client)
            .await
            .context("Failed to obtain stream")?;

        let epoch_running = Arc::new(AtomicBool::new(false));

        while !cancelled.load(Ordering::Relaxed) {
            let res = self.run_single(epoch_running.clone(), &mut stream).await;
            if let Err(err) = res {
                log::error!("Pubsub error: {}", err);
            }
        }
        Ok(())
    }
}

async fn handle_instruction_with_retry(
    nezha_api: ArcNezhaAPI,
    epoch_running: Arc<AtomicBool>,
    instruction: UiCompiledInstruction,
    account_keys: Vec<Pubkey>,
) {
    for i in 0..=MAX_RETRIES {
        if i > 0 {
            log::info!("Retry {}/{}", i, MAX_RETRIES);
        }
        match handle_instruction(
            nezha_api.as_ref(),
            epoch_running.clone(),
            instruction.clone(),
            &account_keys,
        )
        .await
        .with_context(|| format!("Failed to handle instruction: {:?}", instruction))
        {
            Ok(_) => break,
            Err(e) => {
                log::error!("Pubsub error: {}", e);
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
}

pub fn extract_instructions(
    program_id: Pubkey,
    transaction: solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
) -> Result<(Vec<Pubkey>, Vec<UiCompiledInstruction>)> {
    let ui_transaction = if let EncodedTransaction::Json(transaction) = transaction.transaction.transaction {
        transaction
    } else {
        bail!("Failed to decode EncodedTransaction into UiTransaction");
    };

    let message = if let UiMessage::Raw(message) = ui_transaction.message {
        message
    } else {
        bail!("Failed to decode UiMessage into UiRawMessage");
    };

    let mut ret = Vec::with_capacity(message.instructions.len());

    for instruction in message.instructions {
        let program_id_str = &message.account_keys[instruction.program_id_index as usize];
        if program_id
            != Pubkey::from_str(&program_id_str)
                .with_context(|| format!("Failed to decode program id: {}", program_id_str))?
        {
            continue;
        }

        ret.push(instruction);
    }

    let account_keys = message
        .account_keys
        .iter()
        .map(|key| Pubkey::from_str(&key).with_context(|| format!("Failed to decode program id: {}", key)))
        .collect::<Result<Vec<_>>>()?;

    Ok((account_keys, ret))
}

pub struct SolanaPubsubConfig {
    pub rpc_ws_url: String,
    pub program_id: Pubkey,
}

impl SolanaPubsubConfig {
    pub async fn new(rpc_ws_url: String, program_id: Pubkey) -> Self {
        Self { rpc_ws_url, program_id }
    }
}

impl SolanaPubsubConfig {
    // We have to do this tickery because we can't return (PubsubClient, BoxStream)
    // BoxStream's lifetime is tied to PubsubClient by the sdk.
    // So the tuple (PubsubClient, BoxStream) becomes self referential.
    pub async fn connect<'a>(&self, client: &'a mut Option<PubsubClient>) -> Result<BoxStream<'a, RpcResp>> {
        *client = Some(
            PubsubClient::new(&self.rpc_ws_url)
                .await
                .with_context(|| format!("Failed to connect to RPC Websocket {}", self.rpc_ws_url))?,
        );
        let (stream, _) = client
            .as_ref()
            .unwrap()
            .logs_subscribe(
                RpcTransactionLogsFilter::Mentions(vec![self.program_id.to_string()]),
                RpcTransactionLogsConfig {
                    commitment: Some(CommitmentConfig::finalized()),
                },
            )
            .await
            .with_context(|| "Failed to subscribe to logs using RPC Websocket")?;

        Ok(stream)
    }
}

// --------- Event handlers ----------------- //

pub async fn handle_instruction(
    nezha_api: &(dyn NezhaAPI + Send + Sync),
    epoch_running: Arc<AtomicBool>,
    instruction: UiCompiledInstruction,
    account_keys: &[Pubkey],
) -> Result<(), anyhow::Error> {
    let accounts = instruction.accounts;
    let data = bs58::decode(&instruction.data)
        .into_vec()
        .with_context(|| format!("Failed to bs58 decode instruction data: {}", &instruction.data))?;
    let instruction = StakingInstruction::try_from_slice(&data).with_context(|| {
        format!(
            "Failed to parse instruction data into StakingInstruction: {}",
            &instruction.data
        )
    })?;
    match instruction {
        StakingInstruction::CreateEpoch { .. } => {
            epoch_running.store(true, Ordering::Relaxed);
        }
        StakingInstruction::YieldWithdrawByInvestor { .. }
        | StakingInstruction::YieldDepositByInvestor { .. }
        | StakingInstruction::FranciumInvest { .. }
        | StakingInstruction::FranciumWithdraw { .. }
        | StakingInstruction::CreateEpochWinnersMeta { .. }
        | StakingInstruction::PublishWinners { .. } => {
            epoch_running.store(false, Ordering::Relaxed);
        }
        StakingInstruction::RequestStakeUpdate { amount } => {
            let user_pubkey = &account_keys[accounts[0] as usize];
            let action = if amount < 0 { "withdraw" } else { "deposit" };
            let amount = FPUSDC::from_usdc(amount.abs() as _);
            log::info!("Received by stake update request {}: {action} {amount}", user_pubkey);
            process_stake_update_request(nezha_api, user_pubkey).await?;
        }
        StakingInstruction::ApproveStakeUpdate { .. } => {
            let user_pubkey = &account_keys[accounts[1] as usize];
            log::info!("Received stake update approval {}", user_pubkey);
            process_stake_update_approval(nezha_api, user_pubkey, &epoch_running.load(Ordering::Relaxed)).await?;
        }
        StakingInstruction::CompleteStakeUpdate { .. } => {
            let user_pubkey = &account_keys[accounts[1] as usize];
            log::info!("Received stake update completion {}", user_pubkey);
            process_stake_update_completion(nezha_api, user_pubkey).await?;
        }
        x => {
            log::info!("Ignoring instruction {:?}", x);
        }
    };
    Ok(())
}

pub async fn process_stake_update_request(
    nezha_api: &(dyn NezhaAPI + Send + Sync),
    user_pubkey: &Pubkey,
) -> Result<()> {
    nezha_api.approve_stake_update(&user_pubkey).await.map_err(|e| {
        log::error!("Failed to approve deposit: {}", e);
        e
    })?;
    log::info!("Approved deposit for: {}", user_pubkey);
    Ok(())
}

pub async fn process_stake_update_approval(
    nezha_api: &(dyn NezhaAPI + Send + Sync),
    user_pubkey: &Pubkey,
    epoch_running: &bool,
) -> Result<()> {
    if *epoch_running {
        nezha_api.complete_stake_update(&user_pubkey).await.map_err(|e| {
            log::error!("Failed to complete deposit: {}", e);
            e
        })?;
    }
    log::info!("Approved deposit for: {}", user_pubkey);
    Ok(())
}

pub async fn process_stake_update_completion(
    nezha_api: &(dyn NezhaAPI + Send + Sync),
    user_pubkey: &Pubkey,
) -> Result<()> {
    nezha_api.generate_ticket(&user_pubkey).await.map_err(|e| {
        log::error!("Failed to generate ticket: {}", e);
        e
    })?;
    log::info!("Generated ticket for: {}", user_pubkey);
    Ok(())
}
