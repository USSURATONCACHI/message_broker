mod services;
mod stores;
mod datatypes;
mod fillers;
mod server;

use std::{fs::File, path::{Path, PathBuf}, str::FromStr, sync::Arc};
use tokio::task;
use server::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load server
    let path = PathBuf::from_str("./server.save.bin")?;
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
    
    let mut file = File::create("./server.save.bin")?;

    bincode::serialize_into(&mut file, &*server)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

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
