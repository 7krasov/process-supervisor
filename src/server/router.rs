use core::str;
use bytes::Bytes;
use hyper::body::Incoming;
use hyper::{Request, Response};
use hyper::http::Error;
use http_body_util::{BodyExt, Full};
// use async_trait::async_trait;



// fn route_method<T>(route: &Route) -> &str {
//     return route.data.method.as_str();
// }

// fn route_path<T>(route: &Route) -> &str {
//     return route.data.path.as_str();
// }


// #[async_trait]
pub trait Handlable: Send + Sync {
    fn method(&self) -> &str;
    fn path(&self) -> &str;
    // async fn handle_data(&self, body: String) -> Result<Response<Full<Bytes>>, Error>;
    fn handle_data(&self, body: String) -> Result<Response<Full<Bytes>>, Error>;
}


pub struct RouteData {
    pub method: String,
    pub path: String,
}


//"run" route
pub struct RunRoute {
    pub data: RouteData,
    // pub method: String,
    // pub path: String,
}

// #[async_trait]
impl Handlable for RunRoute {
    fn method(&self) -> &str {
        return self.data.method.as_str();
    }
    fn path(&self) -> &str {
        return self.data.path.as_str();
    }
    //async fn handle_data(&self, body: String) -> Result<Response<Full<Bytes>>, Error> {
    fn handle_data(&self, _body: String) -> Result<Response<Full<Bytes>>, Error> {
        let bytes = bytes::Bytes::from("A process is running.");
        let body = Full::new(bytes);
        return Response::builder()
        .status(200)
        .body(body);
        // return Response::builder()
        // .status(200)
        // .body(body);
    }
}

//"404" route
pub struct Route404 {
    pub data: RouteData,
}

// #[async_trait]
impl Handlable for Route404 {
    fn method(&self) -> &str {
        return self.data.method.as_str();
    }
    fn path(&self) -> &str {
        return self.data.path.as_str();
    }
    // async fn handle_data(&self, body: String) -> Result<Response<Full<Bytes>>, Error> {
    fn handle_data(&self, _body: String) -> Result<Response<Full<Bytes>>, Error> {
        let bytes = bytes::Bytes::from("404");
        let body = Full::new(bytes);
        return Response::builder()
        .status(404)
        .body(body);
    }
}


//"kill" route
pub struct KillRoute {
    pub data: RouteData,
}

// #[async_trait]
impl Handlable for KillRoute {
    fn method(&self) -> &str {
        return self.data.method.as_str();
    }
    fn path(&self) -> &str {
        return self.data.path.as_str();
    }
    fn handle_data(&self, _body: String) -> Result<Response<Full<Bytes>>, Error> {
        let bytes = bytes::Bytes::from("OK");
        let body = Full::new(bytes);
        return Response::builder()
        .status(200)
        .body(body);
    }
}

pub struct Router {
    routes: Vec<Box<dyn Handlable + Send + Sync>>,
    not_found_route: Box<dyn Handlable + Send + Sync>,
}

impl Router {
    pub fn new(routes:Vec<Box<dyn Handlable + Send + Sync>>) -> Self {
        let route_404 = Route404 {data: RouteData {method: "GET".to_string(), path: "/404".to_string()}};
        Self { routes, not_found_route: Box::new(route_404) as Box<dyn Handlable + Send + Sync>}
    }
    pub async fn handle_request(self, req: Request<Incoming>) -> Response<Full<Bytes>> {
        let route = self.route(req.method().as_str(), req.uri().path());


        let b = req.collect().await;

        if let Err(err) = b {
            let err_response_text = err.to_string();
            let err_bytes = bytes::Bytes::from(err_response_text);
            let err_body = Full::new(err_bytes);
            
            let response =
                Response::builder()
                .status(500)
                .body(err_body)
                .unwrap();
            return response;
        }

        let b = b.unwrap();
        let b = b.to_bytes();

        let response = route.handle_data(str::from_utf8(&b).unwrap().to_string());
        
        match response {
            Ok(response) => response,
            Err(err) => {
                let err_response_text = err.to_string();
                let err_bytes = bytes::Bytes::from(err_response_text);
                let err_body = Full::new(err_bytes);
        
                let response = Response::builder()
                    .status(500)
                    .body(err_body)
                    .unwrap();
                response
            }
        }
    }

    fn route(&self, method: &str, path: &str) -> &Box<(dyn Handlable + Send + Sync + 'static)> {
        let route = self
            .routes
            .iter()
            .find(|route| route.method() == method && route.path() == path);

        
        if route.is_none() {
            return &self.not_found_route;
        }

        let route = route.unwrap();
        return route;
    }
}

// impl Default for Router {
//     fn default() -> Self {
//         let mut routes: Vec<Box<dyn Route + Send + Sync>> = vec![
//             Route404 {method: "GET".to_string(), path: "/404".to_string()},
//         ];

//         Self { routes }
//     }
// }