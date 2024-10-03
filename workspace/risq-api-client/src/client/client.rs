use std::fmt::Debug;

use super::AppResult;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};

#[async_trait]
pub trait Client {
    async fn get<Resp>(&self, res: &str, rel_path: &str) -> AppResult<Resp>
    where
        Resp: DeserializeOwned;
    async fn get_with_query<Query, Resp>(&self, res: &str, rel_path: &str, query: &Query) -> AppResult<Resp>
    where
        Query: Serialize + Send + Sync + Debug,
        Resp: DeserializeOwned;
    async fn post<Req, Resp>(&self, res: &str, rel_path: &str, data: &Req) -> AppResult<Resp>
    where
        Req: Serialize + Send + Sync + Debug,
        Resp: DeserializeOwned;
    async fn delete<Resp>(&self, res: &str, rel_path: &str) -> AppResult<Resp>
    where
        Resp: DeserializeOwned;

    async fn delete_raw(&self, res: &str, rel_path: &str) -> AppResult<(u16, Vec<u8>)>;
}
