use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum ZbxResponse<T> {
    Ok {
        jsonrpc: String,
        result: T,
        id: i64,
    },
    Err {
        jsonrpc: String,
        error: DataErrorApiZbx,
        id: i64,
    },
}

#[derive(Deserialize, Debug)]
pub struct DataErrorApiZbx {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ZbxError {
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("zabbix api error: {kind} — {message}{}", data.as_deref().map(|d| format!(" ({d})")).unwrap_or_default())]
    Api {
        kind: ZbxErrorKind,
        message: String,
        data: Option<String>,
    },
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
