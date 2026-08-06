use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub eventid: i64,
    pub nodo: String,
    pub severity: Severity,
    pub trigger: String,
    pub start_time: i64,
    pub opdata: String,
    pub end_time: Option<i64>,
    pub status: Status,
}

#[derive(Deserialize, Serialize, Debug, sqlx::Type)]
#[sqlx(rename_all = "lowercase", type_name = "event_status")]
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

impl TryFrom<&str> for Status {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "RESOLVED" => Self::Resolved,
            "ONGOING" => Self::Ongoing,
            _ => return Err(()),
        })
    }
}

#[derive(Debug, Deserialize, Serialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase", type_name = "severity_level")]
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

impl TryFrom<&str> for Severity {
    type Error = ();
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "warning" => Severity::Warning,
            "information" => Severity::Information,
            "average" => Severity::Average,
            "high" => Severity::High,
            "disaster" => Severity::Disaster,
            "notClassifier" => Severity::NotClassifier,
            _ => return Err(()),
        })
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

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Timestamp(i64);

impl Timestamp {
    pub fn set(&mut self, n: i64) {
        self.0 = n;
    }
    pub fn get(&self) -> i64 {
        self.0
    }
    pub fn new(n: i64) -> Self {
        Self(n)
    }
}

impl std::ops::Deref for Timestamp {
    type Target = i64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Timestamp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct EventId(i64);

impl EventId {
    pub fn new(n: i64) -> Self {
        Self(n)
    }
    pub fn set(&mut self, n: i64) {
        self.0 = n;
    }

    pub fn get(&self) -> i64 {
        self.0
    }
}

impl std::ops::Deref for EventId {
    type Target = i64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for EventId {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
