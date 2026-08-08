use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ApiZbxResponse<T> {
    pub id: i32,
    pub jsonrpc: String,
    pub result: T,
}

#[derive(Debug, Deserialize)]
pub struct ErrorApiZbxResponse {
    pub id: i32,
    pub jsonrpc: String,
    pub error: DataErrorApiZbx,
}

#[derive(Debug, Deserialize)]
pub struct DataErrorApiZbx {
    pub code: i64,
    pub message: String,
    pub data: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ZbxError {
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("zabbix api error: {kind} — {data}")]
    Api { kind: ZbxErrorKind, data: String },
}

#[derive(Debug, thiserror::Error)]
pub enum ZbxErrorKind {
    #[error("invalid params")]
    InvalidParams, // -32602
    #[error("method not found")]
    MethodNotFound, // -32601
    #[error("invalid request")]
    InvalidRequest, // -32600
    #[error("application error (auth/permissions)")]
    ApplicationError, // -32500
    #[error("unknown error code: {0}")]
    Unknown(i64),
}

impl From<i64> for ZbxErrorKind {
    fn from(code: i64) -> Self {
        match code {
            -32600 => ZbxErrorKind::InvalidRequest,
            -32601 => ZbxErrorKind::MethodNotFound,
            -32602 => ZbxErrorKind::InvalidParams,
            -32500 => ZbxErrorKind::ApplicationError,
            other => ZbxErrorKind::Unknown(other),
        }
    }
}
