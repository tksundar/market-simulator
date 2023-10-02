use std::collections::{HashMap, VecDeque};
use std::fs::OpenOptions;
use std::io::Write;
use std::str::FromStr;

use rocket::FromForm;
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use sim::matchers::fifo_matcher::FIFOMatcher;
use sim::matchers::matcher::Matcher;
use sim::matchers::prorata_matcher::ProrataMatcher;

use sim::model::domain::{OrderBook, OrderBookKey, OrderSingle};
use sim::model::domain::Side::{Buy, Sell};

#[derive(Debug, Clone, Serialize, FromForm)]
pub struct Order {
    symbol: String,
    qty: u32,
    price: f64,
    side: String,
    order_type: String,
    cl_ord_id: String,
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

pub fn persist_order_book(ob: &OB) {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true) // Create the file if it doesn't exist
        .truncate(true) //overwrite content
        .open("orderbook.json").unwrap();
    let content = to_string(&ob).unwrap();
    file.write_all(content.as_bytes()).expect("Error writing");
    file.flush().expect("error flushing");
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



