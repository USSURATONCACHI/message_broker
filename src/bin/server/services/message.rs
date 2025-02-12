use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, Weak};
use std::time::Duration;

use broker::concurrent_list::{ChunkReadGuard, ChunkRef, ConcurrentList};
use broker::message_capnp::message_receiver;
use broker::util::{Handle, StoreRegistry};
use broker::message_capnp::message_service::{self, DeleteMessageParams, DeleteMessageResults, GetMessagesSyncParams, GetMessagesSyncResults, PostMessageParams, SubscribeParams, SubscribeResults, UnsubscribeParams, UnsubscribeResults};
use broker::message_capnp::message_service::PostMessageResults;
use capnp::{capability::Promise, Error};
use capnp_rpc::pry;
use chrono::Utc;
use uuid::Uuid;

use crate::datatypes::Message;
use crate::fillers::{fill_capnp_message, fill_capnp_uuid};
use crate::{datatypes::Topic, stores::{CrudStore, LoginStore}};


pub struct MessageService {
    peer: SocketAddr,

    login_store: Handle<LoginStore>,
    topic_store: Handle<CrudStore<Topic>>,

    messages_reader: ChunkRef<Message>,
    messages_writer: ChunkRef<Message>,
    subscribers: Vec<Arc<(Uuid, message_receiver::Client)>>,
}

impl MessageService {
    pub fn new(peer: SocketAddr, stores: &StoreRegistry) -> Self {
        let messages_handle = stores.get::<Arc<ConcurrentList<Message>>>().reference();

        Self {
            peer,
            login_store: stores.get::<Handle<LoginStore>>().clone(),
            topic_store: stores.get::<Handle<CrudStore<Topic>>>().clone(),

            messages_reader: messages_handle.clone(),
            messages_writer: messages_handle,

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
            results.get().init_messages().init_err().set_topic_doesnt_exist(());
            return Promise::ok(());
        }

        // We need to know amount of messages beforehand
        let filter_fn=  |msg: &ChunkReadGuard<'_, Message>| msg.topic_uuid == topic_uuid;

        self.messages_reader.go_to_back_node();
        let messages_count = self.messages_reader.clone().filter(filter_fn).count();

        // Return all the messages
        let mut builder = results.get().init_messages().initn_ok(messages_count as u32);

        for (index, message) in self.messages_reader.clone().filter(filter_fn).enumerate()
        {
            let capnp_message = builder.reborrow().get(index as u32);
            fill_capnp_message(capnp_message, message.deref());
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
            results.get().init_messages().init_err().set_topic_doesnt_exist(());
            return Promise::ok(());
        }

        let receiver = pry!(reader.get_receiver());

        async fn spin_on_messages(mut messages_reader: ChunkRef<Message>, uuid_receiver: Weak<(Uuid, message_receiver::Client)>) {
            messages_reader.go_to_front_node();

            loop {
                if let None = messages_reader.next() {
                    break;
                }
            } 

            'main: loop {
                let (topic_uuid, receiver) = match uuid_receiver.upgrade() {
                    Some(arc) => (arc.0, (&arc.1).clone()),
                    None => {
                        break
                    },
                };

                while let Some(next) = messages_reader.next() {
                    let deref = next.deref_option();
                    if let Some(message) = deref {
                        if message.topic_uuid != topic_uuid {
                            continue;
                        }

                        let mut request = receiver.receive_request();
                        let mut builder = request.get();
                        fill_capnp_uuid(builder.reborrow().init_topic(), message.topic_uuid);
                        fill_capnp_message(builder.reborrow().init_message(), message);
                        if let Err(err) = request.send().await {
                            println!("Server send error: {err:?}");
                            break 'main;
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(16)).await;
            }
        }
        
        let arc=  Arc::new((topic_uuid, receiver));
        let weak = Arc::downgrade(&arc);
        self.subscribers.push(arc);
        let future = spin_on_messages(self.messages_reader.clone(), weak);
        tokio::task::spawn_local(future);

        results.get().init_messages().init_ok();

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