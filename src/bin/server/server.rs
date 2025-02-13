use std::net::SocketAddr;
use std::sync::Arc;
use broker::concurrent_list::ConcurrentList;
use capnp_rpc::RpcSystem;
use serde::ser::SerializeStruct;
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::Notify;
use serde::{Serialize, Serializer, Deserialize, Deserializer};

use broker::util::{stream_to_rpc_network, Handle, StoreRegistry};
use broker::main_capnp::root_service;

use crate::services::{AuthService, MessageService, RootService, TopicService};
use crate::datatypes::{Topic, Message};
use crate::stores::{CrudStore, LoginStore};

pub struct Server {
    interrupt: Notify,
    stores: StoreRegistry,

    messages: ConcurrentList<Message>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Self {
        let topics = CrudStore::<Topic>::default();
        let messages = ConcurrentList::<Message>::default();
        Self::from_messages(topics, messages)
    }

    pub fn from_messages(topics: CrudStore<Topic>, messages: ConcurrentList<Message>) -> Self {
        let mut stores = StoreRegistry::new();

        let topics = Handle::from(topics);

        stores.add(messages.reference());
        stores.add(topics);
        stores.add(Handle::<LoginStore>::new());

        Self {
            interrupt: Notify::new(),
            stores,
            messages,
        }
    }

    pub async fn listen(self: Arc<Self>, addr: &str) -> std::io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        let mut connections = vec!();
        
        loop {
            tokio::select! {
                _ = self.interrupt.notified() => {
                    eprintln!("Stopping the listener.");
                    break;
                }
                accepted = listener.accept() => {
                    match accepted {
                        Err(e) => eprintln!("Failed to accept connection: {e}"),
                        Ok((stream, addr)) => {
                            let process_fut = self.clone().process_connection(stream, addr);
                            // let send = ; TODO: ?????
                            let spawn = tokio::task::spawn_local(process_fut);
                            connections.push(spawn);
                        }
                    };
                }
            }
        }

        println!("Aborting {} coroutines...", connections.len());
        for conn in connections {
            conn.abort();
        }
        println!("Aborted.");

        Ok(())
    }

    async fn process_connection(self: Arc<Self>, stream: TcpStream, addr: SocketAddr) {
        println!("Accepted connection from addr: {addr}");

        // Services
        let auth = AuthService::new(addr, &self.stores);
        let topic = TopicService::new(addr, &self.stores);
        let message = MessageService::new(addr, &self.stores);

        // Root service
        let root = RootService {
            auth: capnp_rpc::new_client(auth), 
            topic: capnp_rpc::new_client(topic),
            message: capnp_rpc::new_client(message),
        };
        let root_client: root_service::Client = capnp_rpc::new_client(root);

        // Network
        let network = stream_to_rpc_network(stream);
        let rpc_system = RpcSystem::new(Box::new(network), Some(root_client.client));
        
        // Launch
        tokio::task::spawn_local(rpc_system).await.unwrap().unwrap();
        println!("Peer {addr} disconnected");
    }

    fn interrupt(&self) {
        self.interrupt.notify_waiters();
    }
}

impl Server {
    pub fn set_interrupt_handler(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);

        let result = ctrlc::set_handler(
            move || match weak.upgrade() {
                Some(server) => server.interrupt(),
                None => eprintln!("Server no longer exists, nothing to interrupt."),
            }
        );

        match result {
            Ok(_) => {},

            Err(ctrlc::Error::NoSuchSignal(signal_type)) => 
                eprintln!("Signal {signal_type:?} not found, CTRL + C interrupt will not be handled gracefully."),

            Err(ctrlc::Error::MultipleHandlers) =>
                eprintln!("CTRL + C interrupt already has a handler, interrupt may not be handled gracefully."),

            Err(ctrlc::Error::System(err)) =>
                eprintln!("CTRL + C interrupt not set, interrupt may not be handled gracefully. Reason: {err}."),
        }
    }
}

impl Serialize for Server {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Get read access to the topics store
        let topics_handle = self.stores.get::<Handle<CrudStore<Topic>>>();
        let topics_store = topics_handle.get();
        
        // Create a serializer for our two data fields
        let mut state = serializer.serialize_struct("Server", 2)?;
        state.serialize_field("messages", &self.messages)?;
        state.serialize_field("topics", &*topics_store)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Server {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Temporary structure to capture serialized fields
        #[derive(Deserialize)]
        struct ServerData {
            messages: ConcurrentList<Message>,
            topics: CrudStore<Topic>,
        }

        let data = ServerData::deserialize(deserializer)?;
        
        // Create new server instance with default stores
        Ok(Server::from_messages(data.topics, data.messages))
    }
}