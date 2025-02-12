use std::io::{stdout, Write};

use broker::message_capnp::message_receiver::{self, ReceiveParams};
use capnp::capability::Promise;
use capnp_rpc::pry;

use crate::{datatypes::Message, readers::read_capnp_message};


pub struct MessageReceiver {
    action: Box<dyn FnMut(Message)>,
}

impl MessageReceiver {
    pub fn new(action: impl 'static + FnMut(Message)) -> Self {
        Self {
            action: Box::new(action),
        }
    }
}

impl message_receiver::Server for MessageReceiver {
    fn receive(&mut self, params: ReceiveParams) -> Promise<(), capnp::Error> {
        let reader = pry!(params.get());
        
        let message = pry!(reader.get_message());
        let message = pry!(read_capnp_message(message));
        
        (self.action)(message);
        stdout().flush().unwrap();
        Promise::ok(())
    }
}