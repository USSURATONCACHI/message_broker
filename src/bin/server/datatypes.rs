use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use uuid::Uuid;

pub type Username = String;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Topic {
    pub name: String,
    pub creator: Username,
    pub timestamp: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Message {
    pub uuid: Uuid,
    pub topic_uuid: Uuid,
    pub author_name: Username,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}