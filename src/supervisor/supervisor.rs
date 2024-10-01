use std::collections::HashMap;
use std::fmt;
use std::io::Error;
use std::sync::Arc;
use std::time::SystemTime;
use nix::unistd::Pid;
use nix::sys::signal::{self};
use tokio::sync::RwLock;
use std::process::{Child, Command};
use procfs::process::Process;
use tokio::task;
use serde::Serialize;
use tokio::time::{sleep, sleep_until, Duration, Instant};

use crate::supervisor::results::TerminateResult;

use super::results::{KillResult, LaunchResult, OldKillResult};

const SIGTERM_TIMEOUT_SECS: u64 = 20;

#[derive(Debug, Serialize)]
pub struct ChildState {
    source_id: i32,
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
    processes: Arc<RwLock<HashMap<i32, Child>>>,
    kill_queue: Arc<RwLock<HashMap<i32, u64>>>,
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            kill_queue: Arc::new(RwLock::new(HashMap::new())),
        }
    }

	pub async fn launch(&self, source_id: i32) -> LaunchResult {
        let mut command = Command::new("php");
        command.arg("worker/worker.php");

        let mut result = LaunchResult::new();

        let spawn_result = command.spawn();
        match spawn_result {
            Ok(child) => {
                let pid = child.id();
                let processes_arc = self.processes.clone();
                processes_arc.write().await.insert(source_id, child);
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

    pub async fn terminate(&self, source_id: i32) -> TerminateResult {
        
        let before_time = Instant::now();

        let processes_arc = self.processes.clone();
        let mut processes_guard = processes_arc.write().await;
        
        //extract child PID from the processes
        let child = processes_guard.get_mut(&source_id);

        println!("terminate: After getting of child from the process list time: {:?}", Instant::now().duration_since(before_time));

        let mut result = TerminateResult::new();
        if child.is_none() {
            result.set_error("Child not found PID for SIGTERM sending".to_string());
            return result;
        }
        let child = child.unwrap();
        let pid: i32 = child.id() as i32;

        // drop(processes_guard);
    
        println!("terminate: After dropping time: {:?}", Instant::now().duration_since(before_time));

        println!("terminate: Sending SIGTERM to PID: {}", pid);      
        let signal_result = signal::kill(Pid::from_raw(pid), signal::SIGTERM);
        println!("terminate: After SIGTERM sending: {:?}", Instant::now().duration_since(before_time));
    
        return match signal_result {
            Ok(_) => {
                let start = SystemTime::now();
                self.kill_queue.clone().write().await.insert(
                    source_id,
                    start.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
                );
                result.set_success();
                result
            },
            Err(e) => {
                result.set_error(e.to_string());
                result
            }
        };
        
    }

    pub async fn kill_old(&self, source_id: i32) -> OldKillResult {
        
        let before_time = Instant::now();

        let processes_arc = self.processes.clone();
        let mut processes_guard = processes_arc.write().await;
        
        //extract child PID from the processes
        let child = processes_guard.get_mut(&source_id);

        println!("kill: After getting of child from the process list time: {:?}", Instant::now().duration_since(before_time));

        if child.is_none() {
            let mut result = OldKillResult::new();
            result.set_error("Child not found PID for SIGTERM sending".to_string());
            return result;
        }
        let child = child.unwrap();
        let pid: i32 = child.id() as i32;

        drop(processes_guard);
    
        println!("kill: After dropping time: {:?}", Instant::now().duration_since(before_time));

        println!("kill: Sending SIGTERM to PID: {}", pid);      
        signal::kill(Pid::from_raw(pid), signal::SIGTERM).unwrap();
        println!("kill: After SIGTERM sending: {:?}", Instant::now().duration_since(before_time));
        let duration = Duration::from_secs(SIGTERM_TIMEOUT_SECS);
        let deadline = Instant::now() + duration;
        println!("kill: Sleeping until: {:?}", deadline);
        sleep_until(deadline).await;
        println!("kill: After sleep_until awaiting: {:?}", Instant::now().duration_since(before_time));

        let mut result = OldKillResult::new();
        //after SIGTERM timeout we should send SIGKILL signal to make sure the process will be terminated
        let processes_arc = self.processes.clone();
        println!("kill: After new processes_arc cloning: {:?}", Instant::now().duration_since(before_time));
        let mut processes_guard = processes_arc.write().await;
        println!("kill: After new processes_guard awaiting: {:?}", Instant::now().duration_since(before_time));
        println!("kill: After new result_guard awaiting: {:?}", Instant::now().duration_since(before_time));
        //extract child PID from the processes
        let child = processes_guard.get_mut(&source_id);
        if child.is_none() {
            result.set_error("Child not found PID for SIGKILL sending".to_string());
            return result;
        }
        let child = child.unwrap();

        //send SIGKILL (9) signal
        println!("Sending SIGKILL to PID: {}", pid);
        let kill_result = child.kill();
        println!("After kill time: {:?}", Instant::now().duration_since(before_time));

        match kill_result {
            Ok(_) => {
                let exit_status = child.try_wait();
                println!("kill: After try_wait time: {:?}", Instant::now().duration_since(before_time));

                let ch = processes_guard.remove(&source_id);
                println!("After remove time: {:?}", Instant::now().duration_since(before_time));
                if ch.is_none() {
                    println!("Failed to remove child from processes for source_id: {}", source_id);
                }
                drop(processes_guard);

                match exit_status {
                    Ok(status) => {
                        match status {
                            Some(_) => {
                                println!("Status code; {:?}", status.unwrap().code());
                                result.set_success(status.unwrap().code());
                            },
                            None => {
                                //probably, the process was finished before the killing signal sending
                                result.set_success(Some(9999999));
                            }
                        };
                    },
                    Err(e) => {
                        println!("Error: {}", e);
                        result.set_error(e.to_string());
                    }
                };
            },
            Err(e) => {
                result.set_error(e.to_string());
            }
        }

        return result;
        
    }

    pub async fn kill(&self, source_id: i32, terminate_signal_time: u64) -> KillResult {
        
        let now = SystemTime::now();
        let wait_time_elapsed = now.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() - terminate_signal_time;

        if wait_time_elapsed < SIGTERM_TIMEOUT_SECS {
            println!("There is some time left before SIGKILL sending. Sleeping...");
            let sleep_time = SIGTERM_TIMEOUT_SECS - wait_time_elapsed;
            println!("kill: Sleeping seconds: {:?}", sleep_time);
            sleep(Duration::from_secs(sleep_time)).await;
            println!("kill: After sleep awaiting: {:?}", now.elapsed());
        }

        let before_time = Instant::now();

        let mut result = KillResult::new();
        //after SIGTERM timeout we should send SIGKILL signal to make sure the process will be terminated
        let processes_arc = self.processes.clone();
        println!("kill: After new processes_arc cloning: {:?}", Instant::now().duration_since(before_time));
        let mut processes_guard = processes_arc.write().await;
        println!("kill: After new processes_guard awaiting: {:?}", Instant::now().duration_since(before_time));
        //extract child PID from the processes
        let child = processes_guard.get_mut(&source_id);
        if child.is_none() {
            result.set_error("Child not found PID for SIGKILL sending.".to_string());
            return result;
        }
        let child = child.unwrap();

        let state = self.get_child_state(source_id, child).unwrap();
        if state.is_finished {
            println!("It seems the process finished itsef. Exit code: {:?}", state.exit_code);
            result.set_success(state.exit_code);
            let ch = processes_guard.remove(&source_id);
            println!("After child remove time: {:?}", Instant::now().duration_since(before_time));
                if ch.is_none() {
                    println!("Failed to remove child from processes for source_id: {}", source_id);
                }
            // drop(processes_guard);
            return result;
        }

        //send SIGKILL (9) signal
        println!("Sending SIGKILL to PID: {}", child.id());
        let kill_result = child.kill();
        println!("After kill time: {:?}", Instant::now().duration_since(before_time));

        match kill_result {
            Ok(_) => {
                let exit_status = child.try_wait();
                println!("kill: After try_wait time: {:?}", Instant::now().duration_since(before_time));

                let ch = processes_guard.remove(&source_id);
                println!("After child remove time: {:?}", Instant::now().duration_since(before_time));
                if ch.is_none() {
                    println!("Failed to remove child from processes for source_id: {}", source_id);
                }
                drop(processes_guard);

                match exit_status {
                    Ok(status) => {
                        match status {
                            Some(_) => {
                                println!("Status code; {:?}", status.unwrap().code());
                                result.set_success(status.unwrap().code());
                            },
                            None => {
                                //probably, the process was finished before the killing signal sending
                                result.set_success(Some(9999999));
                            }
                        };
                    },
                    Err(e) => {
                        println!("Error: {}", e);
                        result.set_error(e.to_string());
                    }
                };
            },
            Err(e) => {
                result.set_error(e.to_string());
            }
        }

        return result;
        
    }

    pub async fn get_state_list(self: Arc<Self>) -> HashMap<i32, ChildState> {
        let before_time = Instant::now();
        println!("get_state_list: Before self.processes.clone(), time: {:?}", Instant::now().duration_since(before_time));
        let processes_arc = self.processes.clone();
        println!("get_state_list: After self.processes.clone(), time: {:?}", Instant::now().duration_since(before_time));
        let processes_guard = processes_arc.read().await;
        println!("get_state_list: Afterprocesses_arc.read().await, time: {:?}", Instant::now().duration_since(before_time));
        let keys: Vec<i32> = processes_guard.keys().cloned().collect();
        drop(processes_guard);

        let futures: Vec<_> = keys.into_iter().map(|source_id| {
            let supervisor = self.clone();
            task::spawn(async move {
                let res = supervisor.get_process_state(source_id).await;
                match res {
                    Ok(state) => state,
                    Err(e) => {
                        println!("get_child_state returned error: {}", e);
                        ChildState {
                            source_id,
                            is_running: false,
                            is_finished: false,
                            exit_code: None,
                            is_killed: false,
                            rss_anon_memory_kb: None,
                        }
                    }
                }
            })
        }).collect();

        println!("get_state_list: After futures collect, time: {:?}", Instant::now().duration_since(before_time));

        let mut states = HashMap::new();
        for future in futures {
            match future.await {
                Ok(state) => {
                    println!("get_state_list: Before states.insert, time: {:?}", Instant::now().duration_since(before_time));
                    states.insert(state.source_id,state);
                },
                Err(e) => {
                    println!("get_state_list: Task Join Error: {}", e);
                }
            }
        }  
        
        //TODO: find failed results and fill the array with an appropriate result
        states 

        // return processes_guard.keys().map(|source_id| {
        //     self.get_child_state(*source_id).await;
        // }).collect();
        // let keys = processes_guard.keys();
        // return keys.map(|(source_id)| {
        //     self.get_child_state(*source_id).unwrap()
        // }).collect();
    }

	pub async fn get_process_state(& self, source_id: i32) -> Result<ChildState, Error> {
        let before_time = Instant::now();
        println!("get_process_state: Before self.processes.clone(), time: {:?}", Instant::now().duration_since(before_time));
		let processes_arc = self.processes.clone();
        println!("get_process_state: After self.processes.clone(), time: {:?}", Instant::now().duration_since(before_time));
		let mut processes_guard = processes_arc.write().await;
        println!("get_process_state: After processes_arc.write().await, time: {:?}", Instant::now().duration_since(before_time));
		let child = processes_guard.get_mut(&source_id).ok_or_else(|| Error::new(std::io::ErrorKind::NotFound, "Child not found"))?;
        println!("get_process_state: After processes_guard.get_mut, time: {:?}", Instant::now().duration_since(before_time));
        
        self.get_child_state(source_id, child)
	}

    fn get_child_state(& self, source_id: i32, child: &mut Child) -> Result<ChildState, Error> {
        let before_time = Instant::now();
        let exit_status = child.try_wait()?;
        println!("get_child_state: After child.try_wait()?, time: {:?}", Instant::now().duration_since(before_time));
		let is_finished = exit_status.is_some();
		let exit_code = exit_status.and_then(|status| status.code());
		Ok(ChildState {
            source_id,
			is_running: !is_finished,
			is_finished,
			exit_code,
			is_killed: false,
			rss_anon_memory_kb: self.get_memory_usage(child.id()).ok()
		})
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

    pub async fn process_kill_queue(&self) {
        let option: Option<(i32, u64)> = self.pop_kill_queue().await;
        if option.is_none() {
            return;
        }

        let (source_id, terminate_signal_time) = option.unwrap();
        self.kill(source_id, terminate_signal_time).await;
    }

    pub async fn pop_kill_queue(&self) -> Option<(i32, u64)> {
        let mut kill_queue_guard = self.kill_queue.write().await;
        let (source_id, terminate_signal_time) = kill_queue_guard.iter().next().map(|(k, v)| (*k, *v))?;
        kill_queue_guard.remove(&source_id);
        // drop(kill_queue_guard);
        Some((source_id, terminate_signal_time))
    }
}

impl Clone for Supervisor {
    fn clone(&self) -> Self {
        Self {
            processes: Arc::clone(&self.processes),
            kill_queue: Arc::clone(&self.kill_queue),
        }
    }
}
