use std::collections::HashMap;

use serde::{
    Deserialize, Deserializer, Serialize,
    de::{DeserializeSeed, IgnoredAny, Visitor},
};
use serde_json::Value;

type ZbxAuditDetail = HashMap<String, ZbxResourceOp>;

#[derive(Debug)]
pub struct ZbxAuditLog {
    pub resourceid: i64,
    pub details: ZbxAuditDetail,
    pub action: AuditAction,
    pub clock: i64,
    pub resourcename: String,
}

impl<'de> Deserialize<'de> for ZbxAuditLog {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum _Fields {
            Field0,
            Field1,
            Field3,
            Field4,
            Field5,
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
                    "action" => Ok(_Fields::Field3),
                    "clock" => Ok(_Fields::Field4),
                    "resourcename" => Ok(_Fields::Field5),
                    _ => Ok(_Fields::Ignore),
                }
            }
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    0u64 => Ok(_Fields::Field0),
                    1u64 => Ok(_Fields::Field1),
                    2u64 => Ok(_Fields::Field3),
                    3u64 => Ok(_Fields::Field4),
                    4u64 => Ok(_Fields::Field5),
                    _ => Ok(_Fields::Ignore),
                }
            }
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    b"resourceid" => Ok(_Fields::Field0),
                    b"details" => Ok(_Fields::Field1),
                    b"action" => Ok(_Fields::Field3),
                    b"clock" => Ok(_Fields::Field4),
                    b"resourcename" => Ok(_Fields::Field5),
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

        struct _VisitI64;

        impl<'de> Visitor<'de> for _VisitI64 {
            type Value = i64;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("")
            }
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(v)
            }
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i64::try_from(v).map_err(|_| {
                    serde::de::Error::invalid_value(
                        serde::de::Unexpected::Unsigned(v),
                        &"a value that fits in i64",
                    )
                })
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse().map_err(|_| {
                    serde::de::Error::invalid_value(
                        serde::de::Unexpected::Str(v),
                        &"a string containing a valid i64",
                    )
                })
            }
        }

        struct FlexibleI64Seed;

        impl<'de> DeserializeSeed<'de> for FlexibleI64Seed {
            type Value = i64;

            fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_any(_VisitI64)
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
                let mut resourceid = None;
                let mut details = None;
                let mut action = None;
                let mut clock = None;
                let mut rname = None;
                while let Some(n) = map.next_key::<_Fields>()? {
                    match n {
                        _Fields::Field0 => {
                            if resourceid.is_some() {
                                return Err(serde::de::Error::duplicate_field("resourceid"));
                            }

                            resourceid = Some(map.next_value_seed(FlexibleI64Seed)?);
                        }
                        _Fields::Field1 => match map.next_value::<String>() {
                            Ok(raw) => {
                                if details.is_some() {
                                    return Err(serde::de::Error::duplicate_field("details"));
                                } else {
                                    details = Some(if raw.trim().is_empty() {
                                        HashMap::new()
                                    } else {
                                        serde_json::from_str(&raw).map_err(|e| {
                                            serde::de::Error::custom(format!(
                                                "error parseando details: {e}"
                                            ))
                                        })?
                                    });
                                }
                            }
                            Err(er) => return Err(er),
                        },
                        _Fields::Field3 => {
                            if action.is_some() {
                                return Err(serde::de::Error::duplicate_field("action"));
                            }

                            action = Some(map.next_value::<AuditAction>()?);
                        }
                        _Fields::Field4 => {
                            if clock.is_some() {
                                return Err(serde::de::Error::duplicate_field("action"));
                            }

                            clock = Some(map.next_value_seed(FlexibleI64Seed)?);
                        }
                        _Fields::Field5 => {
                            if rname.is_some() {
                                return Err(serde::de::Error::duplicate_field("resourcename"));
                            }

                            rname = Some(map.next_value::<String>()?);
                        }
                        _Fields::Ignore => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }

                Ok(ZbxAuditLog {
                    resourceid: resourceid
                        .ok_or_else(|| serde::de::Error::missing_field("resourceid"))?,
                    details: details.ok_or_else(|| serde::de::Error::missing_field("details"))?,
                    action: action.ok_or_else(|| serde::de::Error::missing_field("action"))?,
                    clock: clock.ok_or_else(|| serde::de::Error::missing_field("clock"))?,
                    resourcename: rname.ok_or_else(|| serde::de::Error::missing_field("clock"))?,
                })
            }
        }
        const FIELDS: &'static [&'static str; 5] =
            &["resourceid", "details", "action", "clock", "resourcename"];

        deserializer.deserialize_struct("ZbxAuditLog", FIELDS, _Visitor)
    }
}

#[derive(Debug)]
pub enum AuditAction {
    Add,
    Update,
    Delete,
}

impl<'de> Deserialize<'de> for AuditAction {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct _Visit;

        impl<'de> Visitor<'de> for _Visit {
            type Value = AuditAction;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a string or integer representing an AuditAction (0, 1 or 2)")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "0" => Ok(AuditAction::Add),
                    "1" => Ok(AuditAction::Update),
                    "2" => Ok(AuditAction::Delete),
                    e => Err(serde::de::Error::unknown_variant(e, &["0", "1", "2"])),
                }
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    0 => Ok(AuditAction::Add),
                    1 => Ok(AuditAction::Update),
                    2 => Ok(AuditAction::Delete),
                    e => Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Signed(e),
                        &"0, 1 or 2",
                    )),
                }
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    0 => Ok(AuditAction::Add),
                    1 => Ok(AuditAction::Update),
                    2 => Ok(AuditAction::Delete),
                    e => Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Unsigned(e),
                        &"0, 1 or 2",
                    )),
                }
            }
        }

        deserializer.deserialize_any(_Visit)
    }
}

/*
#[derive(Debug)]
pub struct ZbxAuditDetail {
    pub host: Option<ZbxResourceOp>,
    pub status: Option<ZbxResourceOp>,
}

impl<'de> Deserialize<'de> for ZbxAuditDetail {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum _Field {
            Field0,
            Field1,
            Ignore,
        }
        struct _FieldVisitor;
        impl<'de> Visitor<'de> for _FieldVisitor {
            type Value = _Field;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "host.host" => Ok(_Field::Field0),
                    "host.status" => Ok(_Field::Field1),
                    _ => Ok(_Field::Ignore),
                }
            }
        }

        impl<'de> Deserialize<'de> for _Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_identifier(_FieldVisitor)
            }
        }

        struct _Visitor;

        impl<'de> Visitor<'de> for _Visitor {
            type Value = ZbxAuditDetail;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut host = None;
                let mut status = None;
                while let Some(variant) = map.next_key::<_Field>()? {
                    match variant {
                        _Field::Field0 => {
                            if host.is_some() {
                                return Err(serde::de::Error::duplicate_field("host.host"));
                            } else {
                                host = Some(map.next_value::<ZbxResourceOp>()?);
                            }
                        }
                        _Field::Field1 => {
                            if status.is_some() {
                                return Err(serde::de::Error::duplicate_field("host.status"));
                            } else {
                                status = Some(map.next_value::<ZbxResourceOp>()?);
                            }
                        }
                        _Field::Ignore => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }
                Ok(ZbxAuditDetail { host, status })
            }
        }
        const FIELDS: &'static [&'static str] = &["host", "status"];
        deserializer.deserialize_struct("ZbxAuditDetail", FIELDS, _Visitor)
    }
}
*/

#[derive(Debug)]
pub enum ZbxResourceOp {
    Add { value: String },
    Update { new: String, old: String },
    Delete { value: String },
}

impl<'de> Deserialize<'de> for ZbxResourceOp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum _Field {
            Field0,
            Field1,
            Field2,
        }

        struct _FieldVisitor;

        impl<'de> Visitor<'de> for _FieldVisitor {
            type Value = _Field;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("")
            }
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    "add" => Ok(_Field::Field0),
                    "update" => Ok(_Field::Field1),
                    "delete" => Ok(_Field::Field2),
                    e => Err(serde::de::Error::custom(format!("Invalid Variant {e:?}"))),
                }
            }
        }

        impl<'de> Deserialize<'de> for _Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                deserializer.deserialize_identifier(_FieldVisitor)
            }
        }

        struct _Visitor;

        impl<'de> Visitor<'de> for _Visitor {
            type Value = ZbxResourceOp;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("")
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                match seq.next_element::<_Field>()? {
                    Some(_Field::Field0) => Ok(ZbxResourceOp::Add {
                        value: seq
                            .next_element::<String>()?
                            .ok_or_else(|| serde::de::Error::missing_field("value"))?,
                    }),
                    Some(_Field::Field1) => {
                        let new = seq.next_element::<String>()?;
                        let old = seq.next_element::<String>()?;

                        Ok(ZbxResourceOp::Update {
                            new: new.ok_or_else(|| serde::de::Error::missing_field("new"))?,
                            old: old.ok_or_else(|| serde::de::Error::missing_field("old"))?,
                        })
                    }
                    Some(_Field::Field2) => Ok(ZbxResourceOp::Delete {
                        value: seq
                            .next_element::<String>()?
                            .ok_or_else(|| serde::de::Error::missing_field("value"))?,
                    }),
                    None => Err(serde::de::Error::custom("")),
                }
            }
        }

        deserializer.deserialize_seq(_Visitor)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_add() {
        let json = r#"["add", "192.168.1.10"]"#;
        let result: ZbxResourceOp = serde_json::from_str(json).unwrap();

        match result {
            ZbxResourceOp::Add { value } => assert_eq!(value, "192.168.1.10"),
            other => panic!("esperaba Add, obtuve {:?}", other),
        }
    }

    #[test]
    fn test_deserialize_update() {
        let json = r#"["update", "servidor-nuevo", "servidor-viejo"]"#;
        let result: ZbxResourceOp = serde_json::from_str(json).unwrap();

        match result {
            ZbxResourceOp::Update { new, old } => {
                assert_eq!(new, "servidor-nuevo");
                assert_eq!(old, "servidor-viejo");
            }
            other => panic!("esperaba Update, obtuve {:?}", other),
        }
    }

    #[test]
    fn test_deserialize_delete() {
        let json = r#"["delete", "15"]"#;
        let result: ZbxResourceOp = serde_json::from_str(json).unwrap();

        match result {
            ZbxResourceOp::Delete { value } => assert_eq!(value, "15"),
            other => panic!("esperaba Delete, obtuve {:?}", other),
        }
    }

    #[test]
    fn test_deserialize_invalid_variant() {
        let json = r#"["rename", "x"]"#;
        let result: Result<ZbxResourceOp, _> = serde_json::from_str(json);

        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_missing_update_fields() {
        // update necesita 2 valores además del tag, acá solo viene 1
        let json = r#"["update", "solo-uno"]"#;
        let result: Result<ZbxResourceOp, _> = serde_json::from_str(json);

        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_delete_without_value() {
        // caso real de Zabbix: delete de un sub-objeto sin valor, ej. host.groups[N]
        let json = r#"["delete"]"#;
        let result: Result<ZbxResourceOp, _> = serde_json::from_str(json);

        // con tu implementación actual esto debería FALLAR (Delete requiere value: String)
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_empty_array() {
        let json = r#"[]"#;
        let result: Result<ZbxResourceOp, _> = serde_json::from_str(json);

        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_update_rename() {
        let json = r#"
        {
            "auditid": "cmsze1uj20001o9iyi4tluziv",
            "userid": "1",
            "username": "admin",
            "clock": "1787101432",
            "ip": "172.30.0.158",
            "action": "1",
            "resourcetype": "4",
            "resourceid": "10715",
            "resource_cuid": "0",
            "resourcename": "CA-GP-",
            "recordsetid": "cmsze1uj10000o9iyn07fcaav",
            "details": "{\"host.host\":[\"update\",\"CA-GP\",\"CA-GP-\"],\"host.name\":[\"update\",\"CA-GP\",\"CA-GP-\"]}"
        }
        "#;

        let log: ZbxAuditLog = serde_json::from_str(json).unwrap();

        assert_eq!(log.resourceid, 10715);
        assert_eq!(log.details.len(), 2);

        match log.details.get("host.host") {
            Some(ZbxResourceOp::Update { new, old }) => {
                assert_eq!(new, "CA-GP");
                assert_eq!(old, "CA-GP-");
            }
            other => panic!("esperaba Update en host.host, obtuve {:?}", other),
        }

        match log.details.get("host.name") {
            Some(ZbxResourceOp::Update { new, old }) => {
                assert_eq!(new, "CA-GP");
                assert_eq!(old, "CA-GP-");
            }
            other => panic!("esperaba Update en host.name, obtuve {:?}", other),
        }
    }

    #[test]
    fn test_deserialize_add_with_empty_details() {
        let json = r#"
        {
            "auditid": "cmshr5b2q000fe1iyv5wgabfc",
            "userid": "1",
            "username": "admin",
            "clock": "1786035037",
            "ip": "172.30.0.1",
            "action": "0",
            "resourcetype": "4",
            "resourceid": "10973",
            "resource_cuid": "0",
            "resourcename": "Nodo_Campo-De-Las-Carreras-1497_Y_M-Lillo",
            "recordsetid": "cmshr5b2q000ee1iyek29jyuz",
            "details": ""
        }
        "#;

        let log: ZbxAuditLog = serde_json::from_str(json).unwrap();

        assert_eq!(log.resourceid, 10973);
        assert!(log.details.is_empty());
    }

    #[test]
    fn test_deserialize_delete_with_empty_details() {
        let json = r#"
        {
            "auditid": "cmrzhoiiq001pshiy9hi0dvf8",
            "userid": "1",
            "username": "admin",
            "clock": "1784930746",
            "ip": "172.30.0.158",
            "action": "2",
            "resourcetype": "4",
            "resourceid": "10719",
            "resource_cuid": "0",
            "resourcename": "Teleste-01",
            "recordsetid": "cmrzhoieu0000shiy4weab5kx",
            "details": ""
        }
        "#;

        let log: ZbxAuditLog = serde_json::from_str(json).unwrap();

        assert_eq!(log.resourceid, 10719);
        assert!(log.details.is_empty());
    }

    #[test]
    fn test_deserialize_ignores_unknown_fields() {
        // confirma que campos no mapeados (auditid, userid, clock, etc.)
        // se descartan sin romper el parseo
        let json = r#"
        {
            "auditid": "xyz",
            "userid": "1",
            "clock": "123456",
            "action": "1",
            "resourcetype": "4",
            "resourceid": "999",
            "resourcename": "algo",
            "details": ""
        }
        "#;

        let log: ZbxAuditLog = serde_json::from_str(json).unwrap();
        assert_eq!(log.resourceid, 999);
    }

    #[test]
    fn test_deserialize_missing_resourceid_fails() {
        let json = r#"
        {
            "details": ""
        }
        "#;

        let result: Result<ZbxAuditLog, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_missing_details_fails() {
        let json = r#"
        {
            "resourceid": "999"
        }
        "#;

        let result: Result<ZbxAuditLog, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_full_array_response() {
        // el caso real: la API devuelve un array de estos objetos
        let json = r#"
        [
            {
                "resourceid": "10953",
                "action": "1",
                "details": "{\"host.host\":[\"update\",\"Teleste-02\",\"Teleste-021\"],\"host.name\":[\"update\",\"Teleste-02\",\"Teleste-021\"]}"
            },
            {
                "resourceid": "10719",
                "action": "2",
                "details": ""
            }
        ]
        "#;

        let logs: Vec<ZbxAuditLog> = serde_json::from_str(json).unwrap();

        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].resourceid, 10953);
        assert_eq!(logs[1].resourceid, 10719);
        assert!(logs[1].details.is_empty());
    }
}
