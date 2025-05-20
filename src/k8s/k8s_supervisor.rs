use crate::k8s::k8s_common::K8sParams;
use k8s_openapi::api::core::v1::Pod;
use kube::api::Api;
use std::error::Error;

pub async fn is_drain_mode(k8s_params: K8sParams) -> Result<bool, Box<dyn Error>> {
    //getting pod
    let pods: Api<Pod> =
        Api::namespaced(k8s_params.get_client(), k8s_params.get_namespace().as_ref());
    let pod = pods.get(k8s_params.get_pod_name().as_ref()).await;
    if pod.is_err() {
        println!("Unable to get pod: {:?}", pod.err());
        return Ok(false);
    }
    let annotations = pod.unwrap().metadata.annotations.unwrap_or_default();
    Ok(matches!(annotations.get("drain"), Some(val) if val == "true"))
}
