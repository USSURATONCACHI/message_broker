mod services;
mod stores;
mod datatypes;
mod fillers;
mod server;

use std::sync::Arc;
use tokio::task;
use server::Server;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let server = Arc::new(Server::new());
    server.set_interrupt_handler();

    let addr = "127.0.0.1:8080";

    println!("Starting the server on `{addr}`...");
    task::LocalSet::new().run_until(
        server.listen(addr)
    ).await?;
    println!("Server stopped.");

    Ok(())
}
