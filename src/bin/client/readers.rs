use broker::{message_capnp::{self}, topic_capnp::topic, util_capnp};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::datatypes::{Message, Topic};

pub fn read_capnp_uuid(reader: util_capnp::uuid::Reader<'_>) -> Uuid {
    Uuid::from_u64_pair(
        reader.get_upper(), 
        reader.get_lower()
    )
}

pub fn read_capnp_message(reader: message_capnp::message::Reader<'_>) -> Result<Message, capnp::Error> {

    Ok(Message {
        uuid: read_capnp_uuid(reader.reborrow().get_uuid()?),
        topic_uuid: read_capnp_uuid(reader.reborrow().get_topic_uuid()?),
        author_name: reader.reborrow().get_author_name()?.to_string()?,
        content: reader.reborrow().get_content()?.to_string()?,
        timestamp: read_capnp_timestamp(reader.reborrow().get_timestamp()?),
        key: None,
    })
}

pub fn read_capnp_timestamp(reader: util_capnp::timestamp::Reader<'_>) -> DateTime<Utc> {
    DateTime::from_timestamp(reader.get_seconds(), reader.get_nanos()).unwrap()
}

pub fn read_capnp_topic(reader: topic::Reader<'_>) -> Result<Topic, capnp::Error> {
    Ok(Topic {
        uuid: read_capnp_uuid(reader.get_uuid()?),
        name: reader.get_name()?.to_string()?,
        creator: reader.get_owner_username()?.to_string()?,
        timestamp: read_capnp_timestamp(reader.get_created_at()?),
    })
}
