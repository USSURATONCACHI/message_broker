mod services;
mod stores;
mod datatypes;
mod fillers;

use std::net::SocketAddr;
use std::sync::Arc;
use broker::concurrent_list::ConcurrentList;
use capnp_rpc::RpcSystem;
use tokio::net::{TcpStream, TcpListener};
use tokio::sync::Notify;

use broker::util::{stream_to_rpc_network, Handle, StoreRegistry};
use broker::main_capnp::root_service;

use services::{AuthService, MessageService, RootService, TopicService};
use datatypes::{Topic, Message};
use stores::{CrudStore, LoginStore};
use tokio::task;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let server = Arc::new(Server::new());
    server.set_interrupt_handler();

    let addr = "127.0.0.1:8080";

    println!("Starting the server on `{addr}`...");
    task::LocalSet::new().run_until(
        server.listen(addr)
    ).await?;
    println!("Server stopped.");

    Ok(())
}

struct Server {
    interrupt: Notify,
    stores: StoreRegistry,
}

impl Server {
    fn new() -> Self {
        let mut stores = StoreRegistry::new();

        let messages = Arc::new(ConcurrentList::<Message>::new(256));
        stores.add(messages);
        stores.add(Handle::<LoginStore>::new());
        stores.add(Handle::<CrudStore<Topic>>::new());

        Self {
            interrupt: Notify::new(),
            stores,
        }
    }

    async fn listen(self: Arc<Self>, addr: &str) -> std::io::Result<()> {
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
    fn set_interrupt_handler(self: &Arc<Self>) {
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