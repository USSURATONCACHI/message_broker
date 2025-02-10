use std::net::SocketAddr;

use broker::concurrent_list::Chunk;
use broker::message_capnp::message;
use broker::util::{Handle, StoreRegistry};
use broker::message_capnp::message_service::{self, DeleteMessageParams, DeleteMessageResults, GetMessagesParams, GetMessagesResults, PostMessageParams};
use broker::message_capnp::message_service::PostMessageResults;
use capnp::{capability::Promise, Error};
use capnp_rpc::pry;
use chrono::Utc;
use uuid::Uuid;

use crate::datatypes::Message;
use crate::fillers::fill_capnp_message;
use crate::{datatypes::Topic, stores::{CrudStore, LoginStore}};


pub struct MessageService {
    peer: SocketAddr,

    login_store: Handle<LoginStore>,
    topic_store: Handle<CrudStore<Topic>>,
    messages: Handle<Chunk<Message>>,
}

impl MessageService {
    pub fn new(peer: SocketAddr, stores: &StoreRegistry) -> Self {
        Self {
            peer,
            login_store: stores.get(),
            topic_store: stores.get(),
            messages: stores.get(),
        }
    }
}


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
    
    fn delete_message(&mut self, params: DeleteMessageParams, mut results: DeleteMessageResults) -> Promise<(), Error> { 
        let username = pry!(self.login_store.get().check_login(&self.peer));
        todo!("Messages deletion is not implemented yet");
        // Promise::ok(())
    }
    
    fn get_messages(&mut self, params: GetMessagesParams, mut results: GetMessagesResults) -> Promise<(), Error> { 
        let username = pry!(self.login_store.get().check_login(&self.peer));
        todo!();
        Promise::ok(())
    }
    
}