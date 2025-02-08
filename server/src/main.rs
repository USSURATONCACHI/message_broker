use std::future::Future;
use std::io::Error;
use std::net::SocketAddr;
use std::ops::DerefMut;
use std::pin::pin;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::Arc;
use std::time::Duration;

use capnp::capability::Promise;
use capnp_rpc::rpc_twoparty_capnp;
use capnp_rpc::twoparty;
use capnp_rpc::RpcSystem;
use tokio::io::AsyncWriteExt;
use tokio::sync::Notify;
use tokio::net::TcpStream;
use tokio::net::TcpListener;
use tokio::time::sleep;
use tokio_util::compat::FuturesAsyncWriteCompatExt;
use tokio_util::compat::TokioAsyncReadCompatExt;
use tokio_util::compat::TokioAsyncWriteCompatExt;


mod schema_capnp {
    include!(concat!(env!("OUT_DIR"), "/schema_capnp.rs"));
}


use crate::schema_capnp::echo;
struct EchoImpl;

impl echo::Server for EchoImpl {
    fn echo(&mut self, params: echo::EchoParams, mut results: echo::EchoResults) -> Promise<(), capnp::Error> {
        todo!()
    }
}


struct SendFuture<F: Future> {
    inner: Arc<Mutex<F>>,
}

impl<F: Future + Unpin> From<F> for SendFuture<F> {
    fn from(value: F) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value))
        }
    }
}

unsafe impl<F: Future + Unpin> Send for SendFuture<F> {}

impl<F: Future + Unpin> Future for SendFuture<F> {
    type Output = F::Output;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let mut lock = self.inner.lock().unwrap();
        let mutref = lock.deref_mut();
        let pin = Pin::new(mutref);
        pin.poll(cx)
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
    last_message: Mutex<String>,
}

impl Server {
    fn new() -> Self {
        Self {
            interrupt: Notify::new(),
            last_message: Mutex::new("Start".to_string())
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
                        Ok((stream, addr)) => 
                            connections.push(
                                tokio::spawn(self.clone().process_connection(stream, addr))
                            ),
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

    async fn process_connection(self: Arc<Self>, mut stream: TcpStream, addr: SocketAddr) {
        println!("Accepted connection from addr: {addr}");

        let (reader, writer) = stream.into_split();

        let reader = futures::io::BufReader::new(TokioAsyncReadCompatExt::compat(reader));
        let writer = futures::io::BufWriter::new(TokioAsyncWriteCompatExt::compat_write(writer));

        let network = twoparty::VatNetwork::new(
            reader,
            writer,
            rpc_twoparty_capnp::Side::Server,
            Default::default(),
        );

        let rpc_system = RpcSystem::new(Box::new(network), None);
        
        tokio::task::spawn(SendFuture::from(rpc_system))
            .await
            .unwrap()
            .unwrap();
    }

    fn interrupt(&self) {
        self.interrupt.notify_waiters();
    }

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