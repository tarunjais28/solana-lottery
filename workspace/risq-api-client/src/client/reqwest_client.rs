use anyhow::Context;
use async_trait::async_trait;
use reqwest::Response;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

use super::{AppError, AppResult, Client, HttpStatusCode, ServerError};

#[derive(Clone)]
pub struct ReqwestClient {
    base_url: String,
    bearer_token: String,
    client: reqwest::Client,
}

fn get_path(base_url: &str, res: &str, rel_path: &str) -> String {
    format!("{base_url}/risq/b2b/{res}/api/v1/{rel_path}")
}

impl ReqwestClient {
    pub fn new(base_url: String, partner_id: String, api_key: String) -> Self {
        Self {
            base_url,
            bearer_token: format!("{partner_id}:{api_key}"),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Client for ReqwestClient {
    async fn get<Resp>(&self, res: &str, rel_path: &str) -> AppResult<Resp>
    where
        Resp: DeserializeOwned,
    {
        let path = get_path(&self.base_url, res, rel_path);
        let bearer_token = &self.bearer_token;

        log::info!(target: "risq_api", "GET {path}");
        log::info!(target: "risq_api", "Authorization: Bearer {bearer_token}");

        let resp = self
            .client
            .get(path)
            .bearer_auth(bearer_token)
            .send()
            .await
            .context("Failed to send request")?;

        let bytes = handle_error(resp)
            .await?
            .bytes()
            .await
            .context("Failed to read response")?;
        deserialize_json(&bytes)
    }

    async fn get_with_query<Query, Resp>(&self, res: &str, rel_path: &str, query: &Query) -> AppResult<Resp>
    where
        Query: Serialize + Send + Sync + Debug,
        Resp: DeserializeOwned,
    {
        let path = get_path(&self.base_url, res, rel_path);
        let bearer_token = &self.bearer_token;

        log::debug!(target: "risq_api", "GET {path}?{query:?}");
        log::debug!(target: "risq_api", "Authorization: Bearer {bearer_token}");

        let resp = self
            .client
            .get(path)
            .query(query)
            .bearer_auth(bearer_token)
            .send()
            .await
            .context("Failed to send request")?;

        let bytes = handle_error(resp)
            .await?
            .bytes()
            .await
            .context("Failed to read response")?;
        deserialize_json(&bytes)
    }

    async fn post<Req, Resp>(&self, res: &str, rel_path: &str, data: &Req) -> AppResult<Resp>
    where
        Req: Serialize + Send + Sync + Debug,
        Resp: DeserializeOwned,
    {
        let path = get_path(&self.base_url, res, rel_path);
        let bearer_token = &self.bearer_token;

        log::debug!(target: "risq_api", "POST {path}");
        log::debug!(target: "risq_api", "Authorization: Bearer {bearer_token}");

        let data_str = serde_json::to_string(data).unwrap();
        log::debug!(target: "risq_api", "Body: {data_str}");

        let resp = self
            .client
            .post(path)
            .header("Content-Type", "application/json")
            .body(data_str)
            .bearer_auth(bearer_token)
            .send()
            .await
            .context("Failed to send request")?;

        let bytes = handle_error(resp)
            .await?
            .bytes()
            .await
            .context("Failed to read response")?;
        deserialize_json(&bytes)
    }

    async fn delete<Resp>(&self, res: &str, rel_path: &str) -> AppResult<Resp>
    where
        Resp: DeserializeOwned,
    {
        let path = get_path(&self.base_url, res, rel_path);
        let bearer_token = &self.bearer_token;

        log::debug!(target: "risq_api", "DELETE {path}");
        log::debug!(target: "risq_api", "Authorization: Bearer {bearer_token}");

        let resp = self
            .client
            .delete(path)
            .bearer_auth(bearer_token)
            .send()
            .await
            .context("Failed to send request")?;

        let bytes = handle_error(resp)
            .await?
            .bytes()
            .await
            .context("Failed to read response")?;
        deserialize_json(&bytes)
    }

    async fn delete_raw(&self, res: &str, rel_path: &str) -> AppResult<(u16, Vec<u8>)> {
        let path = get_path(&self.base_url, res, rel_path);
        let bearer_token = &self.bearer_token;

        log::debug!(target: "risq_api", "DELETE {path}");
        log::debug!(target: "risq_api", "Authorization: Bearer {bearer_token}");

        let resp = self
            .client
            .delete(path)
            .bearer_auth(bearer_token)
            .send()
            .await
            .context("Failed to send request")?;

        let resp = handle_error(resp).await?;

        let status_code = resp.status();
        let bytes = resp.bytes().await.context("Failed to read response")?;
        Ok((status_code.as_u16(), bytes.to_vec()))
    }
}

async fn handle_error(resp: Response) -> AppResult<Response> {
    let status = resp.status();
    if !status.is_success() {
        let bytes = resp.bytes().await.context("Failed to read response")?;
        let error: ServerError = deserialize_json(&bytes).context("Failed to parse error")?;
        let http_status = match status.as_u16() {
            400 => Ok(HttpStatusCode::BadRequest),
            401 => Ok(HttpStatusCode::Unauthorized),
            404 => Ok(HttpStatusCode::NotFound),
            409 => Ok(HttpStatusCode::Conflict),
            500 => Ok(HttpStatusCode::InternalServerError),
            x => Err(anyhow::anyhow!("Unexpected status code {x}")),
        }?;
        let app_error: AppError = (http_status, error).into();
        Err(app_error)
    } else {
        Ok(resp)
    }
}

fn deserialize_json<T: DeserializeOwned>(bytes: &[u8]) -> AppResult<T> {
    serde_json::from_slice(bytes)
        .map_err(|err| {
            let mut serde_error = format_serde_error::SerdeError::new(String::from_utf8_lossy(bytes).to_string(), err);
            serde_error.set_contextualize(false);
            serde_error
        })
        .context("Failed to parse response")
        .map_err(|e| e.into())
}
