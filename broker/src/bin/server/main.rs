use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Weak;
use std::time::Duration;

use broker::util::stream_to_rpc_network;
use broker::util::Handle;
use broker::util::SendFuture;
use broker::util::StoreRegistry;
use capnp::capability::Promise;
use capnp_rpc::pry;
use capnp_rpc::RpcSystem;
use services::AuthService;
use services::EchoService;
use services::RootService;
use stores::LoginStore;
use tokio::sync::Notify;
use tokio::net::TcpStream;
use tokio::net::TcpListener;

use broker::echo_capnp::echo;
use broker::main_capnp::root_service;
use broker::echo_capnp::ping_receiver;

mod services;
mod stores;

struct EchoImpl {
    peer: SocketAddr,
    pings_receiver: Option<Arc<ping_receiver::Client>>, 
}

impl EchoImpl {
    pub fn new(peer: SocketAddr) -> Self {
        Self {
            peer,
            pings_receiver: None,
        }
    }
}

// fn subscribe_to_pings(&mut self, _: SubscribeToPingsParams<>, _: SubscribeToPingsResults<>) -> ::capnp::capability::Promise<(), ::capnp::Error> { ::capnp::capability::Promise::err(::capnp::Error::unimplemented("method echo::Server::subscribe_to_pings not implemented".to_string())) }

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let server = Arc::new(Server::new());
    server.set_interrupt_handler();

    let addr = "127.0.0.1:8080";

    println!("Starting the server on `{addr}`...");
    server.listen(addr).await?;
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

        stores.add(Handle::<LoginStore>::new());

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
                            let send = SendFuture::from(process_fut);
                            connections.push(tokio::spawn(send));
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
        let echo = EchoService::new(addr, &self.stores);

        let root = RootService {
            auth: capnp_rpc::new_client(auth), 
            echo: capnp_rpc::new_client(echo), 
        };
        let root_client: root_service::Client = capnp_rpc::new_client(root);

        // Network
        let network = stream_to_rpc_network(stream);
        let rpc_system = RpcSystem::new(Box::new(network), Some(root_client.client));
        
        SendFuture::from(rpc_system).await.unwrap();
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