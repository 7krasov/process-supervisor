use server::http::http_server::start_http_server;
use std::{env, net::SocketAddr, sync::Arc, time::Duration};
use supervisor::Supervisor;
use tokio::sync::RwLock;

mod k8s;
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

    //catch the drain mode (a special mode in which the application stops accepting new requests and starts shutting down)
    let supervisor_arc_clone = Arc::clone(&supervisor_arc);
    tokio::task::spawn(async move {
        loop {
            let is_drain_mode = k8s::drain_mode_observer::is_drain_mode().await;
            if is_drain_mode.unwrap_or(false) {
                supervisor_arc_clone.write().await.set_is_drain_mode().await;
                println!("Drain mode is enabled.");
                break;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    //process the kill queue
    let supervisor_arc_clone = Arc::clone(&supervisor_arc);
    tokio::task::spawn(async move {
        loop {
            let supervisor_guard = supervisor_arc_clone.read().await;
            supervisor_guard.process_kill_queue().await;
            drop(supervisor_guard);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    //run dispatcher processes if empty slots are available
    let supervisor_arc_clone = Arc::clone(&supervisor_arc);
    tokio::task::spawn(async move {
        loop {
            let supervisor_guard = supervisor_arc_clone.read().await;
            let result = supervisor_guard.populate_empty_slots().await;
            drop(supervisor_guard);
            if result.is_err() {
                println!("Drain mode is caught. Will not populate anymore");
                break;
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });

    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    start_http_server(addr, supervisor_arc).await
}
