use process_supervisor::k8s::{k8s_common, k8s_supervisor};
use process_supervisor::server::http::http_server::start_http_server;
use process_supervisor::supervisor::Supervisor;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port: u16 = match env::var("HTTP_PORT") {
        Ok(port) => port.parse::<u16>().unwrap(),
        Err(_) => {
            println!("HTTP_PORT is not set. Using default 8080");
            8080
        }
    };

    let k8s_params = k8s_common::get_k8s_params().await;

    let supervisor_arc = Arc::new(RwLock::new(Supervisor::new()));

    //terminate supervisor in case the drain mode is enabled and all processes are finished
    let sv_arc = Arc::clone(&supervisor_arc);
    tokio::task::spawn(async move {
        //catch the drain mode (a special mode where which the application stops accepting new requests and starts shutting down)
        loop {
            let is_drain_mode = k8s_supervisor::is_drain_mode(k8s_params.clone()).await;
            if is_drain_mode.unwrap_or(false) {
                let svg = sv_arc.write().await;
                svg.set_is_drain_mode().await;
                drop(svg);
                println!("Drain mode is enabled.");
                break;
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
        //check if all processes are finished. If so, terminate itself
        loop {
            let svg = sv_arc.write().await;
            let sv = Arc::new(svg.clone());
            let all_finished = sv
                .get_state_list()
                .await
                .values()
                .all(|state| state.is_finished());
            drop(svg);
            if all_finished {
                println!("All processes have finished.");
                println!("Terminating supervisor...");
                std::process::exit(0);
            } else {
                println!("Some processes are still running.");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    });

    //process the kill queue
    let sv_arc = Arc::clone(&supervisor_arc);
    tokio::task::spawn(async move {
        loop {
            let svg = sv_arc.read().await;
            svg.process_kill_queue().await;
            drop(svg);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
    //run dispatcher processes if empty slots are available
    let sv_arc = Arc::clone(&supervisor_arc);
    tokio::task::spawn(async move {
        loop {
            let sv_g = sv_arc.read().await;
            //clean list from finished processes
            sv_g.process_states().await;
            let result = sv_g.populate_empty_slots().await;
            drop(sv_g);
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
