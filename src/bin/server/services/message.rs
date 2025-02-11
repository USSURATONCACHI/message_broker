use std::collections::HashMap;
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, RwLock, Weak};

use broker::concurrent_list::Chunk;
use broker::message_capnp::message_receiver;
use broker::util::{Handle, SendFuture, StoreRegistry};
use broker::message_capnp::message_service::{self, DeleteMessageParams, DeleteMessageResults, GetMessagesSyncParams, GetMessagesSyncResults, PostMessageParams, SubscribeParams, SubscribeResults, UnsubscribeParams, UnsubscribeResults};
use broker::message_capnp::message_service::PostMessageResults;
use capnp::{capability::Promise, Error};
use capnp_rpc::pry;
use chrono::Utc;
use uuid::Uuid;

use crate::datatypes::Message;
use crate::fillers::fill_capnp_message;
use crate::services::topic;
use crate::{datatypes::Topic, stores::{CrudStore, LoginStore}};


type Receivers = HashMap<Uuid, message_receiver::Client>;

pub struct MessageService {
    peer: SocketAddr,

    login_store: Handle<LoginStore>,
    topic_store: Handle<CrudStore<Topic>>,
    messages: Handle<Chunk<Message>>,

    subscribers: Arc<RwLock<Receivers>>,
}

impl MessageService {
    pub fn new(peer: SocketAddr, stores: &StoreRegistry) -> Self {
        let messages = stores.get();
        // let subscribers = Default::default();

        // tokio::spawn(SendFuture::from(future));

        Self {
            peer,
            login_store: stores.get(),
            topic_store: stores.get(),
            messages,
            subscribers: Default::default(),
        }
    }
}

// async fn listen_to_messages(messages: Weak<RwLock<Chunk<Message>>>, listeners: Weak<RwLock<Receivers>>) {
//     loop {
//         let messages = match messages.upgrade() {
//             Some(m) => m,
//             None => break,
//         };
//         let listeners = match listeners.upgrade() {
//             Some(l) => l,
//             None => break,
//         };

//         for message in messages.read().unwrap().


//     }
// }


impl message_service::Server for MessageService {
    fn post_message(&mut self, params: PostMessageParams, mut results: PostMessageResults) -> Promise<(), Error> { 
        let username = pry!(self.login_store.get().check_login(&self.peer));
        let reader = pry!(params.get());

        let topic_uuid = pry!(reader.get_topic_id());
        let topic_uuid = Uuid::from_u64_pair(topic_uuid.get_upper(), topic_uuid.get_lower());

        let content = pry!(pry!(reader.get_content()).to_str()).trim();

        // Check valid content
        if content.is_empty() {
            results.get().init_message().init_err().set_invalid_content(());
            return Promise::ok(());
        }

        // Check that topic exists
        println!("Checking out topic uuid: {topic_uuid}");
        if self.topic_store.get().get(topic_uuid).is_none() {
            results.get().init_message().init_err().set_topic_doesnt_exist(());
            return Promise::ok(());
        }

        let message = Message {
            uuid: Uuid::new_v4(),
            topic_uuid,
            author_name: username,
            content: content.to_owned(),
            timestamp: Utc::now(),
        };

        // Fill message response
        let capnp_message = results.get().init_message().init_ok();
        fill_capnp_message(capnp_message, &message);

        // Push the message to DB-like structure
        self.messages.get().push(message);

        Promise::ok(())
    }
    
    fn delete_message(&mut self, _params: DeleteMessageParams, mut _results: DeleteMessageResults) -> Promise<(), Error> { 
        let _username = pry!(self.login_store.get().check_login(&self.peer));
        todo!("Messages deletion is not implemented yet");
        // Promise::ok(())
    }
    
    fn get_messages_sync(&mut self, params: GetMessagesSyncParams, mut results: GetMessagesSyncResults) -> Promise<(), Error> { 
        let _username = pry!(self.login_store.get().check_login(&self.peer));

        let topic_uuid = pry!(pry!(params.get()).get_topic_id());
        let topic_uuid = Uuid::from_u64_pair(topic_uuid.get_upper(), topic_uuid.get_lower());
        
        // Check that topic exists
        if self.topic_store.get().get(topic_uuid).is_none() {
            results.get().init_messages().init_err().set_topic_doesnt_exist(());
            return Promise::ok(());
        }

        // We need to know amount of messages beforehand
        let count = self.messages.get()
            .iter()
            .filter(|msg| msg.topic_uuid == topic_uuid)
            .count();

        // Return all the messages
        let mut builder = results.get().init_messages().initn_ok(count as u32);
        for (index, message) in self.messages
            .get()
            .iter()
            .filter(|msg| msg.topic_uuid == topic_uuid)
            .enumerate()
        {
            let capnp_message = builder.reborrow().get(index as u32);
            fill_capnp_message(capnp_message, message.deref());
        } 
        
        Promise::ok(())
    }
    
    fn subscribe(&mut self, params: SubscribeParams, mut resulsts: SubscribeResults) -> Promise<(), Error> {
        let _username = pry!(self.login_store.get().check_login(&self.peer));
        let reader = pry!(params.get());

        let topic_uuid = pry!(reader.get_topic_id());
        let topic_uuid = Uuid::from_u64_pair(topic_uuid.get_upper(), topic_uuid.get_lower());

        let receiver = pry!(reader.get_receiver());



        todo!()
    }
    
    fn unsubscribe(&mut self, params: UnsubscribeParams, mut results: UnsubscribeResults) ->  Promise<(), Error>{
        todo!()
    }
}