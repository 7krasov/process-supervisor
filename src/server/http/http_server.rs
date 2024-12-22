use super::http_router::{Handleable, ParamType, RouteData, Router};
use super::http_routes::{GetStateList, KillRoute, LaunchRoute, Route404, TerminateRoute};
use crate::supervisor::Supervisor;
use http_body_util::Full;
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::Service;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::{convert::Infallible, net::SocketAddr};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

pub async fn start_http_server(
    addr: SocketAddr,
    supervisor_arc: Arc<RwLock<Supervisor>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let http_service = HttpService { supervisor_arc };

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

        let http_service_cloned = http_service.clone();
        // Spin up a new task in Tokio so we can continue to listen for new TCP connection on the
        // current task without waiting for the processing of the HTTP1 connection we just received
        // to finish
        tokio::task::spawn(async move {
            // Handle the connection from the client using HTTP1 and pass any
            // HTTP requests received on that connection to the `handle` function
            if let Err(err) = http1::Builder::new()
                // .timer(TokioTimer::new())
                // .serve_connection(io, service_fn(handle))
                .serve_connection(io, http_service_cloned)
                .await
            {
                println!("Error serving connection: {:?}", err);
            }
        });
    }
}

#[derive(Debug, Clone)]
struct HttpService {
    supervisor_arc: Arc<RwLock<Supervisor>>,
}

impl Service<Request<Incoming>> for HttpService {
    type Response = Response<Full<Bytes>>;

    type Error = Infallible;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, request: Request<Incoming>) -> Self::Future {
        let router = Router::new(
            vec![
                // Box::new(Route404 {method: "GET".to_owned(), path: "/404".to_owned()})
                Box::new(LaunchRoute {
                    data: RouteData {
                        method: "POST".to_owned(),
                        path: "/launch/{source_id}".to_owned(),
                        params: Some(HashMap::from([(
                            "source_id".to_owned(),
                            ParamType::Integer,
                        )])),
                    },
                }),
                Box::new(TerminateRoute {
                    data: RouteData {
                        method: "POST".to_owned(),
                        path: "/terminate/{source_id}".to_owned(),
                        params: Some(HashMap::from([(
                            "source_id".to_owned(),
                            ParamType::Integer,
                        )])),
                    },
                }),
                Box::new(KillRoute {
                    data: RouteData {
                        method: "POST".to_owned(),
                        path: "/kill/{source_id}".to_owned(),
                        params: Some(HashMap::from([(
                            "source_id".to_owned(),
                            ParamType::Integer,
                        )])),
                    },
                }),
                Box::new(GetStateList {
                    data: RouteData {
                        method: "GET".to_owned(),
                        path: "/state-list".to_owned(),
                        params: None,
                    },
                }),
            ],
            Box::new(Route404 {
                data: RouteData {
                    method: "GET".to_owned(),
                    path: "/404".to_owned(),
                    params: None,
                },
            }) as Box<dyn Handleable>,
        );

        let supervisor = self.supervisor_arc.clone();
        Box::pin(async {
            println!(
                "Request: {} {:?} Body: {:?}",
                request.method(),
                request.uri(),
                request.body()
            );
            // println!("Request: {:?}", request);
            // Ok(async_fn)
            let response: Response<Full<Bytes>> = router.handle_request(request, supervisor).await;
            println!("Response: {} {:?}", response.status(), response.body());
            // println!("Response: {:?}", response);
            Ok(response)
        })
    }
}
