use chrono::{DateTime, Utc};
use uuid::Uuid;

pub type Username = String;

#[derive(Clone, Debug, Default)]
pub struct Topic {
    pub name: String,
    pub creator: Username,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Default)]
pub struct Message {
    pub topic: Uuid,
    pub author_name: Username,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}