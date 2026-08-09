use serde::{Deserialize, Deserializer};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct ZbxHost {
    pub host: String,
    #[serde(deserialize_with = "de_i64_from_str_or_num")]
    pub hostid: i64,
}

#[derive(Debug, Deserialize)]
pub struct ZbxGroup {
    pub name: String,
    #[serde(deserialize_with = "de_i64_from_str_or_num")]
    pub groupid: i64,
}

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

pub fn de_i64_from_str_or_num<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Value::deserialize(deserializer)?;
    match v {
        Value::Number(n) => n
            .as_i64()
            .ok_or_else(|| serde::de::Error::custom("número fuera de rango para i64")),
        Value::String(s) => s.parse::<i64>().map_err(|e| {
            serde::de::Error::custom(format!("no se pudo parsear '{s}' como i64: {e}"))
        }),
        other => Err(serde::de::Error::custom(format!(
            "se esperaba número o string, se recibió: {other:?}"
        ))),
    }
}
