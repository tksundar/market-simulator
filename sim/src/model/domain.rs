use std::collections::{HashMap, VecDeque};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};

use colored::Colorize;
use prettytable::{row, Table};
use serde::{Deserialize, Serialize};

use crate::model::domain::OrderType::{Limit, Market};
use crate::model::domain::Side::{Buy, Sell};
use crate::model::domain::Status::{Filled, New, PartialFill, PendingNew, Rejected, Replaced, UNKNOWN};
use crate::utils::{Aggregator, generate_id, Sigma};

///Order TYpe . Can be either Limit or Market
#[derive(PartialEq, Debug, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum OrderType {
    Market,
    Limit,
}

impl OrderType {
    pub fn string_value(&self) -> String {
        match self {
            Limit => "Limit".to_owned(),
            Market => "Market".to_owned()
        }
    }

    pub fn from(str: &String) -> Self {
        if str == "Limit" {
            OrderType::Limit
        } else {
            OrderType::Market
        }
    }
}

impl Default for OrderType {
    fn default() -> Self {
        Limit
    }
}

#[derive(PartialEq, Debug, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}
/// Side of the order. Can be either Buy or Sell
impl Side {
    pub fn string_value(&self) -> String {
        match self {
            Buy => "Buy".to_owned(),
            Sell => "Sell".to_owned()
        }
    }

    pub fn from(str: &String) -> Self {
        if str == "Buy" {
            Side::Buy
        } else {
            Side::Sell
        }
    }
}

impl Default for Side {
    fn default() -> Self {
        Buy
    }
}
///Order Status and Execution Status. Can be one of New,
///    PendingNew,
///    PartialFill,
///    Filled,
///     Rejected,
///     Replaced,
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Status {
    New,
    PendingNew,
    PartialFill,
    Filled,
    Rejected,
    Replaced,
    UNKNOWN,
}

impl Status {
    ///Returns the FIX specific character for the status as per FIX specification
    pub fn char_value(&self) -> char {
        match self {
            New => '0',
            PendingNew => 'A',
            PartialFill => '1',
            Filled => '2',
            Rejected => '4',
            Replaced => '5',
            UNKNOWN => 'U'
        }
    }

    fn string_value(&self) -> &str {
        match self {
            New => "New",
            PendingNew => "Pending",
            PartialFill => "PartialFill",
            Filled => "Filled",
            Rejected => "Rejected",
            Replaced => "Replaced",
            UNKNOWN => "???"
        }
    }
}

impl Eq for Status {}

impl PartialEq for Status {
    fn eq(&self, other: &Self) -> bool {
        self.char_value() == other.char_value()
    }
}

///Defines an order.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderSingle {
    qty: u32,
    symbol: String,
    price: f64,
    side: Side,
    order_type: OrderType,
    cl_ord_id: String,
}

///Defines a Fill from an Execution
#[derive(Debug, Clone,
Serialize, Deserialize)]
pub struct Fill {
    symbol: String,
    order_id: String,
    execution_id: String,
    exec_type: Status,
    qty: u32,
    leaves_qty: u32,
    cum_qty: u32,
    price: f64,
    side: Side,
    cl_ord_id: String,
    secondary_cl_ord_id: String,
    status: Status,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookKey {
    price: f64,
    symbol: String,
}

///The key on which [`OrderSingle`] instances are stored in the [`OrderBook`]
impl OrderBookKey {
    pub fn new(price: f64, symbol: String) -> Self {
        Self { price, symbol }
    }

    pub fn is_valid(&self) -> bool {
        self.price > 0.0 && self.symbol.len() > 0
    }
}

impl Eq for OrderBookKey {}

impl PartialEq for OrderBookKey {
    fn eq(&self, other: &Self) -> bool {
        self.price == other.price && self.symbol == other.symbol
    }
}


impl Hash for OrderBookKey {
    fn hash<H>(&self, state: &mut H)
        where
            H: Hasher,
    {
        let float_bits: u64 = unsafe { std::mem::transmute(self.price) };
        float_bits.hash(state);
        self.symbol.as_bytes().hash(state);
    }
}

impl OrderBookKey {
    pub fn price(&self) -> f64 {
        self.price
    }
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn set_price(&mut self, price: f64) {
        self.price = price;
    }
    pub fn set_symbol(&mut self, symbol: String) {
        self.symbol = symbol;
    }
}


impl Fill {
    pub fn new(
        symbol: String,
        order_id: String,
        execution_id: String,
        exec_type: Status,
        qty: u32,
        leaves_qty: u32,
        cum_qty: u32,
        price: f64,
        side: Side,
        cl_ord_id: String,
        status: Status) -> Self {
        Self {
            symbol,
            order_id,
            execution_id,
            exec_type,
            qty,
            leaves_qty,
            cum_qty,
            price,
            side,
            cl_ord_id,
            secondary_cl_ord_id: "".to_string(),
            status,
        }
    }

    ///Returns a string formatted a table of all the fills in the `fills` argument
    pub fn pretty_print(fills: &Vec<Fill>) -> String {
        if fills.is_empty() {
            return "No fills".to_string();
        }

        let title = format!("{}", "Fills".reversed().bold());
        println!("\n{}", title);


        let mut table = Table::new();
        table.add_row(row!["Symbol","Qty","Price","client_order_id","exchange_order_id","Side","Order Status"]);
        for fill in fills {
            table.add_row(row![fill.symbol,fill.qty,fill.price,fill.cl_ord_id,fill.secondary_cl_ord_id,fill.side.string_value(),fill.status.string_value()]);
        }

        table.printstd();
        table.to_string()
    }

    pub fn set_qty(&mut self, qty: u32) {
        self.qty = qty;
    }

    pub fn set_cum_qty(&mut self, cum_qty: u32) {
        self.cum_qty = cum_qty;
    }

    pub fn set_leaves_qty(&mut self, leaves_qty: u32) {
        self.leaves_qty = leaves_qty;
    }
    pub fn set_symbol(&mut self, symbol: String) {
        self.symbol = symbol;
    }

    pub fn qty(&self) -> u32 {
        self.qty
    }

    pub fn cum_qty(&self) -> u32 {
        self.cum_qty
    }

    pub fn leaves_qty(&self) -> u32 {
        self.leaves_qty
    }
    pub fn cl_ord_id(&self) -> &str {
        &self.cl_ord_id
    }

    pub fn secondary_cl_ord_id(&self) -> String {
        self.secondary_cl_ord_id.clone()
    }

    pub fn set_secondary_cl_ord_id(&mut self, matching_side_id: String) {
        self.secondary_cl_ord_id = matching_side_id;
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    pub fn set_order_id(&mut self, order_id: String) {
        self.order_id = order_id;
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn side(&self) -> Side {
        self.side.clone()
    }
    pub fn set_side(&mut self, side: Side) {
        self.side = side
    }

    pub fn from(order: &OrderSingle) -> Self {
        Self {
            qty: order.qty(),
            cum_qty: 0,
            leaves_qty: order.qty(),
            order_id: generate_id(),
            execution_id: generate_id(),
            price: order.price(),
            symbol: order.symbol().clone(),
            status: Status::Filled,
            side: order.side(),
            cl_ord_id: order.cl_ord_id().clone(),
            secondary_cl_ord_id: "".to_string(),
            exec_type: Status::New,

        }
    }
}


impl OrderSingle {
    pub fn new(qty: u32,
               symbol: String,
               price: f64,
               side: Side,
               order_type: OrderType,
               cl_ord_id: String) -> Self {
        Self {
            qty,
            symbol,
            price,
            side,
            order_type,
            cl_ord_id,
        }
    }

/// Returns `true` if the order is valid. `false` otherwise. As you can see Market orders are
/// not supported as af now
///
/// # Example
/// self.symbol.trim().len() > 0 &&
///             self.price() > 0.0 &&
///             self.qty() > 0 &&
///             (self.side.string_value() == "Buy" || self.side.string_value() == "Sell") &&
///            self.order_type.string_value() == "Limit
    pub fn is_valid(&self) -> bool {
        self.symbol.trim().len() > 0 &&
            self.price() > 0.0 &&
            self.qty() > 0 &&
            (self.side.string_value() == "Buy" || self.side.string_value() == "Sell") &&
            self.order_type.string_value() == "Limit" &&
            self.cl_ord_id.trim().len() > 0
    }

    pub fn get_order_book_key(&self) -> OrderBookKey {
        OrderBookKey::new(self.price, self.symbol.clone())
    }

    pub fn set_qty(&mut self, qty: u32) {
        self.qty = qty;
    }
    pub fn set_price(&mut self, price: f64) {
        self.price = price;
    }

    pub fn set_side(&mut self, side: Side) {
        self.side = side;
    }

    pub fn matching_side(&self) -> Side {
        match self.side {
            Buy => Sell,
            Sell => Buy,
        }
    }

    pub fn qty(&self) -> u32 {
        self.qty.clone()
    }


    pub fn symbol(&self) -> &String {
        &self.symbol
    }

    pub fn side(&self) -> Side {
        self.side.clone()
    }

    pub fn order_type(&self) -> OrderType {
        self.order_type.clone()
    }

    pub fn cl_ord_id(&self) -> &String {
        &self.cl_ord_id
    }

    pub fn price(&self) -> f64 {
        self.price
    }
}

impl Display for OrderSingle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "OrderSingle:: symbol: {}, quantity: {} , price: {}, side: {}, cl_ord_id: {}",
               self.symbol, self.qty, self.price, self.side.string_value(), self.cl_ord_id)
    }
}

impl Eq for OrderSingle {}

impl PartialEq for OrderSingle {
    fn eq(&self, other: &Self) -> bool {
        self.cl_ord_id() == other.cl_ord_id()
    }
}


impl Hash for OrderSingle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cl_ord_id.as_bytes().hash(state);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrderBook {
    buy_orders: HashMap<OrderBookKey, VecDeque<OrderSingle>>,
    sell_orders: HashMap<OrderBookKey, VecDeque<OrderSingle>>,
}

impl Default for OrderBook {
    fn default() -> Self {
        Self {
            buy_orders: HashMap::new(),
            sell_orders: HashMap::new(),
        }
    }
}

impl OrderBook {
    pub fn new(buy_orders: HashMap<OrderBookKey, VecDeque<OrderSingle>>,
               sell_orders: HashMap<OrderBookKey, VecDeque<OrderSingle>>) -> Self {
        Self {
            buy_orders,
            sell_orders,

        }
    }


    pub fn update_self(&mut self, order_book: &mut Self) {
        self.buy_orders = order_book.buy_orders.clone();
        self.sell_orders = order_book.sell_orders.clone();
    }

    pub fn update_order_book(&mut self, orders: HashMap<OrderBookKey, VecDeque<OrderSingle>>, side: Side) {
        match side {
            Buy => self.buy_orders = orders,
            Sell => self.sell_orders = orders,
        }
    }

    pub fn get_order_book(&self) -> Self {
        self.clone()
    }


    pub fn get_orders_for_matching(&self, side: Side) -> (HashMap<OrderBookKey, VecDeque<OrderSingle>>,
                                                          HashMap<OrderBookKey, VecDeque<OrderSingle>>) {
        match side {
            Buy => (self.buy_orders.clone(), self.sell_orders.clone()),
            Sell => (self.sell_orders.clone(), self.buy_orders.clone())
        }
    }

    pub fn get_orders_for(&self, side: Side) -> HashMap<OrderBookKey, VecDeque<OrderSingle>> {
        match side {
            Buy => self.buy_orders.clone(),
            Sell => self.sell_orders.clone(),
        }
    }

    pub fn print_order_book(&self) {
        println!("Buy Orders >> {:#?}", self.buy_orders);
        println!("Sell Orders >> {:#?}", self.sell_orders);
    }

    pub fn is_empty(&self) -> bool {
        self.buy_orders.is_empty() && self.sell_orders.is_empty()
    }


    pub fn add_order_to_order_book(&mut self, order: OrderSingle) {
        let side = order.side();
        let key = OrderBookKey::new(order.price(), order.symbol().to_owned());
        let order_map = self.order_map(side);
        if order_map.contains_key(&key) {
            order_map.entry(key).and_modify(|dequue| dequue.push_back(order));
        } else {
            let mut deque = VecDeque::new();
            deque.push_back(order);
            order_map.insert(key, deque);
        }
    }

    pub fn order_map(&mut self, side: Side) -> &mut HashMap<OrderBookKey, VecDeque<OrderSingle>> {
        if side == Sell {
            &mut self.sell_orders
        } else {
            &mut self.buy_orders
        }
    }

    pub fn pretty_print_self(&self) -> String {
        let keys = self.get_excl_keys();
        let mut strings = String::new();
        for key in &keys {
            let s = self.print_market_depth_for(key);
            strings.push_str(s.as_str());
        }
        strings
    }


    fn get_excl_keys(&self) -> Vec<&str> {
        let mut all_keys = vec![];
        for (key, _) in &self.buy_orders {
            all_keys.push(key.symbol());
        }
        for (key, _) in &self.sell_orders {
            all_keys.push(key.symbol());
        }
        let mut excl_keys: Vec<&str> = vec![];
        for symbol in all_keys {
            if !excl_keys.contains(&symbol) {
                excl_keys.push(&symbol)
            }
        }
        excl_keys
    }
    pub fn print_market_depth_for(&self, symbol: &str) -> String {
        let mut md_buy = self.get_md(symbol, &self.buy_orders).clone();
        let mut md_sell = self.get_md(symbol, &self.sell_orders).clone();
        let s = format!("market depth for {}", symbol);
        let mut strings = String::new();
        println!("\n{}", s.reversed());
        println!();
        println!("{}", "Bids:".green().bold());
        strings.push_str("Bids:\n");
        strings.push_str(self.print_md(&mut md_buy).as_str());
        println!();
        println!("{}", "Offers:\n".red().bold());
        strings.push_str("Offers:\n");
        strings.push_str(self.print_md(&mut md_sell).as_str());
        strings

    }

    fn print_md(&self, mds_buy: &Vec<MarketDepth>) -> String {
        let mut table = Table::new();
        table.add_row(row!["Quantity","Price"]);
        for md in mds_buy {
            table.add_row(row![md.qty(), md.price()]);
        }

        table.printstd();
        table.to_string()
    }


    fn get_md(&self, symbol: &str, order_map: &HashMap<OrderBookKey, VecDeque<OrderSingle>>) -> Vec<MarketDepth> {
        let mut buys = vec![];

        for (key, orders) in order_map {
            let side = &orders[0].side();
            if key.symbol() == symbol {
                let orders: Vec<OrderSingle> = orders.clone().into_iter().collect();
                let aggregate = Aggregator::sigma(&orders);
                let md = MarketDepth::new(key.price, aggregate, side.clone());
                buys.push(md);
            }
        }
        buys
    }
}

#[derive(Clone, Debug)]
struct MarketDepth {
    price: f64,
    qty: u32,
    side: Side,
}

impl MarketDepth {
    fn new(price: f64, qty: u32, side: Side) -> Self {
        Self { price, qty, side }
    }


    pub fn price(&self) -> f64 {
        self.price
    }
    pub fn qty(&self) -> u32 {
        self.qty
    }
}

mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use log::debug;

    use crate::common::utils::{create_order_book, read_input};
    use crate::model::domain::{OrderBook, OrderBookKey};

    #[test]
    fn test_partial_equals() {
        let ok1 = OrderBookKey::new(101.5, "infy".to_string());
        let ok2 = OrderBookKey::new(101.1, "infy".to_string());
        let ok3 = OrderBookKey::new(101.5, "infy".to_string());

        assert_ne!(ok1, ok2);
        assert_eq!(ok1, ok3);
    }

    #[test]
    fn test_orderbookkey_hash() {
        let ok1 = OrderBookKey::new(101.5, "infy".to_string());
        let ok2 = OrderBookKey::new(101.5, "infy".to_string());
        let ok3 = OrderBookKey::new(101.45, "infy".to_string());

        let mut hasher = DefaultHasher::new();

        ok1.hash(&mut hasher);
        let hash1 = hasher.finish();

        hasher = DefaultHasher::new();

        ok2.hash(&mut hasher);
        let hash2 = hasher.finish();

        hasher = DefaultHasher::new();

        ok3.hash(&mut hasher);
        let hash3 = hasher.finish();


        debug!("{hash1} {hash2} {hash3}");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_ne!(hash2, hash3);
    }

    #[test]
    fn test_print_md() {
        let mut ob = OrderBook::default();
        create_order_book(&mut ob, read_input("test_data/orders.txt"));
        let key = &String::from("IBM");
        let s = ob.pretty_print_self();

        println!("{}", s);
    }
}


   




