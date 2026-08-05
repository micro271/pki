use influxdb3_client::FieldValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Event {
    pub eventid: i64,
    pub nodo: String,
    pub severity: Severity,
    pub trigger: String,

    #[serde(rename = "time")]
    pub start_time: i64,

    pub opdata: String,
    pub end_time: Option<i64>,
    pub status: Status,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Status {
    Resolved,
    Ongoing,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolved => write!(f, "RESOLVED"),
            Self::Ongoing => write!(f, "ONGOING"),
        }
    }
}

impl From<Status> for FieldValue {
    fn from(value: Status) -> Self {
        FieldValue::String(value.to_string())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Severity {
    Warning,
    Information,
    Average,
    High,
    Disaster,
    NotClassifier,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Warning => write!(f, "warning"),
            Severity::Information => write!(f, "information"),
            Severity::Average => write!(f, "average"),
            Severity::High => write!(f, "high"),
            Severity::Disaster => write!(f, "disaster"),
            Severity::NotClassifier => write!(f, "notClassifier"),
        }
    }
}

impl From<Severity> for String {
    fn from(value: Severity) -> Self {
        value.to_string()
    }
}

impl Severity {
    pub fn to_number(self) -> i32 {
        match self {
            Severity::NotClassifier => 0,
            Severity::Information => 1,
            Severity::Warning => 2,
            Severity::Average => 3,
            Severity::High => 4,
            Severity::Disaster => 5,
        }
    }

    pub fn from_number(severity: i32) -> Option<Self> {
        (0..6).contains(&severity).then_some(match severity {
            1 => Severity::Information,
            2 => Severity::Warning,
            3 => Severity::Average,
            4 => Severity::High,
            5 => Severity::Disaster,
            _ => Severity::NotClassifier,
        })
    }
}

pub struct EventRaw {}
