use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{Receiver, Sender};

use crate::model::domain::{Fill, OrderBook, OrderSingle};

/// The Matcher trait defines the functionalities provided by the matchers that implement this trait
pub trait Matcher {
    ///Starts the matcher. The matcher receives the order book through the Receiver , does any matching
    /// ,updates the order book and sends it back using the Sender
    fn start(&mut self, tx: &Sender<OrderBook>, rx: &Receiver<OrderBook>);


    /// Matches the given order book for any matches and returns a list of Fills
    fn match_order_book(&mut self, order_book: &mut OrderBook) -> Vec<Fill>;


    /// When traversing multiple orders to generate a fill, this map helps keep track of the quantity
    /// filled until now
    fn create_cum_qty_map(&self, orders: &VecDeque<OrderSingle>) -> HashMap<String, u32> {
        let mut map = HashMap::new();

        for order in orders {
            map.insert(order.cl_ord_id().clone(), 0);
        }
        map
    }
}


