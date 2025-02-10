use std::{net::SocketAddr, sync::{Arc, Weak}, time::Duration};

use broker::{echo_capnp::echo, util::{Handle, SendFuture, StoreRegistry}};
use capnp::capability::Promise;
use capnp_rpc::pry;

use broker::echo_capnp::ping_receiver;

use crate::stores::LoginStore;

pub struct EchoService {
    peer: SocketAddr,
    pings_receiver: Option<Arc<ping_receiver::Client>>, 
    
    login_store: Handle<LoginStore>,
}

impl EchoService {
    pub fn new(peer: SocketAddr, stores: &StoreRegistry) -> Self {
        Self {
            peer,
            pings_receiver: None,

            login_store: stores.get::<LoginStore>()
        }
    }
}

impl echo::Server for EchoService {
    fn echo(&mut self, params: echo::EchoParams, mut results: echo::EchoResults) -> Promise<(), capnp::Error> {
        let username = pry!(self.login_store.get().check_login(&self.peer));

        let request = pry!(pry!(params.get()).get_request());

        let in_message = pry!(pry!(request.get_message()).to_str());
        let message = format!("Hello, {in_message}!");
        
        results.get().init_reply().set_message(message);

        println!("Peer {username} said: `{in_message}`");

        Promise::ok(())
    }

    fn subscribe_to_pings(&mut self, params: echo::SubscribeToPingsParams, _: echo::SubscribeToPingsResults) -> Promise<(), capnp::Error> {
        println!("Checking authorization...");
        let username = pry!(self.login_store.get().check_login(&self.peer));
        println!("Username: {username}");

        let receiver: ping_receiver::Client = pry!(pry!(params.get()).get_receiver() );

        println!("Peer {username} tried to subscribe to pings.");
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
