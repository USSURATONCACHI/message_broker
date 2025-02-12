

use std::net::SocketAddr;
use std::net::ToSocketAddrs;
use std::str::from_utf8_unchecked;

use tokio::io;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;

pub fn print_usage() {
    eprintln!("Usage: ./client <address>:<port> <username>");
}

pub fn parse_cli_args() -> Result<(SocketAddr, String), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1); // Skip the program name itself.

    // Parse address
    let addr = match args.next() {
        Some(x) => x,
        None => {
            print_usage();
            return Err("No address provided".into());
        }
    };
    let addr = addr.to_socket_addrs()?
        .next()
        .ok_or("Provided address is invalid")?;

    // Parse username
    let username = match args.next() {
        Some(x) => x,
        None => {
            print_usage();
            return Err("No username provided".into());
        },
    };

    // Done
    Ok((addr, username))
}

pub async fn read_line(reader: &mut (impl AsyncRead + Unpin), buffer: &mut String) -> io::Result<String> {
    let mut bytes: [u8; 1024] = [0; 1024];

    // Refill the buffer if it does not contain the line already
    while !buffer.contains('\n') {
        let was_read = reader.read(&mut bytes).await?;
        buffer.push_str(unsafe { from_utf8_unchecked(&bytes[0..was_read]) });

        // If we reached EOF - return all we have
        if was_read == 0 {
            let result = buffer.clone();
            buffer.clear();
            return Ok(result);
        }
    }

    // If buffer already has the whole line - consume it and return.
    match buffer.find('\n') {
        Some(newline_pos) => {
            let result = buffer[0..=newline_pos].to_owned();
            *buffer = buffer[newline_pos + 1 ..].to_owned();
            Ok(result)
        }
        None => {
            let result = buffer.clone();
            buffer.clear();
            Ok(result)
        }
    }
}