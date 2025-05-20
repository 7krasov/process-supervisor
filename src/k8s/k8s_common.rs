use kube::Client;

const ENV_HOSTNAME: &str = "HOSTNAME";
const NAMESPACE_FILE: &str = "/var/run/secrets/kubernetes.io/serviceaccount/namespace";

#[derive(Clone)]
pub struct K8sParams {
    pod_name: String,
    namespace: String,
    client: Client,
}

impl K8sParams {
    pub fn new(pod_name: String, namespace: String, client: Client) -> Self {
        K8sParams {
            pod_name,
            namespace,
            client,
        }
    }

    pub fn get_pod_name(&self) -> String {
        self.pod_name.clone()
    }
    pub fn get_namespace(&self) -> String {
        self.namespace.clone()
    }
    pub fn get_client(&self) -> Client {
        self.client.clone()
    }
}

pub async fn get_k8s_params() -> K8sParams {
    let pod_name = get_pod_name();
    if pod_name.is_err() {
        println!("Unable to get pod name");
        std::process::exit(1);
    }
    let pod_name = pod_name.unwrap();

    let namespace = get_namespace().await;
    if namespace.is_err() {
        println!("Unable to get namespace");
        std::process::exit(1);
    }
    let namespace = namespace.unwrap();

    //K8s client
    let client = get_client().await;
    if client.is_err() {
        println!("Unable to create kube client");
        std::process::exit(1);
    }
    let client = client.unwrap();

    K8sParams::new(pod_name, namespace, client)
}

fn get_pod_name() -> Result<String, std::io::Error> {
    let pod_name = std::env::var(ENV_HOSTNAME);

    match pod_name {
        Ok(pn) => Ok(pn),
        Err(_) => {
            println!("Unable to get {} env variable value", ENV_HOSTNAME);
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to get pod name",
            ))
        }
    }
}

async fn get_namespace() -> Result<String, std::io::Error> {
    let namespace = tokio::fs::read_to_string(NAMESPACE_FILE).await;

    match namespace {
        Ok(ns) => Ok(ns.trim().to_string()),
        Err(_) => {
            println!("Unable to get namespace");
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unable to get namespace",
            ))
        }
    }
}

async fn get_client() -> Result<Client, kube::Error> {
    let client = Client::try_default().await;

    match client {
        Ok(c) => Ok(c),
        Err(e) => {
            println!("Unable to create kube client: {:?}", e);
            Err(e)
        }
    }
}
