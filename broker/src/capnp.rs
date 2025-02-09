
pub mod auth_capnp {
    include!(concat!(env!("OUT_DIR"), "/auth_capnp.rs"));
}

pub mod echo_capnp {
    include!(concat!(env!("OUT_DIR"), "/echo_capnp.rs"));
}

pub mod main_capnp {
    include!(concat!(env!("OUT_DIR"), "/main_capnp.rs"));
}

pub mod message_capnp {
    include!(concat!(env!("OUT_DIR"), "/message_capnp.rs"));
}

pub mod topic_capnp {
    include!(concat!(env!("OUT_DIR"), "/topic_capnp.rs"));
}

pub mod util_capnp {
    include!(concat!(env!("OUT_DIR"), "/util_capnp.rs"));
}
