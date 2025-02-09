use broker::{auth_capnp::auth_service, echo_capnp::echo};
use capnp::{capability::Promise, Error};

use broker::main_capnp::root_service;

use root_service::{AuthParams, AuthResults, EchoParams, EchoResults};

pub struct RootService {
    pub auth: auth_service::Client,
    pub echo: echo::Client,
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
}