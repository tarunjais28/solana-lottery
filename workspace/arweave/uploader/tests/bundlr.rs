use {
    anyhow::Result,
    arweave_uploader::{
        bundlr::{BundlrApiError, BundlrClient, BundlrConfig},
        DataWithMediaType, UploaderClient,
    },
    rand::distributions::{Alphanumeric, DistString},
    reqwest::header::CONTENT_TYPE,
    serde_json::{json, Value},
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{
        commitment_config::{CommitmentConfig, CommitmentLevel},
        signature::{read_keypair_file, Keypair, Signer},
    },
};

pub const BUNDLR_URL: &str = "https://devnet.bundlr.network";
pub const RPC_URL: &str = "https://api.devnet.solana.com";

fn config() -> BundlrConfig {
    let keypair = read_keypair_file("tests/fixtures/DrfJfPPPcYgc5utTMRQkbb7ExAsVW7Y2WKrfrhQ7ZEfg.json")
        .expect("Failed to read keypair file");

    BundlrConfig {
        bundlr_url: BUNDLR_URL.to_string(),
        keypair,
        api_client: reqwest::Client::new(),
        solana_client: RpcClient::new(RPC_URL.to_string()),
    }
}

#[tokio::test]
async fn test_fund_account() -> Result<()> {
    let keypair = Keypair::new();
    let rpc_client = RpcClient::new(RPC_URL.to_string());

    let signature = rpc_client.request_airdrop(&keypair.pubkey(), 1000000).await?;
    let recent_blockhash = rpc_client.get_latest_blockhash().await?;
    let commitment = CommitmentConfig {
        commitment: CommitmentLevel::Finalized,
    };
    rpc_client
        .confirm_transaction_with_spinner(&signature, &recent_blockhash, commitment)
        .await?;

    let config = BundlrConfig {
        bundlr_url: BUNDLR_URL.to_string(),
        keypair,
        api_client: reqwest::Client::new(),
        solana_client: RpcClient::new(RPC_URL.to_string()),
    };
    let client = BundlrClient::new(config);

    let amount = 10000;
    let balance_before = client.balance().await?;

    let signature = client.fund_account(amount).await?;
    assert!(rpc_client.confirm_transaction(&signature).await.is_ok());

    let balance_after = client.balance().await?;
    assert_eq!(balance_before + amount, balance_after);
    Ok(())
}

#[tokio::test]
async fn test_upload_bytes() -> Result<()> {
    let client = BundlrClient::new(config());

    let string = Alphanumeric.sample_string(&mut rand::thread_rng(), 10);
    let data_with_media_type = DataWithMediaType {
        data: string.clone(),
        media_type: mime::TEXT_PLAIN,
    };
    let response = client.upload_bytes(data_with_media_type).await;
    assert!(response.is_ok(), "{response:?}");
    let id = response.expect("Could not get Arweave ID");
    assert!(!id.is_empty());
    let res = reqwest::get(format!("https://arweave.net/{id}", id = id)).await?;
    let content_type = res
        .headers()
        .get(CONTENT_TYPE)
        .expect("Could not get content type")
        .to_str()?
        .parse::<mime::Mime>()?;
    assert_eq!(content_type.essence_str(), mime::TEXT_PLAIN);
    let response = res.text().await?;
    assert_eq!(response, string);
    Ok(())
}

#[tokio::test]
async fn test_upload_bytes_with_no_balance() -> Result<()> {
    let keypair = Keypair::new();

    let config = BundlrConfig {
        bundlr_url: BUNDLR_URL.to_string(),
        keypair,
        api_client: reqwest::Client::new(),
        solana_client: RpcClient::new(RPC_URL.to_string()),
    };
    let client = BundlrClient::new(config);
    let greeting = "hello there";
    let data_with_media_type = DataWithMediaType {
        data: greeting.to_string(),
        media_type: mime::TEXT_PLAIN,
    };
    let response = client.upload_bytes(data_with_media_type).await;
    assert!(response.is_err(), "{response:?}");
    let error = response.expect_err("Could not get error");
    let error = error
        .downcast_ref::<BundlrApiError>()
        .expect("Could not downcast error to BundlrApiError");
    assert!(matches!(error, BundlrApiError::NotEnoughFunds(_)));
    Ok(())
}

#[tokio::test]
async fn test_upload_json() -> Result<()> {
    let client = BundlrClient::new(config());

    let string = Alphanumeric.sample_string(&mut rand::thread_rng(), 10);
    let json = json!({
        "string": string,
    });
    let response = client.upload_json(&json).await;
    assert!(response.is_ok(), "{response:?}");
    let id = response.expect("Could not get Arweave ID");
    assert!(!id.is_empty());
    let res = reqwest::get(format!("https://arweave.net/{id}", id = id)).await?;
    let content_type = res
        .headers()
        .get(CONTENT_TYPE)
        .expect("Could not get content type")
        .to_str()?
        .parse::<mime::Mime>()?;
    assert_eq!(content_type.essence_str(), mime::APPLICATION_JSON);
    let response = res.json::<Value>().await?;
    assert_eq!(response, json);
    Ok(())
}
