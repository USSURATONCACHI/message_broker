use std::fmt::Display;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

pub type Retention = Option<Duration>;

#[derive(Clone, Debug, Default)]
pub struct Topic {
    pub uuid: Uuid,
    pub name: String,
    pub creator: String,
    pub timestamp: DateTime<Utc>,
    pub retention: Retention,
}

#[derive(Clone, Debug, Default)]
pub struct Message {
    pub uuid: Uuid,
    pub topic_uuid: Uuid,
    pub author_name: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub key: Option<String>,
}


impl Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Topic '{}' ({})", self.name, self.uuid)?;
        writeln!(f, "\tCreator: {}", self.creator)?;
        write!(f, "\tTimestamp: {}", self.timestamp.with_timezone(&chrono::Local).format("%Y.%m.%d %H:%M:%S"))?;
        Ok(())
    }
}