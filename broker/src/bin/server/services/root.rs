use broker::message_capnp::message_service;
use broker::topic_capnp::topic_service;
use broker::echo_capnp::echo;
use broker::auth_capnp::auth_service;
use broker::main_capnp::root_service::{MessageParams, MessageResults, TopicParams, TopicResults};
use capnp::{capability::Promise, Error};

use broker::main_capnp::root_service;

use root_service::{AuthParams, AuthResults, EchoParams, EchoResults};

pub struct RootService {
    pub auth: auth_service::Client,
    pub echo: echo::Client,
    pub topic: topic_service::Client,
    pub message: message_service::Client,
}

impl root_service::Server for RootService {
    fn auth(&mut self, _: AuthParams, mut results: AuthResults) -> Promise<(), Error> { 
        results.get().set_service(self.auth.clone());
        Promise::ok(())
    }
    
    fn echo(&mut self, _: EchoParams, mut results: EchoResults) -> Promise<(), Error> { 
        results.get().set_service(self.echo.clone());
        Promise::ok(())
    }

    fn topic(&mut self, _: TopicParams, mut results: TopicResults) -> Promise<(), Error> { 
        results.get().set_service(self.topic.clone());
        Promise::ok(())
    }

    fn message(&mut self, _: MessageParams, mut results: MessageResults) -> Promise<(), Error> { 
        results.get().set_service(self.message.clone());
        Promise::ok(())
    }
}