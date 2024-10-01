use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;
use crate::supervisor::supervisor::Supervisor;
use super::http_router::Handlable;
use super::http_router::ParamType;
use super::http_router::RouteData;
use async_trait::async_trait;
use bytes::Bytes;
use hyper::http::Error;
use hyper::Response;
use http_body_util::Full;

//"launch" route
#[derive(Debug)]
pub struct LaunchRoute {
    pub data: RouteData,
}

#[async_trait]
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
    async fn handle_data(&self, route_req_params: HashMap<String, String>, _body: String, supervisor_arc: Arc<RwLock<Supervisor>>) -> Result<Response<Full<Bytes>>, Error> {

        let source_id = route_req_params.get("source_id").unwrap().parse::<i32>().unwrap();

        let supervisor_guard = supervisor_arc.read().await;
        let future = supervisor_guard.launch(source_id);
        let result = future.await;
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


#[derive(Debug)]
pub struct GetStateList {
    pub data: RouteData,
}

#[async_trait]
impl Handlable for GetStateList {
    fn method(&self) -> &str {
        return self.data.method.as_str();
    }
    fn path(&self) -> &str {
        return self.data.path.as_str();
    }
    async fn handle_data(&self, _route_req_params: HashMap<String, String>, _body: String, supervisor_arc: Arc<RwLock<Supervisor>>) -> Result<Response<Full<Bytes>>, Error> {
        let before_time = Instant::now();
        println!("GetStateList: before supervisor_clone: {:?}", Instant::now().duration_since(before_time));
        let supervisor_arc_clone = {
            println!("GetStateList: before read lock: {:?}", Instant::now().duration_since(before_time));
            let supervisor_guard = supervisor_arc.read().await;
            println!("GetStateList: after read lock: {:?}", Instant::now().duration_since(before_time));
            Arc::new(supervisor_guard.clone())
        };
        println!("GetStateList: after supervisor_clone: {:?}", Instant::now().duration_since(before_time));
        
        let processes = supervisor_arc_clone.get_state_list().await;
        let json_message = serde_json::to_string(&processes).unwrap();
        return self.prepare_response(json_message, 200);
    }
    
}

//"404" route
#[derive(Debug)]
pub struct Route404 {
    pub data: RouteData,
}

#[async_trait]
impl Handlable for Route404 {
    fn method(&self) -> &str {
        return self.data.method.as_str();
    }
    fn path(&self) -> &str {
        return self.data.path.as_str();
    }
    // async fn handle_data(&self, body: String) -> Result<Response<Full<Bytes>>, Error> {
    async fn handle_data(&self, _route_req_params: HashMap<String, String>, _body: String, _supervisor_arc: Arc<RwLock<Supervisor>>) -> Result<Response<Full<Bytes>>, Error> {
        return self.prepare_response("404".to_string(), 404);
    }
}


//"terminate" route
#[derive(Debug)]
pub struct TerminateRoute {
    pub data: RouteData,
}

#[async_trait]
impl Handlable for TerminateRoute {
    fn method(&self) -> &str {
        return self.data.method.as_str();
    }
    fn path(&self) -> &str {
        return self.data.path.as_str();
    }
    fn params(&self) -> Option<HashMap<String, ParamType>> {
        return self.data.params.clone();
    }
    async fn handle_data(&self, route_req_params: HashMap<String, String>, _body: String, supervisor_arc: Arc<RwLock<Supervisor>>) -> Result<Response<Full<Bytes>>, Error> {
        let source_id = route_req_params.get("source_id").unwrap().parse::<i32>().unwrap();
        let supervisor_guard = supervisor_arc.read().await;
        let result = supervisor_guard.terminate(source_id).await;

        let http_status_code = match result.is_success() {
            true => 200,
            false => 500
        };
        let message = match result.is_success() {
            true => format!("A process got the termination signal for source {}", source_id),
            false => format!("Failed to start a termination of the process for source {}. Error: {:?}", source_id, result.error_message().unwrap_or(&"Unknown error".to_string()))
        };

        return self.prepare_response(message, http_status_code);
    }
}

//"kill" route
#[derive(Debug)]
pub struct KillRoute {
    pub data: RouteData,
}

#[async_trait]
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
    async fn handle_data(&self, route_req_params: HashMap<String, String>, _body: String, supervisor_arc: Arc<RwLock<Supervisor>>) -> Result<Response<Full<Bytes>>, Error> {
        let source_id = route_req_params.get("source_id").unwrap().parse::<i32>().unwrap();
        let supervisor_guard = supervisor_arc.read().await;
        let result = supervisor_guard.kill_old(source_id).await;

        let http_status_code = match result.is_success() {
            true => 200,
            false => 500
        };
        let message = match result.is_success() {
            true => format!("A process was killed for source {}", source_id),
            false => format!("Failed to kill a process for source {}. Error: {:?}", source_id, result.error_message().unwrap_or(&"Unknown error".to_string()))
        };

        return self.prepare_response(message, http_status_code);
    }
}
