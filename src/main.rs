mod supervisor {
    pub mod supervisor;
}
use crate::supervisor::supervisor::Supervisor;

use std::{thread::sleep, time::Duration};


fn main() {

    let mut lnchr = Supervisor::new();
    lnchr.launch(2506);
    
    let state = lnchr.get_child_state(2506);
    
    match state {
        Ok(state) => println!("{}", state),
        Err(e) => println!("Error: {}", e),
    }

    //wait until process will be run
    sleep(Duration::from_secs(1));

    let kill_state = lnchr.kill(2506);

    match kill_state {
        Ok(kill_state) => println!("{}", kill_state),
        Err(e) => println!("Error: {}", e),
    }
}