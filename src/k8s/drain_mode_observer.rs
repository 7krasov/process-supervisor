use k8s_openapi::api::core::v1::Pod;
use kube::{api::Api, Client};
use std::error::Error;

const ENV_HOSTNAME: &str = "HOSTNAME";
const NAMESPACE_FILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/namespace";

pub async fn is_drain_mode() -> Result<bool, Box<dyn Error>> {
    //pod name
    let pod_name = std::env::var(ENV_HOSTNAME);
    if pod_name.is_err() {
        println!("Unable to get {} env variable value", ENV_HOSTNAME);
        return Ok(false);
    }
    let pod_name = pod_name.unwrap();

    //namespace
    let namespace = tokio::fs::read_to_string(NAMESPACE_FILE).await;
    if namespace.is_err() {
        println!("Unable to get namespace");
        return Ok(false);
    }
    let namespace = namespace.unwrap().trim().to_string();

    //K8s client
    let client = Client::try_default().await;
    if client.is_err() {
        println!("Unable to create kube client");
        return Ok(false);
    }
    let client = client.unwrap();

    //getting pod
    let pods: Api<Pod> = Api::namespaced(client, &namespace);
    let pod = pods.get(&pod_name).await;
    if pod.is_err() {
        println!("Unable to get pod");
        return Ok(false);
    }
    let annotations = pod.unwrap().metadata.annotations.unwrap_or_default();

    Ok(matches!(annotations.get("drain"), Some(val) if val == "true"))
}
