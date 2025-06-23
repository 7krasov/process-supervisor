use std::env;

pub struct EnvParams {
    http_port: u16,
    sigterm_timeout_secs: u64,
    max_children_count: usize,
}

impl EnvParams {
    pub fn http_port(&self) -> u16 {
        self.http_port
    }

    pub fn sigterm_timeout_secs(&self) -> u64 {
        self.sigterm_timeout_secs
    }

    pub fn max_children_count(&self) -> usize {
        self.max_children_count
    }
}

pub fn fetch_env_params() -> EnvParams {
    let http_port: u16 = match env::var("HTTP_PORT") {
        Ok(port) => port.parse::<u16>().unwrap(),
        Err(_) => {
            println!("HTTP_PORT is not set. Using default 8080");
            8080
        }
    };

    let sigterm_timeout_secs: u64 = match env::var("SIGTERM_TIMEOUT_SECS") {
        Ok(timeout) => timeout.parse::<u64>().unwrap(),
        Err(_) => {
            println!("SIGTERM_TIMEOUT_SECS is not set. Using default 20");
            20
        }
    };

    let max_children_count: usize = match env::var("MAX_CHILDREN_COUNT") {
        Ok(count) => count.parse::<usize>().unwrap(),
        Err(_) => {
            println!("MAX_CHILDREN_COUNT is not set. Using default 10");
            10
        }
    };
    EnvParams {
        http_port,
        sigterm_timeout_secs,
        max_children_count,
    }
}
