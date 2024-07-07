use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Request, Response};
use crate::supervisor::supervisor::Supervisor;

//type BoxBody = http_body_util::combinators::BoxBody<Bytes, Error>;


pub async fn run_command(_req: Request<Incoming>) -> Result<Response<Full<Bytes>>, hyper::http::Error> {
    let mut lnchr = Supervisor::new();
    lnchr.launch(2506);
    
    let state = lnchr.get_child_state(2506);

    match state {
        Ok(state) =>
            Response::builder()
            .status(200)
            .body(Full::new(bytes::Bytes::from(state.to_string()))),
        Err(e) =>
            Response::builder()
            .status(500)
            .body(Full::new(bytes::Bytes::from(e.to_string()))),
    }
}

// async fn kill_command(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
//     let mut lnchr = Supervisor::new();
//     let kill_state = lnchr.kill(2506);
    
//     match kill_state {
//         Ok(kill_state) => Response::new(200, format!("{}", kill_state)),
//         Err(e) => Response::new(500, format!("Error: {}", e)),
//     }
// }