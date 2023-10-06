extern crate log;

use std::{env, io, process};
use std::io::BufRead;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::sleep;
use std::time::Duration;

use colored::Colorize;
use log::{error, info};

use crate::common::utils;
use crate::matchers::fifo_matcher::FIFOMatcher;
use crate::matchers::matcher::Matcher;
use crate::matchers::prorata_matcher::ProrataMatcher;
use crate::model::domain::OrderBook;

pub mod model;
pub mod matchers;
pub mod formatters;
pub mod common;




pub struct CmdArgs {
    pub file_path: String,
    pub algo: String,
}

fn print_help() {
    println!("Usage:");
    println!("If using cargo:");
    println!("cargo -- -h for help");
    println!("cargo run <Enter> for starting without an orderbook or algo param");
    println!("cargo run -- <order_file_path> <algo(FIFO|PRO>");
    println!();
    println!("If using executable:");
    println!("exchange_simulator <Enter> for starting without an orderbook or algo param");
    println!("exchange_simulator.exe -h for help");
    println!("exchange_simulator.exe <order_file+path> <algo(FIFO|PRO>");
    process::exit(0);
}
///This function sits in a loop accepting user input for matching until the user quits
/// User inputs are added to the order book and sent to the matcher for matching
pub fn start_user(tx: &Sender<OrderBook>, rx: &Receiver<OrderBook>, file_path: String) {
    if file_path == "-h" {
        print_help();
    }
    let input = utils::read_input(&file_path);
    let mut ob = utils::create_order_book(input);
    if let Err(e) = tx.send(ob) {
        error!("Error sending file {}",e);
    }
    sleep(Duration::from_secs(1));
    loop {
        let mut order_book = match rx.recv() {
            Ok(ob) => ob,
            Err(e) => {
                error!("Error receiving order book {}",e);
                OrderBook::default()
            }
        };

        order_book.pretty_print_self();
        info!("Enter an order({} ) to match  or {} to quit","<id> <symbol> <qty> <price> <side(Buy|Sell)>".bold().reversed(), "q".bold().red());
        println!("");
        let mut line = String::new();
        io::stdin().read_line(&mut line).unwrap();
        if line.chars().nth(0) == Some('q') {
            process::exit(0);
        }
        let order = utils::create_order_from_string(line.trim_end().to_string());
        order_book.add_order_to_order_book(order);
        if let Err(e) = tx.send(order_book) {
            error!("Error sending order book {}",e);
        };
        sleep(Duration::from_secs(1));
    }
}

/// creates the matcher based on the algo and starts the matcher which spins in a loop matching the order book as and when available
/// C
pub fn start_matcher(tx: &Sender<OrderBook>, rx: &Receiver<OrderBook>, algo: String) {
    if algo == "PRO" {
        let mut matcher = ProrataMatcher;
        matcher.start(&tx, &rx);
    } else {
        let mut matcher = FIFOMatcher;
        matcher.start(&tx, &rx);
    }
}

pub fn get_cmd_args() -> Result<CmdArgs, &'static str> {
    let mut args = env::args();
    args.next();
    let file_path = match args.next() {
        Some(path) => path,
        None => return Err("No order file provided"),
    };

    let algo = match args.next() {
        Some(arg) => arg,
        None => String::from("FIFO")
    };

    let cmd_args = CmdArgs {
        file_path: file_path,
        algo: algo,
    };
    Ok(cmd_args)
}



