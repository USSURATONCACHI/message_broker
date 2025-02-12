use std::net::SocketAddr;

use broker::{main_capnp::root_service, util::stream_to_rpc_network};
use capnp_rpc::{rpc_twoparty_capnp, RpcSystem};
use tokio::net::TcpStream;

pub type RpcSystemHandle = tokio::task::JoinHandle<Result<(), capnp::Error>>;

pub async fn connect_to_server(addr: SocketAddr) -> Result<(RpcSystemHandle, root_service::Client), std::io::Error> {
    let stream = TcpStream::connect(addr).await?;
    let _ = stream.set_nodelay(true);

    // RPC Init
    let network = stream_to_rpc_network(stream);
    let mut rpc_system = RpcSystem::new(Box::new(network), None);

    let root: root_service::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Client);
    let rpc_handle = tokio::task::spawn_local(rpc_system);

    Ok((rpc_handle, root))
}