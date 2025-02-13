

use capnp_rpc::{rpc_twoparty_capnp, twoparty};
use tokio::net::TcpStream;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub type RpcNetwork = twoparty::VatNetwork<futures::io::BufReader<tokio_util::compat::Compat<tokio::net::tcp::OwnedReadHalf>>>;

/// Converts a `tokio::net::TcpStream` into a `capnp_rpc::twoparty::VatNetwork`.
/// use std::net::SocketAddr;
/// ```rust
/// # use capnp_rpc::RpcSystem;
/// # use std::net::SocketAddr;
/// # use tokio::net::TcpStream;
/// 
/// use broker::util::stream_to_rpc_network;
/// use std::str::FromStr;
/// 
/// #[tokio::main]
/// async fn main() -> std::io::Result<()> {
///     let addr: SocketAddr = SocketAddr::from_str("127.0.0.1:8080").unwrap();
///     let stream = TcpStream::connect(addr).await?;
///     let _ = stream.set_nodelay(true);
///     let network = stream_to_rpc_network(stream);
///     let mut rpc_system = RpcSystem::new(Box::new(network), None);
///     Ok(())
/// }
/// 
/// ```

pub fn stream_to_rpc_network(stream: TcpStream) -> RpcNetwork {
    let (reader, writer) = stream.into_split();

    let reader = futures::io::BufReader::new(TokioAsyncReadCompatExt::compat(reader));
    let writer = futures::io::BufWriter::new(TokioAsyncWriteCompatExt::compat_write(writer));
    
    let network: RpcNetwork  = twoparty::VatNetwork::new(
        reader,
        writer,
        rpc_twoparty_capnp::Side::Server,
        Default::default(),
    );    
    
    network
}
