use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use log::warn;

use sim::{CmdArgs, get_cmd_args, start_matcher, start_user};
use sim::model::domain::OrderBook;

#[allow(unused_imports)]

///The entry point for the sim module. The user input thread and matcher thread are started form here
fn main() {
    env_logger::init();

    let (tx1, rx1) = std::sync::mpsc::channel();
    let (tx2, rx2) = std::sync::mpsc::channel();
    let cmd_args = match get_cmd_args() {
        Ok(cmd) => cmd,
        Err(e) => {
            warn!("No Order File or algo provided. Defaulting to FIFO matcher with an empty order book {}",e );
            CmdArgs {
                file_path: String::new(),
                algo: "FIFO".to_string(),
            }
        }
    };
    let matcher = std::thread::spawn(move || {
        start_matcher(&tx2, &rx1,cmd_args.algo);
    });
    let user = std::thread::spawn(move || {
        start_user(&tx1, &rx2, cmd_args.file_path);
    });

    user.join().expect("error");
    matcher.join().expect("error");
}

