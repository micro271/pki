use serde::{
    Deserialize, Deserializer, Serialize,
    de::{IgnoredAny, Visitor},
};
use serde_json::Value;

#[derive(Debug)]
pub struct ZbxAuditLog {
    pub resourceid: i64,
    pub details: ZbxAuditDetail,
}

impl<'de> Deserialize<'de> for ZbxAuditLog {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum _Fields {
            Field0,
            Field1,
            Ignore,
        }

        struct _FieldVisitor;

        impl<'de> Visitor<'de> for _FieldVisitor {
            type Value = _Fields;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "resourceid" => Ok(_Fields::Field0),
                    "details" => Ok(_Fields::Field1),
                    _ => Ok(_Fields::Ignore),
                }
            }
        }
        impl<'de> Deserialize<'de> for _Fields {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_identifier(_FieldVisitor)
            }
        }

        struct _Visitor;

        impl<'de> Visitor<'de> for _Visitor {
            type Value = ZbxAuditLog;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut resourceid: Option<i64> = None;
                let mut details: Option<ZbxAuditDetail> = None;

                while let Some(n) = map.next_key::<_Fields>()? {
                    match n {
                        _Fields::Field0 => match map.next_value::<i64>() {
                            Ok(val) => {
                                if resourceid.is_some() {
                                    return Err(serde::de::Error::duplicate_field("resourceid"));
                                } else {
                                    resourceid = Some(val)
                                }
                            }
                            Err(er) => return Err(er),
                        },
                        _Fields::Field1 => match map.next_value::<ZbxAuditDetail>() {
                            Ok(val) => {
                                if details.is_some() {
                                    return Err(serde::de::Error::duplicate_field("details"));
                                } else {
                                    details = Some(val)
                                }
                            }
                            Err(er) => return Err(er),
                        },
                        _Fields::Ignore => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }

                Ok(ZbxAuditLog {
                    resourceid: resourceid.unwrap(),
                    details: details.unwrap(),
                })
            }
        }
        const FIELDS: &'static [&'static str; 2] = &["resourceid", "details"];

        deserializer.deserialize_struct("ZbxAuditLog", FIELDS, _Visitor)
    }
}

#[derive(Debug)]
pub struct ZbxAuditDetail {
    pub host: Option<ZbxResourceOp>,
    pub status: Option<ZbxResourceOp>,
}

impl<'de> Deserialize<'de> for ZbxAuditDetail {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }
}

#[derive(Debug)]
pub enum ZbxResourceOp {
    Add { value: String },
    Update { new: String, old: String },
    Delete { value: String },
}

impl<'de> Deserialize<'de> for ZbxResourceOp {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        todo!()
    }
}

#[derive(Debug, Deserialize)]
pub struct ZbxHost {
    pub host: String,
    #[serde(deserialize_with = "de_i64_from_str_or_num")]
    pub hostid: i64,
    pub status: HostStatus,
}

#[derive(Debug, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase", type_name = "host_status")]
pub enum HostStatus {
    Enable,
    Disable,
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

pub enum ZbxApiResourceType {
    User,
    Host,
    Action,
    UserGroup,
    Trigger,
    HostGroup,
    Item,
    Proxy,
    Template,
}

impl Serialize for ZbxApiResourceType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value = match self {
            ZbxApiResourceType::User => "0",
            ZbxApiResourceType::Host => "4",
            ZbxApiResourceType::Action => "5",
            ZbxApiResourceType::UserGroup => "11",
            ZbxApiResourceType::Trigger => "13",
            ZbxApiResourceType::HostGroup => "14",
            ZbxApiResourceType::Item => "15",
            ZbxApiResourceType::Proxy => "26",
            ZbxApiResourceType::Template => "30",
        };
        serializer.serialize_str(value)
    }
}

impl<'de> Deserialize<'de> for ZbxApiResourceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ResourceTypeVisitor;

        impl<'de> Visitor<'de> for ResourceTypeVisitor {
            type Value = ZbxApiResourceType;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a valid Zabbix resource type number")
            }
            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    0 => Ok(ZbxApiResourceType::User),
                    4 => Ok(ZbxApiResourceType::Host),
                    5 => Ok(ZbxApiResourceType::Action),
                    11 => Ok(ZbxApiResourceType::UserGroup),
                    13 => Ok(ZbxApiResourceType::Trigger),
                    14 => Ok(ZbxApiResourceType::HostGroup),
                    15 => Ok(ZbxApiResourceType::Item),
                    26 => Ok(ZbxApiResourceType::Proxy),
                    30 => Ok(ZbxApiResourceType::Template),
                    _ => Err(E::custom(format!("unknown resource type: {}", value))),
                }
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let num: u64 = value
                    .parse()
                    .map_err(|_| E::custom(format!("invalid resource type string: {}", value)))?;
                self.visit_u64(num)
            }
        }

        deserializer.deserialize_any(ResourceTypeVisitor)
    }
}
