use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::{Context, Result};
use borsh::BorshDeserialize;
use chrono::{DateTime, NaiveDateTime, Utc};
use nezha_staking::{
    accounts as ac,
    fixed_point::FPUSDC,
    instruction::StakingInstruction,
    state::{EpochWinnersMeta, EpochWinnersPage, Winner, WinnerProcessingStatus, MAX_NUM_WINNERS_PER_PAGE},
};
use service::{
    model::{
        prize::Prize,
        transaction::{Transaction, TransactionId, TransactionType},
    },
    prize::PrizeRepository,
    transaction::{TransactionHistoryRepository, UserTransactionRepository},
};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_client::GetConfirmedSignaturesForAddress2Config};
use solana_sdk::{
    borsh::try_from_slice_unchecked, commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signature,
};
use solana_transaction_status::{
    option_serializer::OptionSerializer, EncodedTransaction, UiCompiledInstruction, UiInnerInstructions, UiInstruction,
    UiMessage, UiRawMessage, UiTransactionEncoding,
};

use crate::indexer::util::SolanaProgramContext;

pub enum TransactionItem {
    User(Transaction),
    Prize(Prize),
    Others,
}

pub struct PollingIndexer {
    rpc_client: Arc<RpcClient>,
    context: Arc<SolanaProgramContext>,
    retry_delay: Duration,
    batch_size: usize,
    transaction_history_repository: Box<dyn TransactionHistoryRepository>,
    user_transaction_repository: Box<dyn UserTransactionRepository>,
    prize_repository: Box<dyn PrizeRepository>,
}

impl PollingIndexer {
    pub fn new(
        rpc_client: Arc<RpcClient>,
        context: Arc<SolanaProgramContext>,
        retry_delay: Duration,
        batch_size: usize,
        transaction_history_repository: Box<dyn TransactionHistoryRepository>,
        user_transaction_repository: Box<dyn UserTransactionRepository>,
        prize_repository: Box<dyn PrizeRepository>,
    ) -> Self {
        Self {
            rpc_client,
            context,
            retry_delay,
            batch_size,
            transaction_history_repository,
            user_transaction_repository,
            prize_repository,
        }
    }

    pub async fn run_loop(&self) -> Result<()> {
        loop {
            let res = self.run().await;
            if let Err(e) = res {
                log::error!("Error while indexing transactions: {}", e);
            }
            tokio::time::sleep(self.retry_delay).await;
        }
    }

    pub async fn run(&self) -> Result<()> {
        let last_saved = self
            .transaction_history_repository
            .last_saved()
            .await?
            .map(Signature::try_from)
            .transpose()?;

        let mut before = None;
        let mut transactions = Vec::new();
        loop {
            let config = GetConfirmedSignaturesForAddress2Config {
                before,
                until: last_saved,
                limit: None,
                commitment: Some(CommitmentConfig::finalized()),
            };

            let txs = self
                .rpc_client
                .get_signatures_for_address_with_config(&self.context.staking_program_id, config)
                .await?;
            match txs.last() {
                Some(tx) => {
                    before = Some(Signature::from_str(&tx.signature)?);
                    transactions.extend(txs);
                }
                None => break,
            }
        }

        if !transactions.is_empty() {
            log::info!("Found {} transaction(s) since last saved", transactions.len());
        }

        let mut parsed_transactions = Vec::new();

        for transaction in transactions {
            log::info!("Parsing transaction {}", transaction.signature);
            let mut transaction_items = Vec::new();
            if transaction.err.is_some() {
                log::info!("Skipping errorful transaction {} ", transaction.signature);
                continue;
            }
            let transaction_id = Signature::from_str(&transaction.signature)?;

            let transaction = self
                .rpc_client
                .get_transaction(&transaction_id, UiTransactionEncoding::Json)
                .await?;

            let transaction_time = transaction
                .block_time
                .map(|secs| DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(secs, 0), Utc));
            let inner_instructions: Option<Vec<UiInnerInstructions>> = match transaction.transaction.meta {
                Some(meta) => match meta.inner_instructions {
                    OptionSerializer::Some(ixs) => Some(ixs),
                    OptionSerializer::None | OptionSerializer::Skip => None,
                },
                _ => None,
            };
            if let EncodedTransaction::Json(ui_transaction) = transaction.transaction.transaction {
                if let UiMessage::Raw(message) = ui_transaction.message {
                    for (index, ui_instruction) in message.instructions.iter().enumerate() {
                        if self.context.staking_program_id
                            == Pubkey::from_str(&message.account_keys[ui_instruction.program_id_index as usize])?
                        {
                            let data = bs58::decode(&ui_instruction.data).into_vec()?;
                            let instruction = StakingInstruction::try_from_slice(&data)?;
                            let transaction_id = TransactionId::from(transaction_id);
                            match instruction {
                                StakingInstruction::RequestStakeUpdate { amount } => {
                                    let wallet = get_instruction_account(0, ui_instruction, &message)?;
                                    let transaction = Transaction {
                                        transaction_id: transaction_id.clone(),
                                        instruction_index: index as u8,
                                        wallet,
                                        amount: FPUSDC::from_usdc(amount.abs() as u64),
                                        mint: self.context.usdc_mint_pubkey,
                                        time: transaction_time,
                                        transaction_type: if amount < 0 {
                                            TransactionType::WithdrawAttempt
                                        } else {
                                            TransactionType::DepositAttempt
                                        },
                                    };
                                    transaction_items.push(TransactionItem::User(transaction));
                                }
                                StakingInstruction::ApproveStakeUpdate { amount } => {
                                    let wallet = get_instruction_account(1, ui_instruction, &message)?;
                                    let transaction = Transaction {
                                        transaction_id: transaction_id.clone(),
                                        instruction_index: index as u8,
                                        wallet,
                                        amount: FPUSDC::from_usdc(amount.abs() as u64),
                                        mint: self.context.usdc_mint_pubkey,
                                        time: transaction_time,
                                        transaction_type: if amount < 0 {
                                            TransactionType::WithdrawApproved
                                        } else {
                                            TransactionType::DepositApproved
                                        },
                                    };
                                    transaction_items.push(TransactionItem::User(transaction));
                                }
                                StakingInstruction::CompleteStakeUpdate => {
                                    let transfer = get_token_transfer(
                                        &inner_instructions.as_ref().context("Can't find inner instructions")?,
                                        index as u8,
                                        &message,
                                    )?
                                    .context("Could not find amount for cancel stake update")?;

                                    let transaction_type = if transfer.destination
                                        == *ac::deposit_vault(&self.context.staking_program_id)
                                    {
                                        TransactionType::DepositCompleted
                                    } else {
                                        TransactionType::WithdrawCompleted
                                    };

                                    let wallet = get_instruction_account(1, ui_instruction, &message)?;
                                    let transaction = Transaction {
                                        transaction_id: transaction_id.clone(),
                                        instruction_index: index as u8,
                                        wallet,
                                        amount: FPUSDC::from_usdc(transfer.amount),
                                        mint: self.context.usdc_mint_pubkey,
                                        time: transaction_time,
                                        transaction_type,
                                    };
                                    transaction_items.push(TransactionItem::User(transaction));
                                }
                                StakingInstruction::CancelStakeUpdate { amount } => {
                                    let wallet = get_instruction_account(1, ui_instruction, &message)?;
                                    let transaction = Transaction {
                                        transaction_id: transaction_id.clone(),
                                        instruction_index: index as u8,
                                        wallet,
                                        amount: FPUSDC::from_usdc(amount.abs() as u64),
                                        mint: self.context.usdc_mint_pubkey,
                                        time: transaction_time,
                                        transaction_type: if amount < 0 {
                                            TransactionType::WithdrawCancelled
                                        } else {
                                            TransactionType::DepositCancelled
                                        },
                                    };
                                    transaction_items.push(TransactionItem::User(transaction));
                                }
                                StakingInstruction::ClaimWinning {
                                    epoch_index,
                                    winner_index,
                                    tier,
                                    page,
                                    ..
                                } => {
                                    let transfer = get_token_transfer(
                                        inner_instructions.as_ref().context("Can't find inner instructions")?,
                                        index as u8,
                                        &message,
                                    )?
                                    .context("Could not find amount for claim winning")?;
                                    let amount = FPUSDC::from_usdc(transfer.amount);

                                    let wallet = get_instruction_account(0, ui_instruction, &message)?;
                                    let mint = self.context.usdc_mint_pubkey;
                                    let transaction = Transaction {
                                        transaction_id: transaction_id.clone(),
                                        instruction_index: index as u8,
                                        wallet,
                                        amount,
                                        mint,
                                        time: transaction_time,
                                        transaction_type: TransactionType::Claim,
                                    };
                                    transaction_items.push(TransactionItem::User(transaction));

                                    let epoch_winners_meta: EpochWinnersMeta =
                                        get_instruction_account_data(1, ui_instruction, &message, &self.rpc_client)
                                            .await?;

                                    let epoch_winners_page: EpochWinnersPage =
                                        get_instruction_account_data(2, ui_instruction, &message, &self.rpc_client)
                                            .await?;

                                    let claimable = match tier {
                                        1 => epoch_winners_meta.jackpot_claimable,
                                        2 | 3 => true,
                                        _ => unreachable!(),
                                    };

                                    let winner_index_in_page = winner_index - page * MAX_NUM_WINNERS_PER_PAGE as u32;
                                    let winner: &Winner = &epoch_winners_page.winners[winner_index_in_page as usize];

                                    let prize = Prize {
                                        wallet,
                                        epoch_index,
                                        page,
                                        winner_index,
                                        tier,
                                        amount,
                                        claimable,
                                        claimed: winner.claimed,
                                    };
                                    transaction_items.push(TransactionItem::Prize(prize));
                                }
                                StakingInstruction::PublishWinners { .. } | StakingInstruction::FundJackpot { .. } => {
                                    let meta_account_index = match instruction {
                                        StakingInstruction::PublishWinners { .. } => 3,
                                        StakingInstruction::FundJackpot { .. } => 3,
                                        _ => unreachable!(),
                                    };

                                    let epoch_winners_meta: EpochWinnersMeta = get_instruction_account_data(
                                        meta_account_index,
                                        ui_instruction,
                                        &message,
                                        &self.rpc_client,
                                    )
                                    .await?;
                                    if epoch_winners_meta.status == WinnerProcessingStatus::Completed {
                                        let pages = 0..epoch_winners_meta.total_num_pages;
                                        let page_pubkeys = pages
                                            .clone()
                                            .map(|page_index| {
                                                nezha_staking::accounts::epoch_winners_page(
                                                    &self.context.staking_program_id,
                                                    epoch_winners_meta.epoch_index,
                                                    page_index,
                                                )
                                                .pubkey
                                            })
                                            .collect::<Vec<_>>();
                                        let page_accounts =
                                            self.rpc_client.get_multiple_accounts(&page_pubkeys).await?;
                                        for (page, page_account) in pages.zip(page_accounts) {
                                            let page_account = match page_account {
                                                Some(page_account) => page_account,
                                                None => {
                                                    log::error!("Epoch winner page account is None. Page {}", page);
                                                    continue;
                                                }
                                            };
                                            let epoch_winners_page =
                                                try_from_slice_unchecked::<EpochWinnersPage>(&page_account.data)?;
                                            for (winner_index, winner) in epoch_winners_page.winners.iter().enumerate()
                                            {
                                                let claimable = match winner.tier {
                                                    1 => epoch_winners_meta.jackpot_claimable,
                                                    2 | 3 => true,
                                                    _ => unreachable!(),
                                                };
                                                let prize = Prize {
                                                    wallet: winner.address,
                                                    epoch_index: epoch_winners_meta.epoch_index,
                                                    page,
                                                    winner_index: winner_index as _,
                                                    tier: winner.tier,
                                                    amount: winner.prize,
                                                    claimable,
                                                    claimed: winner.claimed,
                                                };
                                                transaction_items.push(TransactionItem::Prize(prize));
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                } else {
                    log::error!("Unexpected transaction encoding");
                }
            }
            parsed_transactions.push((TransactionId::from(transaction_id), transaction_items));
        }

        self.store_transactions(&mut parsed_transactions).await?;

        Ok(())
    }

    async fn store_transactions(
        &self,
        parsed_transactions: &mut Vec<(TransactionId, Vec<TransactionItem>)>,
    ) -> Result<()> {
        while !parsed_transactions.is_empty() {
            let mut count = 0;
            let mut transaction_ids = Vec::new();
            let mut prizes_batch = Vec::new();
            let mut user_tx_batch = Vec::new();
            while let Some((transaction_id, transaction_items)) = parsed_transactions.pop() {
                transaction_ids.push(transaction_id);
                for transaction_item in transaction_items {
                    match transaction_item {
                        TransactionItem::User(transaction) => {
                            log::info!("Storing transaction: {:?}", transaction);
                            user_tx_batch.push(transaction);
                        }
                        TransactionItem::Prize(prize) => {
                            log::info!("Storing prize: {:?}", prize);
                            prizes_batch.push(prize);
                        }
                        _ => {}
                    };
                }
                count += 1;
                if count >= self.batch_size {
                    break;
                }
            }
            tokio::try_join!(
                self.prize_repository.upsert_prizes(&prizes_batch),
                self.user_transaction_repository.store_transactions(&user_tx_batch)
            )?;
            self.transaction_history_repository
                .save_transaction_ids(&transaction_ids)
                .await?;
        }
        Ok(())
    }
}

#[allow(unused)]
struct TokenTransfer {
    source: Pubkey,
    destination: Pubkey,
    amount: u64,
}

fn get_token_transfer(
    inner_instructions_list: &[UiInnerInstructions],
    index: u8,
    message: &UiRawMessage,
) -> Result<Option<TokenTransfer>> {
    if let Some(inner_instructions) = inner_instructions_list.iter().find(|ix| ix.index == index) {
        for instruction in &inner_instructions.instructions {
            let instruction = if let UiInstruction::Compiled(instruction) = instruction {
                instruction
            } else {
                continue;
            };

            let data = bs58::decode(&instruction.data).into_vec()?;

            let token_instruction = if let Ok(instruction) = spl_token::instruction::TokenInstruction::unpack(&data) {
                instruction
            } else {
                continue;
            };

            if let spl_token::instruction::TokenInstruction::Transfer { amount } = token_instruction {
                let source = get_instruction_account(0, instruction, message)?;
                let destination = get_instruction_account(1, instruction, message)?;

                return Ok(Some(TokenTransfer {
                    source,
                    destination,
                    amount,
                }));
            }
        }
    }

    Ok(None)
}

pub fn get_instruction_account(
    index: u32,
    instruction: &UiCompiledInstruction,
    message: &UiRawMessage,
) -> Result<Pubkey> {
    let global_index = instruction
        .accounts
        .get(index as usize)
        .with_context(|| format!("Can't get the global index for instruction index {}", index))?;
    let account = message
        .account_keys
        .get(*global_index as usize)
        .with_context(|| format!("Can't get the account at global index {}", global_index))?;
    Pubkey::from_str(account).with_context(|| format!("Can't parse into pubkey {}", account))
}

pub async fn get_instruction_account_data<T: BorshDeserialize>(
    index: u32,
    instruction: &UiCompiledInstruction,
    message: &UiRawMessage,
    rpc_client: &RpcClient,
) -> Result<T> {
    let pubkey = get_instruction_account(index, instruction, message)?;
    let account_data = rpc_client.get_account_data(&pubkey).await?;
    let account = try_from_slice_unchecked(&account_data)?;
    Ok(account)
}
