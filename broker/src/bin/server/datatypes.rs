use chrono::{DateTime, Utc};

pub type Username = String;

#[derive(Clone, Debug)]
pub struct Topic {
    pub name: String,
    pub creator: Username,
    pub timestamp: DateTime<Utc>,
}