use std::collections::{HashMap, VecDeque};
use std::fs::OpenOptions;
use std::io::Write;
use std::str::FromStr;
use std::sync::Mutex;

use env_logger::Env;
use log::Log;
use rocket::FromForm;
use rocket::response::content::RawHtml;
use serde::{Deserialize, Serialize};
use serde_json::to_string;

use sim::common::utils;
use sim::common::utils::{log, Sigma};
use sim::matchers::fifo_matcher::FIFOMatcher;
use sim::matchers::matcher::Matcher;
use sim::matchers::prorata_matcher::ProrataMatcher;
use sim::model::domain::{OrderBook, OrderBookKey, OrderSingle};
use sim::model::domain::Side::{Buy, Sell};

const LOG_FILE: &str = "web/logs/web.log";

#[derive(Debug, Clone, Serialize, FromForm)]
pub struct Order {
    symbol: String,
    qty: u32,
    price: f64,
    side: String,
    order_type: String,
    cl_ord_id: String,
    format: String,
}

impl Order {
    pub fn symbol(&self) -> &String {
        &self.symbol
    }

    pub fn qty(&self) -> u32 {
        self.qty
    }

    pub fn price(&self) -> f64 {
        self.price
    }

    pub fn side(&self) -> &String {
        &self.side
    }

    pub fn order_type(&self) -> &String {
        &self.order_type
    }

    pub fn cl_ord_id(&self) -> &String {
        &self.cl_ord_id
    }

    pub fn format(&self) -> &String {
        &self.format
    }
}

pub fn init_logger() {
    let log_file_path = "logs/web.log";
    env_logger::Builder::from_env(Env::default().default_filter_or("debug"))
        .format_timestamp(None)
        .write_style(env_logger::WriteStyle::Always)
        .init();
}


#[derive(Serialize, Debug, Deserialize)]
pub struct OB {
    pub buy_orders: HashMap<String, VecDeque<OrderSingle>>,
    pub sell_orders: HashMap<String, VecDeque<OrderSingle>>,
}

impl OB {
    pub fn from(order_book: &OrderBook) -> Self {
        let buy = order_book.get_orders_for(Buy);
        let sell = order_book.get_orders_for(Sell);
        let mut buy_orders = HashMap::new();
        add_string_keys(&mut buy_orders, &buy);
        let mut sell_orders = HashMap::new();
        add_string_keys(&mut sell_orders, &sell);
        OB {
            buy_orders,
            sell_orders,
        }
    }

    pub fn to(ob: &OB) -> OrderBook {
        let mut buy = HashMap::new();
        add_order_book_keys(&mut buy, &ob.buy_orders);
        let mut sell = HashMap::new();
        add_order_book_keys(&mut sell, &ob.sell_orders);
        OrderBook::new(buy, sell)
    }
}

fn add_order_book_keys(target: &mut HashMap<OrderBookKey, VecDeque<OrderSingle>>, source: &HashMap<String, VecDeque<OrderSingle>>) {
    for (key, val) in source {
        let v: Vec<&str> = key.split('_').collect();
        let symbol = v[0].to_string();
        let price = f64::from_str(v[1]).unwrap();
        let key = OrderBookKey::new(price, symbol);
        target.insert(key, val.clone());
    }
}

fn add_string_keys(target: &mut HashMap<String, VecDeque<OrderSingle>>, source: &HashMap<OrderBookKey, VecDeque<OrderSingle>>) {
    for (key, val) in source {
        let mut k = key.symbol().to_string();
        k.push('_');
        k.push_str(key.price().to_string().as_str());
        target.insert(k, val.clone());
    }
}

pub fn persist_order_book(ob: &OB,path:&str) {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true) // Create the file if it doesn't exist
        .truncate(true) //overwrite content
        .open(path).unwrap();
    let content = to_string(&ob).unwrap();

    let mut file_lock = Mutex::new(file);
    {
        let mut file = file_lock.lock().unwrap();
        file.write_all(content.as_bytes()).expect("Error writing");
        file.flush().expect("error flushing");
    }
}

pub fn create_order_book_table(order_book: &OrderBook) -> RawHtml<String> {
    let buy = order_book.get_orders_for(Buy);
    let mut html = String::from("<h3>Order Book </h3>");
    html.push_str("<Table>");
    html.push_str("<tr><td>Symbol</td><td>Quantity</td><td>Price</td><td>Side</tr>");
    for (key, value) in buy {
        add_html(&key, &value, &mut html, "Buy");
    }
    html.push_str("</p>");
    let sell = order_book.get_orders_for(Sell);
    for (key, value) in sell {
        add_html(&key, &value, &mut html, "Sell");
    }
    html.push_str("</table>");
    log(&html, LOG_FILE);
    RawHtml(html)
}

fn add_html(key: &OrderBookKey, orders: &VecDeque<OrderSingle>, html: &mut String, side: &str) {
    let v: Vec<OrderSingle> = orders.clone().into_iter().collect();
    let total = utils::Aggregator::sigma(&v);
    let px = key.price();
    let row = format!("<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>", key.symbol(), total, key.price(), side);
    html.push_str(row.as_str());
}

pub fn get_matcher(algo:&String) -> Box<dyn Matcher>{

    if algo == "FIFO" {
        Box::new(FIFOMatcher::new())
    }else{
        Box::new(ProrataMatcher::new())
    }

}


#[cfg(test)]
mod tests {
    use serde_json::{from_str, to_string};

    use sim::common::utils::create_order_book;
    use sim::common::utils::read_input;
    use sim::model::domain::OrderBook;
    use sim::model::domain::Side::{Buy, Sell};

    use crate::OB;

    #[test]
    fn test_to() {
        let content = std::fs::read_to_string("test_data/ob.json").unwrap();
        let ob: OB = from_str(&content).unwrap();
        let mut order_book = OB::to(&ob);
        assert_eq!(order_book.get_orders_for(Buy).len(), 1);
        assert_eq!(order_book.get_orders_for(Sell).len(), 1);
        order_book.pretty_print_self();
    }

    #[test]
    fn test_from() {
        let mut order_book = OrderBook::default();
        create_order_book(&mut order_book, read_input("test_data/orders.txt"));
        assert_eq!(order_book.get_orders_for(Buy).len(), 1);
        assert_eq!(order_book.get_orders_for(Sell).len(), 1);
        let ob = OB::from(&order_book);
        let json = to_string(&ob).unwrap();
        assert!(!json.is_empty());
        println!("{}", json);
    }
}



