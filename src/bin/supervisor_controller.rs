use process_supervisor::k8s::k8s_common;
use process_supervisor::k8s::k8s_controller::start_controller;

#[tokio::main]
async fn main() {
    let k8s_params = k8s_common::get_k8s_params().await;
    start_controller(k8s_params).await;
}
