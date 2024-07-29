use hyper::{Request, Response};
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use std::collections::HashMap;
use std::{convert::Infallible, net::SocketAddr};
use super::http_router::{Handlable, ParamType, RouteData, Router};
use super::http_routes::{KillRoute, RunRoute, Route404};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

async fn handle(request: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    let router = Router::new(
        vec![
            // Box::new(Route404 {method: "GET".to_string(), path: "/404".to_string()})
            Box::new(RunRoute {
                data: RouteData {
                    method: "POST".to_string(),
                    path: "/run/{source_id}".to_string(),
                    params: Some(HashMap::from([("source_id".to_string(), ParamType::Integer)])),
                }
            }),
            Box::new(KillRoute {
                data: RouteData {
                    method: "POST".to_string(),
                    path: "/kill/{source_id}".to_string(),
                    params: Some(HashMap::from([("source_id".to_string(), ParamType::Integer)])),
                },
            })
        ],
        Box::new(
            Route404 {data: RouteData {method: "GET".to_string(), path: "/404".to_string(), params: None}}
        ) as Box<dyn Handlable + Send + Sync>
    
    );

    let response: Response<Full<Bytes>> = router.handle_request(request).await;

    Ok(response)
}

pub async fn start_http_server(addr: SocketAddr)-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Bind to the port and listen for incoming TCP connections
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on http://{}", addr);
    loop {
        // When an incoming TCP connection is received grab a TCP stream for
        // client<->server communication.
        //
        // Note, this is a .await point, this loop will loop forever but is not a busy loop. The
        // .await point allows the Tokio runtime to pull the task off of the thread until the task
        // has work to do. In this case, a connection arrives on the port we are listening on and
        // the task is woken up, at which point the task is then put back on a thread, and is
        // driven forward by the runtime, eventually yielding a TCP stream.
        let (tcp, _) = listener.accept().await?;
        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(tcp);

        // Spin up a new task in Tokio so we can continue to listen for new TCP connection on the
        // current task without waiting for the processing of the HTTP1 connection we just received
        // to finish
        tokio::task::spawn(async move {
            // Handle the connection from the client using HTTP1 and pass any
            // HTTP requests received on that connection to the `handle` function
            if let Err(err) = http1::Builder::new()
                // .timer(TokioTimer::new())
                .serve_connection(io, service_fn(handle))
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}