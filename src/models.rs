use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Row {
    pub eventid: i64,
    pub host: String,
    pub severity: Severity,
    pub trigger: String,
    pub start_time: i64,
    pub opdata: String,
    pub end_time: i64,
    pub status: Status,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Status {
    Resolved,
    Ongoing,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum Severity {
    Warning,
    Information,
    Average,
    High,
    Desaster,
    NotClassifier,
}

impl Severity {
    pub fn to_number(self) -> i32 {
        match self {
            Severity::NotClassifier => 0,
            Severity::Information => 1,
            Severity::Warning => 2,
            Severity::Average => 3,
            Severity::High => 4,
            Severity::Desaster => 5,
        }
    }

    pub fn from_number(severity: i32) -> Option<Self> {
        (0..6).contains(&severity).then_some(match severity {
            1 => Severity::Information,
            2 => Severity::Warning,
            3 => Severity::Average,
            4 => Severity::High,
            5 => Severity::Desaster,
            _ => Severity::NotClassifier,
        })
    }
}
