use core::str;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use bytes::Bytes;
use hyper::body::Incoming;
use hyper::{Request, Response};
use hyper::http::Error;
use http_body_util::{BodyExt, Full};
use std::fmt::Debug;

use crate::supervisor::supervisor::Supervisor;
// use async_trait::async_trait;



// fn route_method<T>(route: &Route) -> &str {
//     return route.data.method.as_str();
// }

// fn route_path<T>(route: &Route) -> &str {
//     return route.data.path.as_str();
// }


#[async_trait]
pub trait Handlable: Send + Sync + Debug {
    fn method(&self) -> &str;
    fn path(&self) -> &str;
    fn params(&self) -> Option<HashMap<String, ParamType>> {
        return None;
    }
    // async fn handle_data(&self, body: String) -> Result<Response<Full<Bytes>>, Error>;
    async fn handle_data(&self, _route_req_params: HashMap<String, String>, _body: String, _supervisor: Arc<RwLock<Supervisor>>) -> Result<Response<Full<Bytes>>, Error>
    {
        unimplemented!();
        // let message = format!("An unhandled route {} {} {:?}", self.method(), self.path(), route_req_params);
        // let bytes = bytes::Bytes::from(message);
        // let body = Full::new(bytes);
        // return Response::builder()
        // .status(500)
        // .body(body);
    }

    fn prepare_response(&self, message: String, http_status_code: u16) -> Result<Response<Full<Bytes>>, Error> {
        let bytes = bytes::Bytes::from(message);
        let body = Full::new(bytes);
        return Response::builder()
        .status(http_status_code)
        .body(body);
    }
}

#[derive(Clone)]
#[derive(Debug)]
pub enum ParamType {
    AnyString,
    Integer,
}

#[derive(Debug)]
pub struct RouteData {
    pub method: String,
    pub path: String,
    pub params: Option<HashMap<String, ParamType>>,
}

#[derive(Debug)]
pub struct Router {
    routes: Vec<Box<dyn Handlable + Send + Sync>>,
    not_found_route: Box<dyn Handlable + Send + Sync>,
}

impl Router {
    pub fn new(routes:Vec<Box<dyn Handlable + Send + Sync>>, route_404: Box<dyn Handlable + Send + Sync>) -> Self {
        // let route_404 = Route404 {data: RouteData {method: "GET".to_string(), path: "/404".to_string(), params: None}};
        // Self { routes, not_found_route: Box::new(route_404) as Box<dyn Handlable + Send + Sync>}
        Self { routes, not_found_route: route_404}
    }
    pub async fn handle_request(self, req: Request<Incoming>, supervisor: Arc<RwLock<Supervisor>>) -> Response<Full<Bytes>> {
        let route: &Box<dyn Handlable + Send + Sync> = self.route(req.method().as_str(), req.uri().path());
        
        // println!("route: {:?}", route);

        let route_req_params = self.route_request_params(req.uri().path().to_string(), route);

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

        let response = route.handle_data(route_req_params, str::from_utf8(&b).unwrap().to_string(), supervisor);
        
        match response.await {
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

    fn route(&self, req_method: &str, req_path: &str) -> &Box<(dyn Handlable + Send + Sync + 'static)> {
        let req_path_segments: Vec<&str> = req_path.trim_matches('/').split('/').collect();

        let route = self.routes.iter().find(|route| {
            let route_segments: Vec<&str> = route.path().trim_matches('/').split('/').collect();
            if route.method() != req_method || req_path_segments.len() != route_segments.len() {
                return false;
            }

            route_segments.iter().zip(req_path_segments.iter()).all(|(route_segment, path_segment)| {
                if route_segment.starts_with('{') && route_segment.ends_with('}') {
                    // This is a variable segment, consider it a match for any value.
                    true
                } else {
                    // This is a static segment, it must match exactly.
                    route_segment == path_segment
                }
            })
        });

        // match route {
        //     Some(route) => route,
        //     None => &self.not_found_route,
        // }
        // let route = self
        //     .routes
        //     .iter()
        //     .find(|route| route.method() == method && route.path() == path);

        
        if route.is_none() {
            return &self.not_found_route;
        }

        let route = route.unwrap();
        return route;
    }

    fn route_request_params(&self, req_path: String, route: &Box<(dyn Handlable + Send + Sync + 'static)>) -> HashMap<String, String> {

        let route_params = route.params().unwrap_or(HashMap::new());
        let mut route_req_params: HashMap<String, String> = HashMap::new();

        if route_params.is_empty() {
            return route_req_params;
        }

        let req_path_segments: Vec<&str> = req_path.trim_matches('/').split('/').collect();
        let route_path_segments: Vec<&str> = route.path().trim_matches('/').split('/').collect();

        route_path_segments.iter().zip(req_path_segments.iter()).for_each(|(route_segment, req_segment)| {
            if route_segment.starts_with('{') && route_segment.ends_with('}') {
                let name = route_segment.trim_matches(|c| c == '{' || c == '}');

                let param_type = route_params.get(name);
                if param_type.is_none() {
                    return;
                }

                let param_type = param_type.unwrap();
                match param_type {
                    ParamType::AnyString => {
                        route_req_params.insert(name.to_string(), req_segment.to_string());
                    },
                    ParamType::Integer => {
                        let segment = req_segment.parse::<i32>();
                        if segment.is_err() {
                            return;
                        }
                        route_req_params.insert(name.to_string(), req_segment.to_string());
                    },
                }
            }
        });

        return route_req_params;

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
