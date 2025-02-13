mod services;
mod stores;
mod datatypes;
mod fillers;
mod server;
mod retention_eater;

use std::io::Write;
use std::path::PathBuf;
use std::{fs::File, net::SocketAddr, path::Path, str::FromStr, sync::Arc};
use tokio::task;
use server::Server;
use clap::arg;
use clap::Parser;

#[derive(Parser, Debug, Clone)]
pub struct CliArgs {
    #[arg(short, long, default_value_t = SocketAddr::from_str("127.0.0.1:8080").unwrap())]
    pub address: SocketAddr,

    #[arg(short, long, default_value_t = String::from("server.save.bin"))]
    pub state_file: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();
    // Load server
    let path: PathBuf = PathBuf::from_str(&args.state_file)?;
    let addr = "127.0.0.1:8080";

    // Run server
    let server = Arc::new(load_server(&path)?.unwrap_or_default());
    server.set_interrupt_handler();

    run_server(server.clone(), addr).await;

    let server = Arc::into_inner(server).expect("Some inner jobs did not shut down in time");

    // Save server
    save_state(&server, &path).await?;

    Ok(())
}

async fn run_server(server: Arc<Server>, addr: &str) {
    println!("Starting the server on `{addr}`...");
    
    let future = server.listen(addr);
    let run_result = task::LocalSet::new().run_until(future).await;
    
    if run_result.is_err() {
        eprintln!("Server shutdown with error: {}", run_result.unwrap_err());
    } else {
        println!("Server stopped.");
    }
}

async fn save_state(server: &Server, save_path: &Path) -> std::io::Result<()> {
    println!("Saving server state into '{}'...", save_path.display());
    
    let mut file = File::create(save_path)?;

    bincode::serialize_into(&mut file, &*server)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    file.flush().unwrap();

    Ok(())
}

fn load_server(save_path: &Path) -> Result<Option<Server>, std::io::Error> {
    if Path::exists(save_path) {
        println!("Loading existing server state from '{}'...", save_path.display());

        let file = File::open(save_path)?;

        let server = bincode::deserialize_from::<_, Server>(file)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(Some(server))
    } else {
        Ok(None)
    }
}
