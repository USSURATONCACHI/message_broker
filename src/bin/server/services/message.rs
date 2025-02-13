use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, RwLockReadGuard, Weak};
use std::time::Duration;

use broker::concurrent_list::ConcurrentListRef;
use broker::message_capnp::reverse_message_iterator::{NextParams, NextResults, StopParams, StopResults};
use broker::message_capnp::{message_receiver, reverse_message_iterator};
use broker::util::{Handle, ReverseIterator, StoreRegistry};
use broker::message_capnp::message_service::{self, DeleteMessageParams, DeleteMessageResults, GetMessagesSyncParams, GetMessagesSyncResults, PostMessageParams, SubscribeParams, SubscribeResults, UnsubscribeParams, UnsubscribeResults};
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

    messages_reader: ConcurrentListRef<Message>,
    messages_writer: ConcurrentListRef<Message>,
    subscribers: Vec<Arc<(Uuid, message_receiver::Client)>>,
    message_iterators: Vec<reverse_message_iterator::Client>,
}

impl MessageService {
    pub fn new(peer: SocketAddr, stores: &StoreRegistry) -> Self {
        let messages_handle = stores.get::<ConcurrentListRef<Message>>().clone();

        Self {
            peer,
            login_store: stores.get::<Handle<LoginStore>>().clone(),
            topic_store: stores.get::<Handle<CrudStore<Topic>>>().clone(),

            messages_reader: messages_handle.clone(),
            messages_writer: messages_handle,

            subscribers: Default::default(),
            message_iterators: Default::default(),
        }
    }
}

fn sanitize_text(input: &str) -> String {
    input.chars()
        .filter(|&c| {
            // Preserve tabs, spaces, and non-control characters
            c == '\t' || c == ' ' || !c.is_control()
        })
        // Optional: Add this line to remove ANSI escape sequences
        //.filter(|&c| c != '\x1B' && !('\x80'..='\x9F').contains(&c))
        .collect()
}

impl message_service::Server for MessageService {
    fn post_message(&mut self, params: PostMessageParams, mut results: PostMessageResults) -> Promise<(), Error> { 
        let username = pry!(self.login_store.get().check_login(&self.peer));
        let reader = pry!(params.get());

        let topic_uuid = pry!(reader.get_topic_id());
        let topic_uuid = Uuid::from_u64_pair(topic_uuid.get_upper(), topic_uuid.get_lower());

        let content = pry!(pry!(reader.get_content()).to_str()).trim();
        let content = sanitize_text(content);

        let key = pry!(reader.get_key());
        let key = if key.has_t() {
            Some(pry!(pry!(key.get_t()).to_string()))
        } else {
            None
        };

        // Check valid content
        if content.is_empty() {
            results.get().init_message().init_err().set_invalid_content(());
            return Promise::ok(());
        }

        // Check that topic exists
        if self.topic_store.get().get(topic_uuid).is_none() {
            results.get().init_message().init_err().set_entity_does_not_exist(());
            return Promise::ok(());
        }

        let message = Message {
            uuid: Uuid::new_v4(),
            topic_uuid,
            author_name: username,
            content: content.to_owned(),
            timestamp: Utc::now(),
            key,
        };

        // Fill message response
        let capnp_message = results.get().init_message().init_ok();
        fill_capnp_message(capnp_message, &message);

        // Push the message to DB-like structure
        self.messages_writer.push(message);

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
            results.get().init_messages().init_err().set_entity_does_not_exist(());
            return Promise::ok(());
        }

        // We need to know amount of messages beforehand
        let filter_fn = |msg: &RwLockReadGuard<'_, Option<Message>>| 
            msg.is_some() && 
            msg.as_ref().unwrap().topic_uuid == topic_uuid;

        self.messages_reader.drain_backwards();
        let messages_count = self.messages_reader.clone().filter(filter_fn).count();

        // Return all the messages
        let mut builder = results.get().init_messages().initn_ok(messages_count as u32);

        let messages = self.messages_reader.clone()
            .filter(filter_fn)
            .enumerate();
        for (index, message) in messages
        {
            let message = message.as_ref().unwrap();
            let capnp_message = builder.reborrow().get(index as u32);
            fill_capnp_message(capnp_message, message);
        } 
        
        Promise::ok(())
    }
    
    fn subscribe(&mut self, params: SubscribeParams, mut results: SubscribeResults) -> Promise<(), Error> {
        let _username = pry!(self.login_store.get().check_login(&self.peer));
        let reader = pry!(params.get());

        let topic_uuid = pry!(reader.get_topic_id());
        let topic_uuid = Uuid::from_u64_pair(topic_uuid.get_upper(), topic_uuid.get_lower());

        // Check that topic exists
        if self.topic_store.get().get(topic_uuid).is_none() {
            results.get().init_messages().init_err().set_entity_does_not_exist(());
            return Promise::ok(());
        }

        let mut reader_handle = self.messages_reader.clone();
        reader_handle.drain_forward();
        // Create an Arc-Weak pair of message receiver
        {
            let receiver_arc=  Arc::new((topic_uuid, pry!(reader.get_receiver())));
            let receiver_weak = Arc::downgrade(&receiver_arc);
            
            self.subscribers.push(receiver_arc); // `self` owns an Arc, task owns a Weak. This way, task will stop itself, `self` is dropped.

            tokio::task::spawn_local(spin_on_messages(reader_handle.clone(), receiver_weak));
        }

        // Create a message iterator (for user to request messages history.)
        {
            let message_iterator = ReverseMessageIterator::new(reader_handle, topic_uuid);
            let message_iterator: reverse_message_iterator::Client = capnp_rpc::new_client(message_iterator);
            self.message_iterators.push(message_iterator.clone()); 

            // Send the iterator to the client 
            pry!(results.get().init_messages().set_ok(message_iterator));
        }

        Promise::ok(())
    }
    
    fn unsubscribe(&mut self, params: UnsubscribeParams, mut _results: UnsubscribeResults) ->  Promise<(), Error>{
        let _username = pry!(self.login_store.get().check_login(&self.peer));
        let reader = pry!(params.get());

        let topic_uuid = pry!(reader.get_topic_id());
        let topic_uuid = Uuid::from_u64_pair(topic_uuid.get_upper(), topic_uuid.get_lower());
        let _receiver = pry!(reader.get_receiver());

        let indices = self.subscribers.iter().enumerate()
            .filter(|(_, elem)| elem.0 == topic_uuid)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();

        for idx in indices.into_iter().rev() {
            self.subscribers.remove(idx);
        }

        Promise::ok(())
    }
}

async fn spin_on_messages(mut messages_reader: ConcurrentListRef<Message>, uuid_receiver: Weak<(Uuid, message_receiver::Client)>) {
    'main: loop {
        let (topic_uuid, receiver) = match uuid_receiver.upgrade() {
            Some(arc) => (arc.0, (&arc.1).clone()),
            None => {
                break
            },
        };

        while let Some(next) = messages_reader.next() {
            let message = match next.deref() {
                None => break,
                Some(x) => x,
            };

            if message.topic_uuid != topic_uuid {
                continue;
            }

            let mut request = receiver.receive_request();
            fill_capnp_message(request.get().init_message(), message);
            if let Err(_err) = request.send().await {
                break 'main;
            }
        }
        // TODO
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

struct ReverseMessageIterator {
    messages_reader: Option<ConcurrentListRef<Message>>,
    topic_uuid: Uuid,
}

impl ReverseMessageIterator {
    pub fn new(handle: ConcurrentListRef<Message>, topic_uuid: Uuid) -> Self {
        Self {
            messages_reader: Some(handle),
            topic_uuid
        }
    }
}

impl reverse_message_iterator::Server for ReverseMessageIterator {
    fn next(&mut self, params: NextParams, mut results: NextResults) -> Promise<(), Error> {
        let count = pry!(params.get()).get_count();

        if self.messages_reader.is_none() {
            return Promise::err(Error::failed("Iterator was stopped".into()))
        }
        let reader = self.messages_reader.as_mut().unwrap();

        let reader = ReverseIterator::from(reader);
        let messages = reader
            .filter_map(|guard| guard.as_ref().cloned())
            .filter(|message| message.topic_uuid == self.topic_uuid)
            .take(count as usize)
            .collect::<Vec<_>>();

        let mut capnp_messages = results.get().init_messages(messages.len() as u32);
        for (i, message) in messages.into_iter().enumerate() {
            fill_capnp_message(capnp_messages.reborrow().get(i as u32), &message);
        }

        Promise::ok(())
    }
    fn stop(&mut self, _params: StopParams, _results: StopResults) -> Promise<(), Error> {
        self.messages_reader = None;
        Promise::ok(())
    }
}