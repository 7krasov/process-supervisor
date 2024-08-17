use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use crate::supervisor::supervisor::Supervisor;
use super::http_router::Handlable;
use super::http_router::ParamType;
use super::http_router::RouteData;
use bytes::Bytes;
use hyper::http::Error;
use hyper::Response;
use http_body_util::Full;

// use async_trait::async_trait;

//"launch" route
#[derive(Debug)]
pub struct LaunchRoute {
    pub data: RouteData,
}

// #[async_trait]
impl Handlable for LaunchRoute {
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
    fn handle_data(&self, route_req_params: HashMap<String, String>, _body: String, supervisor: Arc<Mutex<Supervisor>>) -> Result<Response<Full<Bytes>>, Error> {

        let source_id = route_req_params.get("source_id").unwrap().parse::<i32>().unwrap();

        let mut supervisor_guard = supervisor.lock().unwrap();
        let result = supervisor_guard.launch(source_id);
        let http_status_code = match result.is_success() {
            true => 200,
            false => 500
        };
        let message = match result.is_success() {
            true => format!("A process was started for source {}, PID={}", source_id, result.pid().unwrap()),
            false => format!("Failed to start a process for source {}. Error: {}", source_id, result.error_message().unwrap())
        };

        return self.prepare_response(message, http_status_code);
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
    fn handle_data(&self, _route_req_params: HashMap<String, String>, _body: String, _supervisor: Arc<Mutex<Supervisor>>) -> Result<Response<Full<Bytes>>, Error> {
        return self.prepare_response("404".to_string(), 404);
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
    fn handle_data(&self, route_req_params: HashMap<String, String>, _body: String, supervisor: Arc<Mutex<Supervisor>>) -> Result<Response<Full<Bytes>>, Error> {
        let source_id = route_req_params.get("source_id").unwrap().parse::<i32>().unwrap();
        let supervisor_guard = supervisor.lock().unwrap();
        let result = supervisor_guard.kill(source_id);

        let http_status_code = match result.is_success() {
            true => 200,
            false => 500
        };
        let message = match result.is_success() {
            true => format!("A process was killed for source {}", source_id),
            false => format!("Failed to kill a process for source {}. Error: {}", source_id, result.error_message().unwrap())
        };

        return self.prepare_response(message, http_status_code);
    }
}
