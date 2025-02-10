use capnp_rpc::{rpc_twoparty_capnp, twoparty};
use tokio::net::TcpStream;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

pub type RpcNetwork = twoparty::VatNetwork<futures::io::BufReader<tokio_util::compat::Compat<tokio::net::tcp::OwnedReadHalf>>>;

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
