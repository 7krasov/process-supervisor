use std::collections::HashMap;
use super::http_router::Handlable;
use super::http_router::ParamType;
use super::http_router::RouteData;

use bytes::Bytes;
use hyper::http::Error;
use hyper::Response;
use http_body_util::Full;

// use async_trait::async_trait;

//"run" route
#[derive(Debug)]
pub struct RunRoute {
    pub data: RouteData,
}

// #[async_trait]
impl Handlable for RunRoute {
    fn method(&self) -> &str {
        return self.data.method.as_str();
    }
    fn path(&self) -> &str {
        return self.data.path.as_str();
    }
    fn params(&self) -> Option<HashMap<String, ParamType>> {
        return self.data.params.clone();
    }
    //async fn handle_data(&self, body: String) -> Result<Response<Full<Bytes>>, Error> {
    fn handle_data(&self, route_req_params: HashMap<String, String>, _body: String) -> Result<Response<Full<Bytes>>, Error> {
        let message = format!("A process is running for source {}", route_req_params.get("source_id").unwrap());
        let bytes = bytes::Bytes::from(message);
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
#[derive(Debug)]
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
    fn handle_data(&self, _route_req_params: HashMap<String, String>, _body: String) -> Result<Response<Full<Bytes>>, Error> {
        let bytes = bytes::Bytes::from("404");
        let body = Full::new(bytes);
        return Response::builder()
        .status(404)
        .body(body);
    }
}


//"kill" route
#[derive(Debug)]
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
    fn params(&self) -> Option<HashMap<String, ParamType>> {
        return self.data.params.clone();
    }
    fn handle_data(&self, route_req_params: HashMap<String, String>, _body: String) -> Result<Response<Full<Bytes>>, Error> {
        let message = format!("A process was killed for source {}", route_req_params.get("source_id").unwrap());
        let bytes = bytes::Bytes::from(message);
        let body = Full::new(bytes);
        return Response::builder()
        .status(200)
        .body(body);
    }
}