use std::collections::HashMap;
use std::fmt;
use std::io::Error;
use std::sync::{Arc, Mutex};
use std::process::{Child, Command};
use procfs::process::Process;

use super::results::{KillResult, LaunchResult};


#[derive(Debug)]
pub struct ChildState {
    is_running: bool,
    is_finished: bool,
    exit_code: Option<i32>,
	is_killed: bool,
	rss_anon_memory_kb: Option<u64>
}

impl fmt::Display for ChildState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "is_running: {}, is_finished: {}, exit_code: {:?}, is_killed: {}, rss_anon_memory_kb: {:?}",
               self.is_running, self.is_finished, self.exit_code, self.is_killed, self.rss_anon_memory_kb)
    }
}

#[derive(Debug)]
pub struct Supervisor {
    processes: Arc<Mutex<HashMap<i32, Child>>>
}

impl Supervisor {
    pub fn new() -> Self {
        Self { processes: Arc::new(Mutex::new(HashMap::new()))  }
    }

	pub fn launch(&mut self, source_id: i32) -> LaunchResult {
        let mut command = Command::new("php");
        command.arg("worker/worker.php");

        let mut result = LaunchResult::new();

        let spawn_result = command.spawn();
        match spawn_result {
            Ok(child) => {
                let pid = child.id();
                let cloned_processes = self.processes.clone();
                cloned_processes.lock().unwrap().insert(source_id, child);
                result.set_success(pid);
                return result;
            },
            Err(e) => {
                // println!("Failed to start command");
                result.set_error(e.to_string());
                return result;
            }
        }
    }

	pub fn get_child_state(& self, source_id: i32) -> Result<ChildState, Error> {
		let cloned_processes = self.processes.clone();
		let mut processes_guard = cloned_processes.lock().unwrap();
		let child = processes_guard.get_mut(&source_id).ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, "Child not found"))?;
		let exit_status = child.try_wait()?;
		let is_finished = exit_status.is_some();
		let exit_code = exit_status.and_then(|status| status.code());
		Ok(ChildState {
			is_running: !is_finished,
			is_finished,
			exit_code,
			is_killed: false,
			rss_anon_memory_kb: self.get_memory_usage(child.id()).ok()
		})
	}

    pub fn kill(& self, source_id: i32) -> KillResult {
        let mut result = KillResult::new();

        let cloned_processes = self.processes.clone();
        let mut processes_guard = cloned_processes.lock().unwrap();
        
        let child = processes_guard.get_mut(&source_id);
        if child.is_none() {
            result.set_error("Child not found".to_string());
            return result;
        }
        let child = child.unwrap();
        let kill_result = child.kill();

        match kill_result {
            Ok(_) => {
                let exit_status = child.try_wait();

                let ch = processes_guard.remove(&source_id);
                if ch.is_none() {
                    println!("Failed to remove child from processes for source_id: {}", source_id);
                }

                match exit_status {
                    Ok(status) => {
                        match status {
                            Some(_) => {
                                result.set_success(status.unwrap().code());
                            },
                            None => {
                                //probably, the process was finished before the killing signal sending
                                result.set_success(Some(9999999));
                            }
                        }
                    },
                    Err(e) => {
                        println!("Error: {}", e);
                        result.set_success(Some(9999999));
                    }
                }
                return result;
            },
            Err(e) => {
                result.set_error(e.to_string());
                return result;
            }
        }
    }

    //returns size in kilobytes
    fn get_memory_usage(& self, pid: u32) -> std::io::Result<u64> {

        let process = Process::new(pid as i32);
        if process.is_err() {
            return Ok(0);
        }
        let process = process.unwrap();

        let status = process.status();
        if status.is_err() {
            return Ok(0);
        }
        let status = status.unwrap();
        let rssanon = status.rssanon;
        if rssanon.is_none() {
            return Ok(0);
        }
        Ok(status.rssanon.unwrap())
	}

}
