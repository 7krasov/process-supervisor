use std::{net::SocketAddr, sync::{Arc, Mutex}};
use supervisor::supervisor::Supervisor;
use server::http::http_server::start_http_server;

mod supervisor {
    pub mod supervisor;
}

mod server {
    pub mod http {
        pub mod http_router;
        pub mod http_routes;
        pub mod http_server;
    }
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let supervisor_arc = Arc::new(Mutex::new(Supervisor::new()));

    let addr: SocketAddr = ([127, 0, 0, 1], 8888).into();
    return start_http_server(addr, supervisor_arc).await;


    // let mut lnchr = Supervisor::new();
    // lnchr.launch(2506);
    
    // let state = lnchr.get_child_state(2506);
    
    // match state {
    //     Ok(state) => println!("{}", state),
    //     Err(e) => println!("Error: {}", e),
    // }

    // //wait until process will be run
    // sleep(Duration::from_secs(1));

    // let kill_state = lnchr.kill(2506);

    // match kill_state {
    //     Ok(kill_state) => println!("{}", kill_state),
    //     Err(e) => println!("Error: {}", e),
    // }
}
