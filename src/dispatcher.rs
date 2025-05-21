use serde_derive::{Deserialize, Serialize};

// const OBTAIN_PROCESS_URL: &str = "http://httpbin.org/headers";
const OBTAIN_PROCESS_URL: &str =
    "processing-dispatcher-service.processing-dispatcher.svc.cluster.local/obtain_new_process";
const REPORT_PROCESS_FINISH_URL: &str =
    "processing-dispatcher-service.processing-dispatcher.svc.cluster.local/report_process_finish/{process_id}";

pub const REPORT_STATUS_SUCCESS: &str = "success";
pub const REPORT_STATUS_ERROR: &str = "error";

#[derive(Deserialize)]
pub struct NewProcess {
    id: String,
    source_id: i32,
}

impl NewProcess {
    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn source_id(&self) -> i32 {
        self.source_id
    }
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

fn get_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .unwrap()
}

pub async fn obtain_new_process() -> Result<NewProcess, ProcessDispatcherClientError> {
    let resp = get_client().get(OBTAIN_PROCESS_URL).send().await;
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
    let new_process_result: serde_json::Result<NewProcess> = serde_json::from_str(&resp_text);

    if new_process_result.is_err() {
        let err = new_process_result.err().unwrap();
        println!("Failed to parse response: {:?}. Data: {:?}", err, resp_text);
        // return Err(Box::new(err));
        return Err(ProcessDispatcherClientError::ParseError(format!(
            "Failed to parse response: {:?}",
            err,
        )));
    }
    Ok(new_process_result.unwrap())
}

#[derive(Serialize)]
pub struct ProcessFinishReport {
    process_id: String,
    result: String,
}

impl ProcessFinishReport {
    pub fn new(process_id: String, result: String) -> Self {
        ProcessFinishReport { process_id, result }
    }
}

pub async fn report_process_finish(
    report: ProcessFinishReport,
) -> Result<(), ProcessDispatcherClientError> {
    let url = REPORT_PROCESS_FINISH_URL.replace("{process_id}", &report.process_id);
    let response = get_client().patch(&url).json(&report).send().await;

    if response.is_err() {
        let err = response.err().unwrap();
        return Err(ProcessDispatcherClientError::NetworkProblem(format!(
            "Failed to report process finish: {:?}",
            err,
        )));
    }
    Ok(())
}
