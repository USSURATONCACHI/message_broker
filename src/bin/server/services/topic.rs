use std::net::SocketAddr;

use broker::{topic_capnp::topic_service::{CreateTopicParams, CreateTopicResults, DeleteTopicParams, DeleteTopicResults, GetAllTopicsParams, GetAllTopicsResults, GetTopicParams, GetTopicResults, UpdateTopicParams, UpdateTopicResults}, util::{Handle, StoreRegistry}};
use broker::topic_capnp::topic_service;
use capnp::{capability::Promise, Error};
use capnp_rpc::pry;
use chrono::{Duration, Utc};

use crate::{datatypes::Topic, fillers::fill_capnp_topic, stores::{CrudStore, LoginStore}};


pub struct TopicService {
    peer: SocketAddr,

    login_store: Handle<LoginStore>,
    topic_store: Handle<CrudStore<Topic>>,
}

impl TopicService {
    pub fn new(peer: SocketAddr, stores: &StoreRegistry) -> Self {

        Self {
            peer,
            login_store: stores.get::<Handle<LoginStore>>().clone(),
            topic_store: stores.get::<Handle<CrudStore<Topic>>>().clone(),
        }
    }
}

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
            retention: None,
        };

        let uuid = self.topic_store.get_mut().create(new_topic.clone());
    
        let capnp_topic = results.get().init_topic().init_ok();
        fill_capnp_topic(capnp_topic, uuid, &new_topic);

        Promise::ok(())
    }

    fn get_topic(&mut self, params: GetTopicParams, mut results: GetTopicResults) -> Promise<(), Error> { 
        let _username = pry!(self.login_store.get().check_login(&self.peer));

        let uuid = pry!(pry!(params.get()).get_topic_id());
        let uuid = uuid::Uuid::from_u64_pair(uuid.get_upper(), uuid.get_lower());

        let topic = self.topic_store.get().get(uuid);

        match topic {
            None => {
                results.get().init_topic().init_err().set_not_found(());
            }

            Some(topic) => {
                let capnp_topic = results.get().init_topic().init_ok();
                fill_capnp_topic(capnp_topic, uuid, &topic);
            }
        }

        Promise::ok(())
    }

    fn get_all_topics(&mut self, _params: GetAllTopicsParams, mut results: GetAllTopicsResults) -> Promise<(), Error> { 
        let _username = pry!(self.login_store.get().check_login(&self.peer));

        let all_topics = self.topic_store.get().get_all();

        let mut capnp_list = results.get().init_topics(all_topics.len() as u32);
        for (index, (uuid, topic)) in all_topics.into_iter().enumerate() {
            let capnp_topic = capnp_list.reborrow().get(index as u32);
            fill_capnp_topic(capnp_topic, uuid, &topic);
        }

        Promise::ok(())
    }

    fn update_topic(&mut self, params: UpdateTopicParams, mut results: UpdateTopicResults) -> Promise<(), Error> { 
        let _username = pry!(self.login_store.get().check_login(&self.peer));

        let uuid = pry!(pry!(params.get()).get_topic_id());
        let uuid = uuid::Uuid::from_u64_pair(uuid.get_upper(), uuid.get_lower());
        
        let new_name = pry!(pry!(pry!(params.get()).get_name()).to_str()).trim();

        let new_retention = pry!(pry!(params.get()).get_retention());
        let new_retention: Option<Duration> = match pry!(new_retention.which()) {
            broker::topic_capnp::retention::Which::None(()) => None,
            broker::topic_capnp::retention::Which::Minutes(minutes) => {
                let duration = Duration::from_std(std::time::Duration::from_secs_f64(minutes * 60.0));
                Some(pry!(duration.map_err(|err| capnp::Error::failed(err.to_string()))))
            }
        };

        let topic = self.topic_store.get().get(uuid);

        match topic {
            None => {
                results.get().init_topic().init_err().set_not_found(());
            }

            Some(mut current_topic) => {
                if current_topic.name != new_name && self.topic_store.get().count(|t| t.name == new_name) > 0 {
                    results.get().init_topic().init_err().set_already_exists(());
                } else {
                    let capnp_topic = results.get().init_topic().init_ok();
                    current_topic.name = new_name.into();
                    current_topic.retention = new_retention;

                    fill_capnp_topic(capnp_topic, uuid, &current_topic);
                    self.topic_store.get_mut().update(uuid, current_topic);
                }
            }
        }

        Promise::ok(())
    }

    fn delete_topic(&mut self, params: DeleteTopicParams, mut results: DeleteTopicResults) -> Promise<(), Error> { 
        let _username = pry!(self.login_store.get().check_login(&self.peer));

        let uuid = pry!(pry!(params.get()).get_topic_id());
        let uuid = uuid::Uuid::from_u64_pair(uuid.get_upper(), uuid.get_lower());

        let topic = self.topic_store.get().get(uuid);

        match topic {
            None => {
                results.get().init_result().init_err().set_not_found(());
            }

            Some(_) => {
                self.topic_store.get_mut().remove(uuid);
                results.get().init_result().init_ok();
            }
        }

        Promise::ok(())
    }
}