use serde::Deserialize;
use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    ClientError(#[from] anyhow::Error),
    #[error("Server error: HTTP {http_code:?}. error code: {code}, request id: {req_id}")]
    ServerError {
        http_code: HttpStatusCode,
        code: String,
        req_id: String,
        debug: Option<String>,
    },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerError {
    pub code: String,
    pub req_id: String,
    pub debug: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum HttpStatusCode {
    OK,
    BadRequest,
    Unauthorized,
    NotFound,
    Conflict,
    InternalServerError,
}

impl From<(HttpStatusCode, ServerError)> for AppError {
    fn from((status, err): (HttpStatusCode, ServerError)) -> Self {
        AppError::ServerError {
            http_code: status,
            code: err.code,
            req_id: err.req_id,
            debug: err.debug,
        }
    }
}
