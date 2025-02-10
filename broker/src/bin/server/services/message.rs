use std::net::SocketAddr;

use broker::{message_capnp::message_service::{self, DeleteMessageParams, DeleteMessageResults, GetMessagesParams, GetMessagesResults, PostMessageParams, PostMessageResults}, topic_capnp::topic_service::{CreateTopicParams, CreateTopicResults, DeleteTopicParams, DeleteTopicResults, GetAllTopicsParams, GetAllTopicsResults, GetTopicParams, GetTopicResults, UpdateTopicParams, UpdateTopicResults}, util::{Handle, StoreRegistry}};
use capnp::{capability::Promise, Error};
use capnp_rpc::pry;

use crate::{datatypes::Topic, stores::{CrudStore, LoginStore}};


pub struct MessageService {
    peer: SocketAddr,

    login_store: Handle<LoginStore>,
    topic_store: Handle<CrudStore<Topic>>,
    messages: Handle<()>,
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
pub trait Server<>   {
}
impl message_service::Server for MessageService {
    fn post_message(&mut self, params: PostMessageParams, mut results: PostMessageResults) -> Promise<(), Error> { 
        let username = pry!(self.login_store.get().check_login(&self.peer));
        todo!();
        Promise::ok(())
    }
    
    fn delete_message(&mut self, params: DeleteMessageParams, mut results: DeleteMessageResults) -> Promise<(), Error> { 
        let username = pry!(self.login_store.get().check_login(&self.peer));
        todo!();
        Promise::ok(())
    }
    
    fn get_messages(&mut self, params: GetMessagesParams, mut results: GetMessagesResults) -> Promise<(), Error> { 
        let username = pry!(self.login_store.get().check_login(&self.peer));
        todo!();
        Promise::ok(())
    }
    
}