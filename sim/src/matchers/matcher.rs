use std::sync::mpsc::{Receiver, Sender};

use crate::model::domain::{Fill, OrderBook};

/// The Matcher trait defines the functionalities provided by the matchers that implement this trait
pub trait Matcher {
    ///Starts the matcher. The matcher receives the order book through the Receiver , does any matching
    /// ,updates the order book and sends it back using the Sender
    fn start(&mut self, tx: &Sender<OrderBook>, rx: &Receiver<OrderBook>);

    /*  ///Matches a single order against the given orders queue and returns a list of fills
      fn match_order(&mut self, order: &OrderSingle, orders: &mut HashMap<OrderBookKey, VecDeque<OrderSingle>>, order_book: &mut OrderBook) -> Vec<Fill>;
  */
    /// Matches the given order book for any matches and returns a list of Fills
    fn match_order_book(&mut self, order_book: &mut OrderBook) -> Vec<Fill>;
}


