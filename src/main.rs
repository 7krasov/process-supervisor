use server::http::http_server::start_http_server;
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use supervisor::Supervisor;
use tokio::sync::RwLock;

mod server;
mod supervisor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port: u16 = match env::var("HTTP_PORT") {
        Ok(port) => port.parse::<u16>().unwrap(),
        Err(_) => {
            println!("HTTP_PORT is not set. Using default 8080");
            8080
        }
    };

    let supervisor_arc = Arc::new(RwLock::new(Supervisor::new()));

    let supervisor_arc_clone = Arc::clone(&supervisor_arc);
    tokio::task::spawn(async move {
        loop {
            let supervisor_guard = supervisor_arc_clone.read().await;
            supervisor_guard.process_kill_queue().await;
            drop(supervisor_guard);
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    // let addr: SocketAddr = ([127, 0, 0, 1], 8888).into();
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    start_http_server(addr, supervisor_arc).await
}
