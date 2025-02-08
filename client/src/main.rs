use std::net::ToSocketAddrs;

use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = match std::env::args().skip(1).next() {
        Some(x) => x,
        None => {
            eprintln!("Usage: ./client <address>:<port>");
            return Ok(());
        }
    };
    let addr = addr.to_socket_addrs()?
        .next()
        .ok_or("Provided address is invalid")?;

    println!("Connecting to server {addr}");

    let stream = TcpStream::connect(addr).await?;
    let _ = stream.set_nodelay(true);

    

    Ok(())
}
