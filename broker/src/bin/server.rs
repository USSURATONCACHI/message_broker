use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Weak;
use std::time::Duration;

use broker::util::stream_to_rpc_network;
use broker::util::SendFuture;
use capnp::capability::Promise;
use capnp_rpc::pry;
use capnp_rpc::RpcSystem;
use tokio::sync::Notify;
use tokio::net::TcpStream;
use tokio::net::TcpListener;


use broker::schema_capnp::echo;
use broker::schema_capnp::ping_receiver;
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

impl echo::Server for EchoImpl {
    fn echo(&mut self, params: echo::EchoParams, mut results: echo::EchoResults) -> Promise<(), capnp::Error> {
        let request = pry!(pry!(params.get()).get_request());

        let in_message = pry!(pry!(request.get_message()).to_str());
        let message = format!("Hello, {in_message}!");
        
        results.get().init_reply().set_message(message);

        println!("Peer {} said: `{in_message}`", self.peer);

        Promise::ok(())
    }

    fn subscribe_to_pings(&mut self, params: echo::SubscribeToPingsParams, _: echo::SubscribeToPingsResults) -> Promise<(), capnp::Error> {
        let receiver: ping_receiver::Client = pry!(pry!(params.get()).get_receiver() );

        println!("Peer {} tried to subscribe to pings.", self.peer);
        self.pings_receiver = Some(Arc::new(receiver));

        {
            let weak = Arc::downgrade(self.pings_receiver.as_ref().unwrap());
            async fn ping_while_alive(weak: Weak<ping_receiver::Client>) {
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                let mut seq = 0u64;

                loop {
                    interval.tick().await;

                    match weak.upgrade() {
                        Some(client) => {
                            let mut request = client.ping_request();
                            request.get().set_seq(seq);
                        
                            let reply = request.send().promise.await;
                            if reply.is_err() {
                                break;
                            }
                        },
                        None => break,
                    }

                    seq += 1;
                }
            }

            tokio::spawn(SendFuture::from(ping_while_alive(weak)));
        }

        Promise::ok(())
    }
}

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
    // _last_message: Mutex<String>,
}

impl Server {
    fn new() -> Self {
        Self {
            interrupt: Notify::new(),
            // last_message: Mutex::new("Start".to_string())
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

        let server_rpc = EchoImpl::new(addr);

        let network = stream_to_rpc_network(stream);
        let client: echo::Client = capnp_rpc::new_client(server_rpc);

        let rpc_system = RpcSystem::new(Box::new(network), Some(client.clone().client));
        
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