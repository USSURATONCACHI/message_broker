use broker::{auth_capnp::auth_service, main_capnp::root_service, message_capnp::{message_receiver, message_service, reverse_message_iterator}, topic_capnp::topic_service, util_capnp};
use capnp::Error;
use uuid::Uuid;

use crate::{datatypes::{Message, Topic}, message_receiver_impl::MessageReceiver, readers::{read_capnp_message, read_capnp_topic, read_capnp_topic_error}};



pub async fn autorize(root: &root_service::Client, username: &str) -> Result<(), capnp::Error> {
    let auth_service = get_auth_client(&root).await?;

    let mut login_request = auth_service.login_request();
    login_request.get().set_username(username);

    let _ = login_request.send().promise.await?;

    Ok(())
}

pub async fn post_message(message_service: &message_service::Client, content: &str, topic_uuid: Uuid, key: Option<&str>) -> Result<Message, capnp::Error> {
    let mut request = message_service.post_message_request();

    let mut builder = request.get();
    builder.set_content(content);

    let mut capnp_uuid = builder.reborrow().init_topic_id();
    let (upper, lower) = topic_uuid.as_u64_pair();
    capnp_uuid.set_upper(upper);
    capnp_uuid.set_lower(lower);

    if let Some(key) = key {
        builder.init_key().set_t(key)?;
    }

    let response = request.send().promise.await?;
    let response = response.get()?.get_message()?;
    
    match response.which()? {
        util_capnp::result::Which::Err(err) => {
            let err = err?;
            let err_message = match err.which()? {
                message_service::error::Which::EntityDoesNotExist(()) => "Topic does not exist",
                message_service::error::Which::InvalidContent(()) => "Invalid content",
            };
            Err(capnp::Error::failed(err_message.to_owned()))
        },
        util_capnp::result::Which::Ok(ok) => {
            let ok = ok?;
            Ok(read_capnp_message(ok)?)
        },
    }
} 

pub async fn get_all_topics(topic: &topic_service::Client) -> Result<Vec<Topic>, capnp::Error> {
    topic.get_all_topics_request()
        .send().promise.await?
        .get()?
        .get_topics()?
        .iter()
        .map(read_capnp_topic)
        .collect::<Result<Vec<_>, _>>()
}

pub async fn get_auth_client(root: &root_service::Client) -> Result<auth_service::Client, capnp::Error> {
    Ok(
        root.auth_request()
            .send().promise
            .await?
            .get()?
            .get_service()?
    )
}
pub async fn get_topic_client(root: &root_service::Client) -> Result<topic_service::Client, capnp::Error> {
    Ok(
        root.topic_request()
            .send().promise
            .await?
            .get()?
            .get_service()?
    )
}
pub async fn get_message_client(root: &root_service::Client) -> Result<message_service::Client, capnp::Error> {
    Ok(
        root.message_request()
            .send().promise
            .await?
            .get()?
            .get_service()?
    )
}

pub async fn create_topic(topic_service: &topic_service::Client, name: &str) -> Result<Topic, capnp::Error> {
    let mut request = topic_service.create_topic_request();
    request.get().set_name(name);
    
    let result = request.send().promise.await?;

    match result.get()?.get_topic()?.which()? {
        broker::util_capnp::result::Which::Ok(ok) => Ok(read_capnp_topic(ok?)?),
        broker::util_capnp::result::Which::Err(err) => {
            let name = match err?.which()? {
                topic_service::error::Which::NotFound(()) => "Topic does not exist.",
                topic_service::error::Which::AlreadyExists(()) => "Topic already exists (unreachable).",
            };
            Err(Error::failed(name.to_string()))
        },
    }
}

pub async fn get_messages_reverse(rev_message_iterator: &reverse_message_iterator::Client, count: u32) -> Result<Vec<Message>, capnp::Error> {
    let mut results = Vec::with_capacity(count as usize);

    let mut request = rev_message_iterator.next_request();
    request.get().set_count(count);

    let response = request.send().promise.await?;
    for element in response.get()?.get_messages()? {
        results.push(read_capnp_message(element)?);
    }

    Ok(results)
}

pub async fn subscribe_to_messages(message_service: &message_service::Client, receiver: MessageReceiver, topic_uuid: Uuid) -> Result<reverse_message_iterator::Client, capnp::Error> {
    let receiver_client: message_receiver::Client = capnp_rpc::new_client(receiver);

    let mut subscribe_request = message_service.subscribe_request();

    let mut builder = subscribe_request.get();
    builder.set_receiver(receiver_client);

    let mut uuid_builder = builder.init_topic_id();
    let (upper, lower) = topic_uuid.as_u64_pair();
    uuid_builder.set_upper(upper);
    uuid_builder.set_lower(lower);

    let response = subscribe_request.send().promise.await?;

    match response.get()?.get_messages()?.which()? {
        broker::util_capnp::result::Which::Ok(messages_iterator) => {
            let messages_iterator = messages_iterator?;
            Ok(messages_iterator)
        },

        broker::util_capnp::result::Which::Err(error) => {
            let error = error?;
            let err_message = match error.which()? {
                message_service::error::Which::EntityDoesNotExist(()) => "Topic does not exist",
                message_service::error::Which::InvalidContent(()) => "Invalid content (unreachable)",
            };
            Err(Error::failed(err_message.to_owned()))
        },
    }
}

pub async fn subscribe_and_get_messages(
    message_service: &message_service::Client, 
    topic: &Topic, 
    new_messages_action: impl 'static + FnMut(Message), 
    old_messages_limit: u32
) -> Result<Vec<Message>, capnp::Error> {
    let live_receiver = MessageReceiver::new(new_messages_action);
    let old_messages_iter = subscribe_to_messages(&message_service, live_receiver, topic.uuid).await?;

    let history = get_messages_reverse(&old_messages_iter, old_messages_limit).await?;
    Ok(history)
}

pub async fn update_topic(topic_service: &topic_service::Client, updated: &Topic) -> Result<Topic, capnp::Error> {
    let mut request = topic_service.update_topic_request();
    let mut builder = request.get(); 
    builder.set_name(&updated.name);
    
    let mut capnp_topic_id = builder.reborrow().init_topic_id();
    capnp_topic_id.set_upper(updated.uuid.as_u64_pair().0);
    capnp_topic_id.set_lower(updated.uuid.as_u64_pair().1);

    match updated.retention {
        Some(retention) => builder.init_retention().set_minutes(retention.num_seconds() as f64 / 60.0),
        None => builder.init_retention().set_none(()),
    }

    let response = request.send().promise.await?;

    let a = response.get()?.get_topic()?;
    match a.which()? {
        util_capnp::result::Which::Ok(ok) => {
            Ok(read_capnp_topic(ok?)?)
        },
        util_capnp::result::Which::Err(err) => {
            read_capnp_topic_error(err?)?;
            unreachable!();
        },
    }
}