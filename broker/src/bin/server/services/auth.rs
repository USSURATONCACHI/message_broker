use std::net::SocketAddr;

use broker::{auth_capnp::auth_service, util::{Handle, StoreRegistry}};
use capnp::{capability::Promise, Error};
use capnp_rpc::pry;

use crate::stores::LoginStore;

pub struct AuthService {
    peer: SocketAddr,

    login_store: Handle<LoginStore>,
}

impl AuthService {
    pub fn new(peer: SocketAddr, stores: &StoreRegistry) -> Self {
        Self {
            peer,
            login_store: stores.get::<LoginStore>(),
        }
    }
}

impl auth_service::Server for AuthService {
    fn login(&mut self, params: auth_service::LoginParams, _: auth_service::LoginResults) -> Promise<(), Error> {
        let username = pry!(pry!(pry!(params.get()).get_username()).to_string());

        self.login_store.get_mut()
            .log_peer_in(self.peer, username.to_string());

        Promise::ok(())
    }
    fn logout(&mut self, params: auth_service::LogoutParams<>, _: auth_service::LogoutResults<>) -> Promise<(), Error> { 
        self.login_store.get_mut()
            .log_peer_out(&self.peer);

        Promise::ok(())
    }
}