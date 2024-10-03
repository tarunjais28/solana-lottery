use pretty_assertions::assert_eq;
use std::collections::BTreeSet;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use rand::Rng;
use service::{
    epoch::FPUSDC,
    model::transaction::{Transaction, TransactionId, TransactionType},
    transaction::{TransactionHistoryRepository, UserTransactionRepository},
};
use solana_sdk::{pubkey::Pubkey, signature::Signature};
use store::transactions::{PostgresTransactionHistoryRepository, PostgresUserTransactionRepository};

use crate::common;

fn create_transaction() -> Transaction {
    let mut rng = rand::thread_rng();
    let transaction_type = rng.gen::<TransactionType>();

    let amount = FPUSDC::from_usdc(rng.gen_range(100..1000u64) * 1_000_000);
    Transaction {
        transaction_id: TransactionId(Signature::new_unique().to_string()),
        instruction_index: rng.gen_range(0..10),
        wallet: Pubkey::new_unique(),
        amount,
        mint: Pubkey::new_unique(),
        time: Some(
            DateTime::parse_from_rfc3339(&Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true))
                .unwrap()
                .with_timezone(&Utc),
        ),
        transaction_type,
    }
}

async fn get_user_transaction_repo() -> PostgresUserTransactionRepository {
    let pool = common::setup().await;
    PostgresUserTransactionRepository::new(pool, 100)
}

#[tokio::test]
async fn test_by_transaction_id_and_instruction_index() -> Result<()> {
    let repo = get_user_transaction_repo().await;
    let transaction = create_transaction();
    repo.store_transaction(&transaction).await?;
    let stored_transaction = repo
        .by_transaction_id_and_instruction_index(&transaction.transaction_id, transaction.instruction_index)
        .await?;
    assert!(stored_transaction.is_some(), "{:?}", stored_transaction);
    assert_eq!(stored_transaction, Some(transaction));
    Ok(())
}

#[tokio::test]
async fn test_by_transaction_id() -> Result<()> {
    let transaction_id = TransactionId::from(Signature::new_unique());
    let pool = common::setup().await;
    let client = pool.get().await?;
    client
        .execute(
            "DELETE FROM user_transaction WHERE transaction_id = $1",
            &[&transaction_id.0],
        )
        .await?;

    let repo = get_user_transaction_repo().await;
    let mut transactions = BTreeSet::new();
    while transactions.len() < 10 {
        let transaction = Transaction {
            transaction_id: transaction_id.clone(),
            ..create_transaction()
        };
        repo.store_transaction(&transaction).await?;
        transactions.insert(transaction);
    }
    let transactions = transactions.into_iter().collect::<Vec<_>>();
    let mut stored_transactions = repo.by_transaction_id(&transaction_id).await?;
    stored_transactions.sort();

    assert_eq!(stored_transactions, transactions);
    Ok(())
}

#[tokio::test]
async fn test_by_wallet() -> Result<()> {
    let wallet = Pubkey::new_unique();
    let pool = common::setup().await;
    let client = pool.get().await?;
    client
        .execute("DELETE FROM user_transaction WHERE wallet = $1", &[&wallet.to_string()])
        .await?;

    let repo = get_user_transaction_repo().await;
    let mut transactions = BTreeSet::new();
    while transactions.len() < 10 {
        let transaction = Transaction {
            wallet: wallet.clone(),
            ..create_transaction()
        };
        repo.store_transaction(&transaction).await?;
        transactions.insert(transaction);
    }
    let transactions = transactions.into_iter().collect::<Vec<_>>();
    let mut stored_transactions = repo.by_wallet(&wallet, 10, 0).await?;
    stored_transactions.sort();
    assert_eq!(stored_transactions, transactions);
    Ok(())
}

#[tokio::test]
async fn test_by_transaction_type() -> Result<()> {
    let mut rng = rand::thread_rng();
    let transaction_type = rng.gen::<TransactionType>();
    let pool = common::setup().await;
    let client = pool.get().await?;
    client
        .execute(
            "DELETE FROM user_transaction WHERE transaction_type = $1",
            &[&transaction_type.to_string()],
        )
        .await?;

    let repo = get_user_transaction_repo().await;
    let mut transactions = BTreeSet::new();
    while transactions.len() < 10 {
        let amount = FPUSDC::from_usdc(rng.gen_range(100..1000u64) * 1_000_000);
        let wallet = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let transaction = Transaction {
            amount,
            wallet,
            mint,
            transaction_type: transaction_type.clone(),
            ..create_transaction()
        };
        repo.store_transaction(&transaction).await?;
        transactions.insert(transaction);
    }
    let transactions = transactions.into_iter().collect::<Vec<_>>();
    let mut stored_transactions = repo.by_type(transaction_type, 10, 0).await?;
    stored_transactions.sort();
    assert_eq!(stored_transactions, transactions);
    Ok(())
}

#[tokio::test]
async fn test_by_wallet_and_transaction_type() -> Result<()> {
    let repo = get_user_transaction_repo().await;
    let wallet = Pubkey::new_unique();

    for _ in 0..10 {
        let transaction = Transaction {
            wallet: wallet.clone(),
            ..create_transaction()
        };
        repo.store_transaction(&transaction).await?;
    }

    let mut rng = rand::thread_rng();
    let transaction_type = rng.gen::<TransactionType>();

    let pool = common::setup().await;
    let client = pool.get().await?;
    client
        .execute(
            "DELETE FROM user_transaction WHERE wallet = $1 AND transaction_type = $2",
            &[&wallet.to_string(), &transaction_type.to_string()],
        )
        .await?;

    let mut transactions = BTreeSet::new();
    while transactions.len() < 10 {
        let transaction = Transaction {
            wallet: wallet.clone(),
            transaction_type: transaction_type.clone(),
            ..create_transaction()
        };
        repo.store_transaction(&transaction).await?;
        transactions.insert(transaction);
    }
    let transactions = transactions.into_iter().collect::<Vec<_>>();

    let mut stored_transactions = repo.by_wallet_and_type(&wallet, transaction_type, 10, 0).await?;
    stored_transactions.sort();
    assert_eq!(stored_transactions, transactions);
    Ok(())
}

#[tokio::test]
async fn test_all() -> Result<()> {
    let pool = common::setup().await;
    let client = pool.get().await?;
    client.execute("DELETE FROM user_transaction", &[]).await?;
    let repo = get_user_transaction_repo().await;
    let mut transactions = BTreeSet::new();
    while transactions.len() < 50 {
        let transaction = create_transaction();
        repo.store_transaction(&transaction).await?;
        transactions.insert(transaction);
    }
    let transactions = transactions.into_iter().collect::<Vec<_>>();

    let mut stored_transactions = repo.all(50, 0).await?;
    stored_transactions.sort();
    assert_eq!(stored_transactions, transactions);
    Ok(())
}

#[tokio::test]
async fn test_store_transaction() -> Result<()> {
    let repo = get_user_transaction_repo().await;
    let transaction = create_transaction();
    repo.store_transaction(&transaction).await?;
    let stored_transaction = repo
        .by_transaction_id_and_instruction_index(&transaction.transaction_id, transaction.instruction_index)
        .await
        .expect("Failed to get transaction by transaction_id");
    assert!(stored_transaction.is_some(), "{:?}", stored_transaction);
    assert_eq!(stored_transaction.unwrap(), transaction);
    Ok(())
}

#[tokio::test]
async fn test_store_transactions() -> Result<()> {
    let repo = get_user_transaction_repo().await;
    let mut transactions = vec![];
    for _ in 0..10 {
        transactions.push(create_transaction());
    }
    repo.store_transactions(&transactions).await?;
    for transaction in transactions {
        let stored_transaction = repo
            .by_transaction_id_and_instruction_index(&transaction.transaction_id, transaction.instruction_index)
            .await
            .expect("Failed to get transaction by transaction_id");
        assert!(stored_transaction.is_some(), "{:?}", stored_transaction);
        assert_eq!(stored_transaction.unwrap(), transaction);
    }
    Ok(())
}

#[tokio::test]
async fn test_store_duplicate_transaction() -> Result<()> {
    let repo = get_user_transaction_repo().await;
    let transaction = create_transaction();
    repo.store_transaction(&transaction).await?;
    let res = repo.store_transaction(&transaction).await;
    assert!(res.is_ok(), "{:?}", res);
    Ok(())
}

#[tokio::test]
async fn test_store_duplicate_transactions() -> Result<()> {
    let repo = get_user_transaction_repo().await;
    let mut transactions = vec![];
    for _ in 0..5 {
        transactions.push(create_transaction());
    }
    repo.store_transactions(&transactions).await?;
    let res = repo.store_transactions(&transactions).await;
    assert!(res.is_ok(), "{:?}", res);
    Ok(())
}

async fn get_transaction_history_repo() -> PostgresTransactionHistoryRepository {
    let pool = common::setup().await;
    PostgresTransactionHistoryRepository::new(pool)
}

#[tokio::test]
async fn test_save_transaction_id() -> Result<()> {
    let repo = get_transaction_history_repo().await;
    let transaction_id = TransactionId::from(Signature::new_unique());
    let res = repo.save_transaction_id(&transaction_id).await;
    assert!(res.is_ok(), "{:?}", res);
    let last_saved = repo.last_saved().await?;
    assert!(last_saved.is_some());
    assert_eq!(last_saved, Some(transaction_id));
    Ok(())
}

#[tokio::test]
async fn test_save_transaction_ids() -> Result<()> {
    let repo = get_transaction_history_repo().await;
    let transaction_ids = (0..10)
        .map(|_| TransactionId::from(Signature::new_unique()))
        .collect::<Vec<_>>();
    let res = repo.save_transaction_ids(&transaction_ids).await;
    assert!(res.is_ok(), "{:?}", res);
    let last_saved = repo.last_saved().await?;
    assert!(last_saved.is_some());
    assert_eq!(last_saved, Some(transaction_ids[transaction_ids.len() - 1].clone()));
    Ok(())
}

#[tokio::test]
async fn test_total_deposit_by_wallet() -> Result<()> {
    let pool = common::setup().await;
    let client = pool.get().await?;
    let wallet = Pubkey::new_unique();
    client
        .execute(
            "DELETE FROM user_transaction WHERE transaction_type=$1 AND wallet=$2",
            &[&TransactionType::DepositCompleted.to_string(), &wallet.to_string()],
        )
        .await?;
    let repo = get_user_transaction_repo().await;
    let total = repo.total_deposit_by_wallet(&wallet).await?;
    assert_eq!(total, FPUSDC::zero());

    let mut expected_total = FPUSDC::zero();
    let mut rng = rand::thread_rng();

    for _ in 0..10 {
        let transaction = Transaction {
            wallet,
            amount: FPUSDC::from_usdc(rng.gen()),
            transaction_type: TransactionType::DepositCompleted,
            ..create_transaction()
        };
        repo.store_transaction(&transaction).await?;
        expected_total = expected_total
            .checked_add(transaction.amount)
            .ok_or(anyhow!("Overflow while calculating total deposit amount"))?;
    }

    let total = repo.total_deposit_by_wallet(&wallet).await?;
    assert_eq!(total, expected_total);

    Ok(())
}
