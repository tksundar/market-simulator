use std::fs::File;
use std::io::{BufRead, BufReader};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use log::{error, trace};
use rand::Rng;

use crate::model::domain::{Fill, OrderBook, OrderSingle, OrderType, Side};

pub struct Aggregator;

pub trait Sigma<T> {
    fn sigma(items: &Vec<T>) -> u32;
}

impl Sigma<OrderSingle> for Aggregator {
    fn sigma(items: &Vec<OrderSingle>) -> u32 {
        let mut sum = 0;
        for order in items {
            sum += order.qty();
        }

        sum
    }
}

impl Sigma<Fill> for Aggregator {
    fn sigma(items: &Vec<Fill>) -> u32 {
        let mut sum = 0;
        for fill in items {
            sum += fill.qty();
        }

        sum
    }
}

pub fn read_input(file_path: &str) -> Vec<String> {
    trace!("reading file {file_path}");
    if file_path.is_empty() {
        return vec![];
    }
    let file = File::options().read(true).open(file_path).unwrap();
    let reader = BufReader::new(file);
    let mut lines = vec![];
    for line in reader.lines() {
        lines.push(line.unwrap());
    }
    lines
}


pub fn generate_id() -> String {
    let num: u64 = rand::thread_rng().gen_range(1..=1000000);
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
    let id = duration.as_secs() * 1000 + duration.subsec_millis() as u64;
    (num + id).to_string()
}

pub fn create_order_book(order_book: &mut OrderBook, input: Vec<String>) {
    if input.len() > 1 {
        trace!("Creating order book");
        for line in input {
            let order = create_order_from_string(line);
            if order.is_valid() {
                order_book.add_order_to_order_book(order);
            }
        }
    }
}

pub fn create_order_from_string(line: String) -> OrderSingle {
    let tokens: Vec<&str> = line.split(" ").collect();
    if tokens.len() != 5 {
        error!("\nInput should contain 5 fields <cl_ord_id symbol qty px side>. Sending order book for matching");
        OrderSingle::default()
    } else {
        let _order = OrderSingle::default();
        trace!("received vector {:?}",tokens);
        let cl_ord_id = String::from(tokens[0]);
        let symbol = String::from(tokens[1]);
        let qty = u32::from_str(tokens[2]).unwrap();
        let price = f64::from_str(tokens[3]).unwrap();
        let order_side = tokens[4];
        trace!("order side = {order_side}");
        let side = if order_side == "Buy" { Side::Buy } else { Side::Sell };
        trace!("side is {}",side.string_value());
        OrderSingle::new(qty, symbol, price, side, OrderType::Limit, cl_ord_id)
    }

    // order
}


#[cfg(test)]
mod tests {
    use crate::model::domain::{OrderBook, OrderBookKey};
    use crate::model::domain::Side::Buy;
    use crate::utils::{create_order_book, read_input};

// use super::*;

    #[test]
    fn test_create_order_book() {
        let order_book = create_test_order_book();
        let (buy, sell) = order_book.get_orders_for_matching(Buy);
        assert_eq!(buy.len(), 2);
        assert_eq!(sell.len(), 2);
    }

    #[test]
    fn test_values() {
        let order_book = create_test_order_book();
        let (buy, sell) = order_book.get_orders_for_matching(Buy);
        let key1 = OrderBookKey::new(601.5, "IBM".to_string());
        let key2 = OrderBookKey::new(601.1, "IBM".to_string());
        let key3 = OrderBookKey::new(602.5, "IBM".to_string());
        let orders = buy.get(&key1).unwrap();
        println!("{:#?}", orders);
        assert_eq!(orders.iter().len(), 4);
        let orders = buy.get(&key2);
        assert_eq!(orders.iter().len(), 1);

        let orders = sell.get(&key1);
        assert_eq!(orders.iter().len(), 1);

        let orders = sell.get(&key3);
        assert_eq!(orders.iter().len(), 1);
    }

    fn create_test_order_book() -> OrderBook {
        let input = read_input("fifo_test_data/orders.txt");
        let mut order_book = OrderBook::default();
        create_order_book(&mut order_book, input);
        order_book
    }
}