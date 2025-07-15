use crate::env::EnvParams;
use k8s_openapi::chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};

pub const DEFAULT_OBTAIN_PROCESS_URL: &str =
    "processing-dispatcher-service.processing-dispatcher.svc.cluster.local/obtain_new_process/{supervisor_id}";
pub const DEFAULT_REPORT_PROCESS_FINISH_URL: &str =
    "processing-dispatcher-service.processing-dispatcher.svc.cluster.local/report_process_finish/{process_id}";

pub const REPORT_STATUS_SUCCESS: &str = "success";
pub const REPORT_STATUS_ERROR: &str = "error";

//TODO: make a workspace from the project and move the dispatcher code to a separate crate here,
//as dispatcher code contracts (responses) and constants are used within another project
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub enum DispatchState {
    Created,
    Pending,
    Processing,
    Error,
    Completed,
    Failed,
}

const PROCESSING_MODE_REGULAR: isize = 1;
const PROCESSING_MODE_SANDBOX: isize = 2;

#[derive(PartialEq, Serialize, Deserialize, Debug)]
pub enum ProcessingMode {
    Regular = PROCESSING_MODE_REGULAR,
    Sandbox = PROCESSING_MODE_SANDBOX,
}

impl ProcessingMode {
    pub fn new(value: isize) -> ProcessingMode {
        match value {
            PROCESSING_MODE_REGULAR => ProcessingMode::Regular,
            PROCESSING_MODE_SANDBOX => ProcessingMode::Sandbox,
            _ => panic!("Unexpected ProcessingMode value"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AssignedProcess {
    pub id: String,
    pub source_id: u32,
    pub state: DispatchState,
    #[serde(rename = "mode")]
    pub r#mode: ProcessingMode,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    pub supervisor_id: String,
}

#[derive(Debug)]
pub enum ProcessDispatcherClientError {
    #[allow(dead_code)]
    NetworkProblem(String),
    #[allow(dead_code)]
    BadResponseBody(String),
    #[allow(dead_code)]
    ParseError(String),
}

fn get_request_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap()
}

#[derive(Debug, Clone)]
pub struct DispatcherClient {
    obtain_process_url: String,
    report_process_finish_url: String,
    supervisor_id: String,
}

impl DispatcherClient {
    pub fn new(env_params: &EnvParams) -> Self {
        DispatcherClient {
            obtain_process_url: env_params.obtain_process_url().into(),
            report_process_finish_url: env_params.report_process_finish_url().into(),
            supervisor_id: env_params.supervisor_id().into(),
        }
    }

    pub async fn obtain_new_process(
        &self,
    ) -> Result<AssignedProcess, ProcessDispatcherClientError> {
        println!(
            "Obtaining new process for supervisor: {}...",
            self.supervisor_id
        );
        let resp = get_request_client()
            .get(
                self.obtain_process_url
                    .replace("{supervisor_id}", &self.supervisor_id),
            )
            .send()
            .await;
        if resp.is_err() {
            let err = resp.err().unwrap();
            return Err(ProcessDispatcherClientError::NetworkProblem(format!(
                "Failed to fetch data from dispatcher: {:?}",
                err,
            )));
        }
        let resp_text_result = resp.unwrap().text().await;
        if resp_text_result.is_err() {
            let err = resp_text_result.err().unwrap();
            return Err(ProcessDispatcherClientError::BadResponseBody(format!(
                "Failed to get response body string: {:?}",
                err,
            )));
        }
        let resp_text = resp_text_result.unwrap();
        let process_result: serde_json::Result<AssignedProcess> = serde_json::from_str(&resp_text);

        if process_result.is_err() {
            let err = process_result.err().unwrap();
            println!("Failed to parse response: {:?}. Data: {:?}", err, resp_text);
            // return Err(Box::new(err));
            return Err(ProcessDispatcherClientError::ParseError(format!(
                "Failed to parse response: {:?}",
                err,
            )));
        }

        Ok(process_result.unwrap())
    }

    pub async fn report_process_finish(
        &self,
        report: ProcessFinishReport,
    ) -> Result<(), ProcessDispatcherClientError> {
        println!("Sending process finish report: {:?}...", report);
        let url = self
            .report_process_finish_url
            .replace("{process_id}", &report.process_id);
        let response = get_request_client().patch(&url).json(&report).send().await;

        if response.is_err() {
            let err = response.err().unwrap();
            return Err(ProcessDispatcherClientError::NetworkProblem(format!(
                "Failed to report process finish: {:?}",
                err,
            )));
        }
        Ok(())
    }
}

#[derive(Serialize, Debug)]
pub struct ProcessFinishReport {
    process_id: String,
    result: String,
}

impl ProcessFinishReport {
    pub fn new(process_id: String, result: String) -> Self {
        ProcessFinishReport { process_id, result }
    }
}
