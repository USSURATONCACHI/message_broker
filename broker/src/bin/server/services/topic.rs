use std::net::SocketAddr;

use broker::{topic_capnp::topic_service::{CreateTopicParams, CreateTopicResults, DeleteTopicParams, DeleteTopicResults, GetAllTopicsParams, GetAllTopicsResults, GetTopicParams, GetTopicResults, UpdateTopicParams, UpdateTopicResults}, util::{Handle, StoreRegistry}};
use broker::topic_capnp::topic_service;
use capnp::{capability::Promise, Error};
use capnp_rpc::pry;
use chrono::{DateTime, Timelike, Utc};
use uuid::Uuid;

use crate::{datatypes::Topic, stores::{CrudStore, LoginStore}};

pub struct TopicService {
    peer: SocketAddr,

    login_store: Handle<LoginStore>,
    topic_store: Handle<CrudStore<Topic>>,
}

impl TopicService {
    pub fn new(peer: SocketAddr, stores: &StoreRegistry) -> Self {
        Self {
            peer,
            login_store: stores.get::<LoginStore>(),
            topic_store: stores.get::<CrudStore<Topic>>(),
        }
    }
}

pub fn fill_capnp_timestamp(mut builder: broker::util_capnp::timestamp::Builder, timestamp: DateTime<Utc>) {
    let seconds = timestamp.timestamp();
    let nanos = timestamp.nanosecond();
    builder.set_seconds(seconds);
    builder.set_nanos(nanos);
}
pub fn fill_capnp_uuid(mut builder: broker::util_capnp::uuid::Builder, uuid: Uuid) {
    let (lower, upper) = uuid.as_u64_pair();
    builder.set_lower(lower);
    builder.set_upper(upper);
}

#[allow(dead_code, unused_variables, unused_mut)]
impl topic_service::Server for TopicService {
    fn create_topic(&mut self, params: CreateTopicParams, mut results: CreateTopicResults) -> Promise<(), Error> { 
        let username = pry!(self.login_store.get().check_login(&self.peer));
        let now = Utc::now();
        let name = pry!(pry!(pry!(params.get()).get_name()).to_string());

        if self.topic_store.get().count(|topic| topic.name == name) > 0 {
            // AlreadyExists
            results.get().init_topic().init_err().set_already_exists(());
            return Promise::ok(());
        }

        let new_topic = Topic {
            name,
            creator: username,
            timestamp: now,
        };

        let uuid = self.topic_store.get_mut().create(new_topic.clone());
    
        let mut capnp_topic = results.get().init_topic().init_ok();

        capnp_topic.set_name(new_topic.name);
        capnp_topic.set_owner_username(new_topic.creator);
        fill_capnp_uuid(capnp_topic.reborrow().init_uuid(), uuid);
        fill_capnp_timestamp(capnp_topic.reborrow().init_created_at(), new_topic.timestamp);
        
        Promise::ok(())
    }

    fn get_topic(&mut self, params: GetTopicParams, mut results: GetTopicResults) -> Promise<(), Error> { 
        let username = pry!(self.login_store.get().check_login(&self.peer));
        Promise::ok(())
    }

    fn get_all_topics(&mut self, params: GetAllTopicsParams, mut results: GetAllTopicsResults) -> Promise<(), Error> { 
        let username = pry!(self.login_store.get().check_login(&self.peer));
        Promise::ok(())
    }

    fn update_topic(&mut self, params: UpdateTopicParams, mut results: UpdateTopicResults) -> Promise<(), Error> { 
        let username = pry!(self.login_store.get().check_login(&self.peer));
        Promise::ok(())
    }

    fn delete_topic(&mut self, params: DeleteTopicParams, mut results: DeleteTopicResults) -> Promise<(), Error> { 
        let username = pry!(self.login_store.get().check_login(&self.peer));
        Promise::ok(())
    }

}