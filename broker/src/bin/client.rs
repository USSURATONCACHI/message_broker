use std::net::{SocketAddr, ToSocketAddrs};
use tokio::io::{AsyncBufReadExt, BufReader};

use capnp_rpc::{rpc_twoparty_capnp, RpcSystem};
use tokio::net::TcpStream;

use broker::{schema_capnp::echo::{self}, util::{stream_to_rpc_network, SendFuture}};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = parse_cli_addr()?;

    // Connection
    println!("Connecting to server {addr}");
    let stream = TcpStream::connect(addr).await?;
    let _ = stream.set_nodelay(true);

    // RPC Init
    let network = stream_to_rpc_network(stream);
    let mut rpc_system = RpcSystem::new(Box::new(network), None);
    let echo_client: echo::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Client);

    tokio::spawn(SendFuture::from(rpc_system));

    do_work(echo_client).await?;

    Ok(())
}

fn parse_cli_addr() -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let addr = match std::env::args().skip(1).next() {
        Some(x) => x,
        None => {
            eprintln!("Usage: ./client <address>:<port>");
            return Err("No address provided".into());
        }
    };
    let addr = addr.to_socket_addrs()?
        .next()
        .ok_or("Provided address is invalid")?;

    Ok(addr)
}

async fn do_work(client: echo::Client) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = String::new();
    let mut reader = BufReader::new(tokio::io::stdin());
    loop {
        buf.clear();
        reader.read_line(&mut buf).await?;
        let trimmed = buf.trim();

        if trimmed.len() == 0 {
            println!("Stopping...");
            break;
        }

        let response = echo_request(&client, trimmed).await?;
        println!("Response: {response}");
    }

    Ok(())
}

async fn echo_request(client: &echo::Client, message: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut request = client.echo_request();
    let mut builder = request.get().init_request();
    builder.set_message(message);

    let reply = request.send().promise.await?;
    let message = reply.get()?
        .get_reply()?
        .get_message()?
        .to_string()?;

    Ok(message)
}


