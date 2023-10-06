use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::str::FromStr;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local};
use log::{error, trace};
use rand::Rng;

use crate::model::domain::{Fill, OrderBook, OrderSingle, OrderType, Side};

pub struct Aggregator;


pub trait Sigma<T> {
    ///Aggregates quantities in a vec of orders or fills
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

///Reads the orders from a file and creates a [`Vec<String>`], one entry per order
/// # Example
///```rust
/// use matching_engine::common::utils::read_input;
/// let input = read_input("test_data/orders.txt");
/// ```
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

///Generates a unique id
pub fn generate_id() -> String {
    let num: u64 = rand::thread_rng().gen_range(1..=1000000);
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
    let id = duration.as_secs() * 1000 + duration.subsec_millis() as u64;
    (num + id).to_string()
}

///Creates an [`OrderBook`] with the orders in the input vector
/// # Example:
///```rust
/// use matching_engine::common::utils::{create_order_book, read_input};
/// let input = read_input("test_data/orders.txt");
/// let mut order_book = create_order_book(input);
pub fn create_order_book(input: Vec<String>) -> OrderBook {
    let mut order_book = OrderBook::default();
    if input.len() > 1 {
        trace!("Creating order book");
        for line in input {
            let order = create_order_from_string(line);
            if order.is_valid() {
                order_book.add_order_to_order_book(order);
            }
        }
    }
    order_book
}
///Creates an Order from the string
/// # Example
///```rust
/// let order_string = "test1 IBM 100 150 Buy";
/// use matching_engine::common::utils::create_order_from_string;
/// let order = create_order_from_string(order_string.to_string());
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
/// logs to a file. Use appropriate logger back end to log messages to a file
#[deprecated]
pub fn log(message: &String, log_file: &str) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(log_file).unwrap();
    let utc: DateTime<Local> = Local::now();

    let formatted_timestamp = utc.format("%Y/%m/%d/%H:%M:%S:%3f").to_string();

    let mut log_message = format!("\n{}-{}", formatted_timestamp, message);
    let file_lock = Mutex::new(file);
    {
        let mut file_guard = file_lock.lock().unwrap();
        file_guard.write_all(log_message.as_bytes()).expect("error writing log");
        file_guard.flush().expect("error flushing");
    }
}


#[cfg(test)]
mod tests {

    use crate::common::utils::{create_order_from_string};
    use crate::model::domain::{OrderBook, OrderBookKey};
    use crate::model::domain::Side::Buy;



    #[test]
    fn test_values() {
        let order1 = "id1 IBM 300 602.5 Buy";
        let order2 = "id2 IBM 300 601.1 Buy";
        let order3 = "id3 IBM 300 601.1 Buy";
        let order4 = "id4 IBM 300 601.9 Buy";
        let mut order_book =OrderBook::default();
        order_book.add_order_to_order_book(create_order_from_string(order1.to_string()));
        order_book.add_order_to_order_book(create_order_from_string(order2.to_string()));
        order_book.add_order_to_order_book(create_order_from_string(order3.to_string()));
        order_book.add_order_to_order_book(create_order_from_string(order4.to_string()));
        let key1 = OrderBookKey::new(602.5, "IBM".to_string());
        let key2 = OrderBookKey::new(601.1, "IBM".to_string());
        let key3 = OrderBookKey::new(601.9, "IBM".to_string());
        let buy = order_book.get_orders_for(Buy);
        let orders = buy.get(&key1).unwrap();
        assert_eq!(buy.len(),3);

        assert_eq!(orders.iter().len(), 1);
        let orders = buy.get(&key2).unwrap();

        assert_eq!(orders.iter().len(), 2);

        let orders = buy.get(&key3).unwrap();
        assert_eq!(orders.iter().len(), 1);

    }
}
