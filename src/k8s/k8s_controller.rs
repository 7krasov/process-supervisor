use crate::k8s::k8s_common::K8sParams;
use futures_util::stream::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{ListParams, Patch, PatchParams};
use kube::core::ObjectList;
use kube::{Api, Client, ResourceExt};
use kube_runtime::controller::Action;
use kube_runtime::{watcher, Controller};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

const FINALIZER_NAME: &str = "process-supervisor/finalizer";

struct ReconcileContext {
    pods: Arc<Api<Pod>>,
}

#[derive(Debug, Error)]
enum ReconcileError {
    #[error("Kubernetes API error: {0}")]
    KubeError(#[from] kube::Error),

    #[error("Serde JSON error: {0}")]
    SerdeError(#[from] serde_json::Error),
}

async fn terminate_supervisors(pods_api: Api<Pod>) {
    println!("Termination supervisor pods...");

    let patch = json!({
        "metadata": {
            "annotations": {
                "terminate": "true"
            }
        }
    });

    loop {
        let pods = get_supervisor_pods(pods_api.clone()).await;
        if pods.items.is_empty() {
            println!("No supervisor pods found. Termination has been completed completed.");
            break;
        }
        println!("Found {} supervisor pods", pods.items.len());

        for pod in pods {
            let name = pod.name_any();
            //annotating a supervisor pod for termination. It will catch a new annotation and terminate itself
            let res = pods_api
                .patch(
                    &name,
                    &PatchParams::apply("process-supervisor/terminate"),
                    &Patch::Merge(&patch),
                )
                .await;
            match res {
                Ok(_) => {
                    println!("Pod {} marked for termination", name);
                }
                Err(e) => {
                    println!("Failed to mark pod {} for termination: {}", name, e);
                }
            }
        }
    }
}

pub async fn get_supervisor_pods(pods_api: Api<Pod>) -> ObjectList<Pod> {
    pods_api.list(&ListParams::default()).await.unwrap()
}

async fn get_controller_pod(client: Client, namespace: &str) -> Option<Pod> {
    let pods: Api<Pod> = Api::namespaced(client, namespace);

    let lp = ListParams::default().labels("controller=true");
    let pod_list = pods.list(&lp).await.ok()?;

    pod_list.items.into_iter().next()
}

// detects if the controller got an annotation to terminate all supervisor pods
async fn check_terminate_annotation(client: Client, namespace: &str) -> bool {
    if let Some(pod) = get_controller_pod(client, namespace).await {
        let should_terminate = pod
            .metadata
            .annotations
            .as_ref()
            .and_then(|a| a.get("terminate-all"))
            .map(|v| v == "true")
            .unwrap_or(false);

        return should_terminate;
    }

    false
}

async fn reconcile(pod: Arc<Pod>, ctx: Arc<ReconcileContext>) -> Result<Action, ReconcileError> {
    println!("Reconciling...");
    let name = pod.name_any();

    let metadata = &pod.metadata;
    let finalizers = metadata.finalizers.clone().unwrap_or_default();

    // add finalizer if it doesn't exist. It does not allow k8s to delete the pod
    if metadata.deletion_timestamp.is_none() && !finalizers.contains(&FINALIZER_NAME.to_string()) {
        println!("Adding finalizer to pod {}", name);
        let patch = json!({
            "metadata": {
                "finalizers": finalizers
                    .into_iter()
                    .chain(std::iter::once(FINALIZER_NAME.to_string()))
                    .collect::<Vec<_>>()
            }
        });

        let res = ctx
            .pods
            .patch(
                &name,
                &PatchParams::apply("process-supervisor/finalizer"),
                &Patch::Merge(&patch),
            )
            .await;
        return match res {
            Ok(_) => {
                println!("Finalizer added to pod {}", name);
                Ok(Action::requeue(Duration::from_secs(1)))
            }
            Err(e) => {
                println!("Failed to add finalizer to pod {}: {}", name, e);
                Err(ReconcileError::KubeError(e))
            }
        };
    }

    // if a pod is marked to be deleted by k8s, add "drain" annotation to it
    // supervisor will not get new processes and will terminate itself after all processes are finished
    if metadata.deletion_timestamp.is_some() {
        let annotations = metadata.annotations.clone().unwrap_or_default();
        let already_draining = annotations
            .get("drain")
            .map(|v| v == "true")
            .unwrap_or(false);

        if !already_draining {
            println!("Pod {} is being deleted. Adding drain annotation...", name);
            let patch = json!({
                "metadata": {
                    "annotations": {
                        "drain": "true"
                    }
                }
            });
            let res = ctx
                .pods
                .patch(
                    &name,
                    &PatchParams::apply("process-supervisor/drain"),
                    &Patch::Merge(&patch),
                )
                .await;
            if let Err(e) = res {
                println!("Failed to add drain annotation to pod {}: {}", name, e);
                return Err(ReconcileError::KubeError(e));
            }
        } else {
            println!("Pod {} is already marked as draining. Waiting...", name);
        }
    }

    // Reconcile with changes awaiting
    Ok(Action::requeue(Duration::from_secs(5)))
}

//error processing
fn error_policy(pod: Arc<Pod>, err: &ReconcileError, _ctx: Arc<ReconcileContext>) -> Action {
    println!("Error occurred for Pod {}: {}", pod.name_any(), err);
    Action::requeue(Duration::from_secs(10)) //try again after some delay
}

async fn controller(namespace: &str, client: Client) {
    //prepare pods API
    let pods = Api::namespaced(client.clone(), namespace).clone();

    //check if the controller pod got an annotation to terminate all supervisor pods
    let should_terminate_supervisors = check_terminate_annotation(client.clone(), namespace).await;
    if should_terminate_supervisors {
        //got a signal to terminate all supervisors
        println!("terminate-supervisor=true label detected...");
        terminate_supervisors(pods.clone()).await;
        println!("Terminating controller in order to remove terminate-all label...");
        std::process::exit(1);
    }

    //prepare controller for supervisor pods
    let controller = Controller::new(
        pods.clone(),
        watcher::Config {
            label_selector: Some("supervisor=true".to_string()),
            ..watcher::Config::default()
        },
    )
    .shutdown_on_signal();

    controller
        .run(
            reconcile,
            error_policy,
            Arc::new(ReconcileContext {
                pods: Arc::new(pods),
            }),
        )
        .for_each(|event| async {
            match event {
                Ok(obj) => {
                    println!(
                        "Reconcile triggered for pod: {}/{}",
                        obj.0.namespace.as_deref().unwrap_or("<no-ns>"),
                        obj.0.name
                    );
                }
                Err(e) => {
                    println!("Controller error: {:?}", e);
                }
            }
        })
        .await;
}

pub async fn start_controller(k8s_params: K8sParams) {
    println!("Starting supervisor controller...");
    loop {
        println!("Calling controller...");
        controller(k8s_params.get_namespace().as_ref(), k8s_params.get_client()).await;
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
