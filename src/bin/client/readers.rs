use broker::{message_capnp::{self}, topic_capnp::{self, topic, topic_service}, util_capnp};
use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::datatypes::{Message, Retention, Topic};

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
        retention: read_capnp_retention(reader.get_retention()?)?,
    })
}

pub fn read_capnp_retention(reader: topic_capnp::retention::Reader<'_>) -> Result<Retention, capnp::Error> {
    match reader.which()? {
        topic_capnp::retention::Which::None(()) => Ok(None),
        topic_capnp::retention::Which::Minutes(mins) => Ok(Some(Duration::seconds((mins * 60.0) as i64))),
    }
}

pub fn read_capnp_topic_error(topic_reader: topic_service::error::Reader<'_>) -> Result<(), capnp::Error> {
    let err_message = match topic_reader.which()? {
        topic_service::error::Which::NotFound(()) => "Topic does not exist",
        topic_service::error::Which::AlreadyExists(()) => "Topic already exists",
    };
    Err(capnp::Error::failed(err_message.to_owned()))
}