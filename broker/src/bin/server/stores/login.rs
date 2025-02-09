use std::collections::HashMap;
use std::net::SocketAddr;

use capnp::Error;

use crate::datatypes::Username;


#[derive(Default)]
pub struct LoginStore {
    usernames_per_socket: HashMap<SocketAddr, Username>,
}

impl LoginStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_login(&self, peer: &SocketAddr) -> Option<Username> {
        self.usernames_per_socket
            .get(peer)
            .cloned()
    }

    pub fn check_login(&self, peer: &SocketAddr) -> Result<Username, Error> {
        self.get_login(peer).ok_or(Error {
            kind: capnp::ErrorKind::Failed,
            extra: "Peer is not logged in".to_owned()
        })
    }

    pub fn log_peer_in(&mut self, peer: SocketAddr, username: String) {
        self.usernames_per_socket.insert(peer, username);
    }

    pub fn log_peer_out(&mut self, peer: &SocketAddr) {
        self.usernames_per_socket.remove(&peer);
    }
}