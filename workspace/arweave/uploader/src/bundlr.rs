use {
    crate::{data::DataWithMediaType, dataitem::DataItem, tag::Tag, UploaderClient},
    anyhow::Result,
    async_trait::async_trait,
    http::StatusCode,
    reqwest::header::CONTENT_TYPE,
    serde::{Deserialize, Serialize},
    serde_json::{json, Value},
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{
        pubkey::Pubkey,
        signature::{Keypair, Signature, Signer},
    },
    std::mem::size_of_val,
    thiserror::Error,
};

const CURRENCY: &str = "solana";

pub struct BundlrConfig {
    pub bundlr_url: String,
    pub keypair: Keypair,
    pub api_client: reqwest::Client,
    pub solana_client: RpcClient,
}
pub struct BundlrClient {
    api: String,
    keypair: Keypair,
    api_client: reqwest::Client,
    solana_client: RpcClient,
}

#[derive(Debug, Clone, Serialize, Deserialize, Error)]
pub enum BundlrApiError {
    #[error("Not enough funds in Bundlr account. Required: {0} lamports")]
    NotEnoughFunds(u64),

    #[error("Cannot parse JSON response")]
    JsonParseError,

    #[error("Response error: {0}")]
    ResponseError(String),

    #[error("Unknown error: {0}")]
    UnknownError(String),
}

impl BundlrClient {
    pub fn new(config: BundlrConfig) -> Self {
        Self {
            api: config.bundlr_url,
            keypair: config.keypair,
            api_client: config.api_client,
            solana_client: config.solana_client,
        }
    }

    pub async fn calculate_price(&self, bytes: usize) -> Result<u64> {
        let response = self
            .api_client
            .get(format!(
                "{api_url}/price/{currency}/{bytes}",
                api_url = self.api,
                currency = CURRENCY,
                bytes = bytes
            ))
            .send()
            .await?;
        let price: u64 = response.json::<u64>().await?;
        Ok(price)
    }

    pub async fn balance(&self) -> Result<u64> {
        let pubkey = self.keypair.pubkey();
        let response = self
            .api_client
            .get(format!(
                "{api_url}/account/balance/{currency}?address={address}",
                api_url = self.api,
                currency = CURRENCY,
                address = pubkey,
            ))
            .send()
            .await?;

        let balance = response.json::<Value>().await?["balance"]
            .as_str()
            .ok_or(BundlrApiError::JsonParseError)?
            .parse::<u64>()?;
        Ok(balance)
    }

    pub async fn fund_account(&self, amount: u64) -> Result<Signature> {
        let to = self.bundlr_address().await?;

        let recent_blockhash = self.solana_client.get_latest_blockhash().await?;
        let transaction = solana_sdk::system_transaction::transfer(&self.keypair, &to, amount, recent_blockhash);
        let signature = self
            .solana_client
            .send_and_confirm_transaction_with_spinner(&transaction)
            .await?;

        let funding_tx = json!({
            "tx_id": signature.to_string()
        });

        let response = self
            .api_client
            .post(format!(
                "{api_url}/account/balance/{currency}",
                api_url = self.api,
                currency = CURRENCY,
            ))
            .header(CONTENT_TYPE, mime::APPLICATION_JSON.to_string())
            .json(&funding_tx)
            .send()
            .await?;

        let status = response.status();
        if status.is_success() {
            Ok(signature)
        } else {
            let msg = format!("Status = {}", status);
            Err(BundlrApiError::ResponseError(msg).into())
        }
    }

    // Bundlr provides payment methods for different currencies.
    // This method returns the address of the Bundlr account for Solana.
    // https://docs.bundlr.network/docs/client/api/classes/Utils#getbundleraddress
    pub async fn bundlr_address(&self) -> Result<Pubkey> {
        let response = self
            .api_client
            .get(format!("{api_url}/info", api_url = self.api,))
            .send()
            .await?;
        let info = response.json::<Value>().await?;
        let solana_address = info["addresses"]["solana"]
            .as_str()
            .ok_or(BundlrApiError::JsonParseError)?
            .parse()?;
        Ok(solana_address)
    }
}

#[async_trait]
impl UploaderClient for BundlrClient {
    async fn upload_bytes<T: AsRef<[u8]> + Send>(&self, data_with_media_type: DataWithMediaType<T>) -> Result<String> {
        let data = data_with_media_type.bytes();
        let bytes = size_of_val(&data);

        let tags = vec![Tag {
            name: CONTENT_TYPE.to_string(),
            value: data_with_media_type.media_type.to_string(),
        }];

        let data_item = DataItem::create(data, tags, &self.keypair)?;
        let tx = data_item.into_inner();

        let response = self
            .api_client
            .post(format!(
                "{api_url}/tx/{currency}",
                api_url = self.api,
                currency = CURRENCY,
            ))
            .header(CONTENT_TYPE, mime::APPLICATION_OCTET_STREAM.to_string())
            .body(tx)
            .send()
            .await?;

        let status = response.status();
        match status {
            StatusCode::OK | StatusCode::CREATED => {
                let id = response.json::<Value>().await?["id"]
                    .as_str()
                    .ok_or(BundlrApiError::JsonParseError)?
                    .to_string();
                Ok(id)
            }
            StatusCode::PAYMENT_REQUIRED => {
                let price = self.calculate_price(bytes).await?;
                Err(BundlrApiError::NotEnoughFunds(price).into())
            }
            _ => {
                let msg = format!("Status = {}", status);
                Err(BundlrApiError::ResponseError(msg).into())
            }
        }
    }

    async fn upload_json<T: Serialize + Sync>(&self, data: &T) -> Result<String> {
        let data = serde_json::to_vec(data)?;
        let data_with_media_type = DataWithMediaType {
            data,
            media_type: mime::APPLICATION_JSON,
        };
        let id = self.upload_bytes(data_with_media_type).await?;
        Ok(id)
    }
}
