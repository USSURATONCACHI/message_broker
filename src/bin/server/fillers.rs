use broker::message_capnp::message;
use chrono::{DateTime, Timelike, Utc};
use uuid::Uuid;

use crate::datatypes::{Message, Retention, Topic};


pub fn fill_capnp_message(mut builder: message::Builder<'_>, message: &Message) {
    builder.set_author_name(&message.author_name);
    builder.set_content(&message.content);
    fill_capnp_timestamp(builder.reborrow().init_timestamp(), message.timestamp);
    fill_capnp_uuid(builder.reborrow().init_topic_uuid(), message.topic_uuid);
    fill_capnp_uuid(builder.init_uuid(), message.uuid);
}

pub fn fill_capnp_timestamp(mut builder: broker::util_capnp::timestamp::Builder, timestamp: DateTime<Utc>) {
    let seconds = timestamp.timestamp();
    let nanos = timestamp.nanosecond();
    builder.set_seconds(seconds);
    builder.set_nanos(nanos);
}

pub fn fill_capnp_uuid(mut builder: broker::util_capnp::uuid::Builder, uuid: Uuid) {
    let (upper, lower) = uuid.as_u64_pair();
    builder.set_lower(lower);
    builder.set_upper(upper);
}

pub fn fill_capnp_retention(mut builder: broker::topic_capnp::retention::Builder, retention: Retention) {
    match retention {
        Some(duration) => builder.set_minutes(duration.num_seconds() as f64 / 60.0),
        None => builder.set_none(()),
    }
}

pub fn fill_capnp_topic(mut builder: broker::topic_capnp::topic::Builder, uuid: Uuid, topic: &Topic) {
    builder.set_name(&topic.name);
    builder.set_owner_username(&topic.creator);
    fill_capnp_uuid(builder.reborrow().init_uuid(), uuid);
    fill_capnp_timestamp(builder.reborrow().init_created_at(), topic.timestamp);
    fill_capnp_retention(builder.reborrow().init_retention(), topic.retention);
}