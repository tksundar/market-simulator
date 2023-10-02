use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{Receiver, Sender};
use std::vec;

use log::{error, info, trace};

use crate::matchers::matcher::Matcher;
use crate::model::domain::{Fill, OrderBook, OrderBookKey, OrderSingle, Side};
use crate::model::domain::Side::{Buy, Sell};
use crate::model::domain::Status::{Filled, PartialFill};
use crate::utils::{Aggregator, Sigma};

#[derive(Debug)]
pub struct FIFOMatcher {
    name: String,
    ex_cum_qty: u32,
}

impl FIFOMatcher {
    pub fn new() -> Self {
        Self {
            name: "FIFO".to_string(),
            ex_cum_qty: 0,
        }
    }

    fn set_ex_cum_qty(&mut self, qty: u32) {
        self.ex_cum_qty = qty;
    }
    fn ex_cum_qty(&self) -> u32 {
        self.ex_cum_qty
    }

    pub fn sum_of_filled_quantities(&self, fills: &Vec<Fill>, side: Side) -> u32 {
        let fills_for_sum: Vec<Fill> = fills.clone().into_iter().filter(|f| f.side() == side).collect();
        Aggregator::sigma(&fills_for_sum)
    }

    ///create client side and exchange side fills from client order and exchange order
    fn update_fills(&mut self, client_order: &OrderSingle, exchange_order: &mut OrderSingle,
                    client_fill: &mut Fill, ex_fill: &mut Fill) {

        //set the secondary ids
        client_fill.set_secondary_cl_ord_id(exchange_order.cl_ord_id().clone());
        ex_fill.set_secondary_cl_ord_id(client_order.cl_ord_id().clone());
        let order_qty = client_order.qty();
        let avail_qty = exchange_order.qty();
        let leaves_qty = client_fill.leaves_qty();
        let cl_cum_qty = client_fill.cum_qty();

        if leaves_qty <= avail_qty {
            self.exchange_partial_fill(cl_cum_qty, avail_qty, leaves_qty, client_fill, ex_fill, exchange_order)
        } else {
            self.client_order_partial_fill(order_qty, avail_qty, client_fill, ex_fill, exchange_order);
        }
    }

    fn client_order_partial_fill(&self, order_qty: u32, avail_qty: u32, client_fill: &mut Fill, ex_fill: &mut Fill, exchange_order: &mut OrderSingle) {
        let cl_cum_qty = client_fill.cum_qty() + exchange_order.qty();
        let leaves_qty = order_qty - cl_cum_qty;
        // trace!(" order qty {} cl_cum_qty {} leaves qty {}",order_qty, cl_cum_qty,leaves_qty);
        client_fill.set_qty(avail_qty); //  100
        client_fill.set_cum_qty(cl_cum_qty); //100
        client_fill.set_leaves_qty(leaves_qty); // 50
        client_fill.set_status(PartialFill);

        ex_fill.set_qty(avail_qty);
        ex_fill.set_cum_qty(avail_qty);
        ex_fill.set_leaves_qty(0);
        ex_fill.set_status(Filled);
        exchange_order.set_qty(0);
    }

    fn exchange_partial_fill(&mut self, mut cl_cum_qty: u32, avail_qty: u32, leaves_qty: u32,
                             client_fill: &mut Fill, ex_fill: &mut Fill, exchange_order: &mut OrderSingle) {
        let mut ex_cum_qty = self.ex_cum_qty();

        cl_cum_qty = cl_cum_qty + leaves_qty;
        client_fill.set_qty(leaves_qty);
        client_fill.set_cum_qty(cl_cum_qty);
        client_fill.set_leaves_qty(0);
        client_fill.set_status(Filled);

        ex_fill.set_qty(leaves_qty);
        ex_fill.set_cum_qty(ex_cum_qty + leaves_qty);
        ex_fill.set_leaves_qty(avail_qty - ex_fill.qty());
        ex_cum_qty += leaves_qty;
        if ex_fill.leaves_qty() == 0 {
            ex_fill.set_status(Filled);
        } else {
            ex_fill.set_status(PartialFill);
        }
        exchange_order.set_qty(avail_qty - leaves_qty);
        self.set_ex_cum_qty(ex_cum_qty);
    }
    fn print_fills(&self, fills: Vec<Fill>, side: Side) {
        let exchange_side = if side == Buy { Sell } else { Buy };
        let client_fills: Vec<Fill> = fills.clone().into_iter().filter(|f| f.side() == side).collect();
        let exchange_fills: Vec<Fill> = fills.into_iter().filter(|f| f.side() == exchange_side).collect();
        info!("Client Fills {:#?}",client_fills);
        info!("Exchange Fills {:#?}",exchange_fills);
    }

    fn get_fills_for(&mut self, matching_map: &mut HashMap<OrderBookKey, VecDeque<OrderSingle>>,
                     order: &OrderSingle) -> Vec<Fill> {
        let mut fills = vec![];
        let key = order.get_order_book_key();

        trace!("order book key {:?}",key);
        if matching_map.contains_key(&key) {
            let deque = matching_map.get(&key).unwrap().clone();
            let mut client_fill = Fill::from(order);

            // trace!("client fill {:#?}",client_fill);
            for avail in deque.iter() {
                let mut exchange_order = avail.clone();
                let mut ex_fill = Fill::from(avail);
                self.update_fills(order, &mut exchange_order, &mut client_fill, &mut ex_fill);
                fills.push(client_fill.clone());
                fills.push(ex_fill.clone());
                if exchange_order.qty() == 0 {
                    matching_map.entry(avail.get_order_book_key()).and_modify(|d| self.remove_order(d));
                } else {
                    matching_map.entry(avail.get_order_book_key()).and_modify(|d| self.remove_order(d));
                    matching_map.entry(avail.get_order_book_key()).and_modify(|d| d.push_front(exchange_order));
                }
                if client_fill.status().clone() == Filled {
                    break;
                }
            }

            let v = matching_map.get(&key);
            if v.is_some() {
                if v.unwrap().is_empty() {
                    matching_map.remove(&key);
                }
            }
        }
        fills
    }


    fn remove_order(&self, deq: &mut VecDeque<OrderSingle>) {
        let o = deq.pop_front();
        if o.is_some() {
            trace!("order  removed {}",o.unwrap().cl_ord_id());
        }
    }
}


impl Matcher for FIFOMatcher {
    fn start(&mut self, tx: &Sender<OrderBook>, rx: &Receiver<OrderBook>) {
        info!("Starting FIFO Matcher...");
        loop {
            let mut ob = rx.recv().unwrap();
            trace!("[FIFOMatcher]received order book to match {:?}",&ob);
            if ob.is_empty() {
                info!("Order Book is empty");
            }
            let fills: Vec<Fill> = self.match_order_book(&mut ob);
            Fill::pretty_print(&fills);
            if let Err(e) = tx.send(ob) {
                error!("error sending order book {}",e);
            }
        }
    }


    /// This function uses the match_order method to do most of the work.
    /// The logic iterates over the buy side orders and tries to match them
    /// with any order available on the sell side
    fn match_order_book(&mut self, order_book: &mut OrderBook) -> Vec<Fill> {
        let (buy, mut sell) = order_book.get_orders_for_matching(Side::Buy);

        let mut fills = vec![];

        let mut temp = HashMap::new();

        for (key, deque) in buy.iter() {
            let mut deque_clone = deque.clone();
            for order in deque.iter() {
                trace!("Matching order with cl_ord_id {}", order.cl_ord_id());
                let sub_fills: Vec<Fill> = self.get_fills_for(&mut sell, order);//self.match_order(order, &mut sell,order_book);
                if sub_fills.is_empty() {
                    continue;
                }
                let buy_fills: Vec<Fill> = sub_fills.clone().into_iter().filter(|f| f.side() == Buy).collect();
                let total = Aggregator::sigma(&buy_fills);

                if order.qty() == total {
                    trace!("removing order with id {}",order.cl_ord_id());
                    deque_clone.pop_front();
                } else {
                    let updated_qty = order.qty() - total;
                    trace!("Updating new quantity {}",updated_qty);
                    let mut order_clone = order.clone();
                    order_clone.set_qty(updated_qty);
                    deque_clone.pop_front();
                    deque_clone.push_front(order_clone);
                }
                for fill in sub_fills {
                    fills.push(fill);
                }
            }

            if !deque_clone.is_empty() {
                temp.insert(key.clone(), deque_clone);
            } else {}
        }

        let mut ob = OrderBook::new(temp, sell);
        order_book.update_self(&mut ob);


        fills
    }
}


#[cfg(test)]
mod tests {
    use crate::matchers::fifo_matcher::FIFOMatcher;
    use crate::matchers::matcher::Matcher;
    use crate::model::domain::{Fill, OrderBook};
    use crate::model::domain::Side::Buy;
    use crate::model::domain::Status::{Filled, PartialFill};
    use crate::utils::{create_order_book, create_order_from_string, read_input};

    #[test]
    fn test_update_fills_order_qty_eq_available_qty() {
        let input = read_input("fifo_test_data/orders.txt");
        let mut order_book = OrderBook::default();
        create_order_book(&mut order_book, input);
        let cl_order = create_order_from_string("test1 IBM 100 601.1 Sell".to_string());
        let key = cl_order.get_order_book_key();
        let (buy, _) = order_book.get_orders_for_matching(Buy);
        let orders = buy.get(&key).unwrap();
        let mut ex_order = orders.clone().pop_front().unwrap();
        let mut fifo = FIFOMatcher::new();
        let mut client_fill = Fill::from(&cl_order);
        let mut ex_fill = Fill::from(&ex_order);

        fifo.update_fills(&cl_order, &mut ex_order, &mut client_fill, &mut ex_fill);
        assert_eq!(client_fill.qty(), 100);
        assert_eq!(client_fill.cum_qty(), 100);
        assert_eq!(client_fill.leaves_qty(), 0);
        assert_eq!(client_fill.status().clone(), Filled);
        assert_eq!(ex_fill.qty(), 100);
        assert_eq!(ex_fill.cum_qty(), 100);
        assert_eq!(ex_fill.leaves_qty(), 0);
        assert_eq!(ex_fill.status().clone(), Filled);
        assert_eq!(client_fill.secondary_cl_ord_id(), "id4");
        assert_eq!(ex_fill.secondary_cl_ord_id(), "test1");
        assert_eq!(ex_order.qty(), 0);
    }

    #[test]
    fn test_order_match_after_order_book_match() {
        let input = read_input("fifo_test_data/orders.txt");
        let mut order_book = OrderBook::default();
        create_order_book(&mut order_book, input.clone());
        let mut fifo = FIFOMatcher::new();
        fifo.match_order_book(&mut order_book);
        let cl_order = create_order_from_string("test1 IBM 100 601.1 Sell".to_string());
        let key = cl_order.get_order_book_key();
        let (buy, _) = order_book.get_orders_for_matching(Buy);
        //trace!("{:#?}",buy);
        let orders = buy.get(&key).unwrap();
        let mut ex_order = orders.clone().pop_front().unwrap();
        let mut fifo = FIFOMatcher::new();
        let mut client_fill = Fill::from(&cl_order);
        let mut ex_fill = Fill::from(&ex_order);

        fifo.update_fills(&cl_order, &mut ex_order, &mut client_fill, &mut ex_fill);
    }


    #[test]
    fn test_update_fills_order_qty_less_than_available_qty() {
        let input = read_input("fifo_test_data/orders.txt");
        let mut order_book = OrderBook::default();
        create_order_book(&mut order_book, input);
        let cl_order = create_order_from_string("test1 IBM 50 601.1 Sell".to_string());
        let key = cl_order.get_order_book_key();
        let (buy, _) = order_book.get_orders_for_matching(Buy);
        //trace!("{:#?}",buy);
        let orders = buy.get(&key).unwrap();
        let mut ex_order = orders.clone().pop_front().unwrap();
        let mut fifo = FIFOMatcher::new();
        let mut client_fill = Fill::from(&cl_order);
        let mut ex_fill = Fill::from(&ex_order);

        fifo.update_fills(&cl_order, &mut ex_order, &mut client_fill, &mut ex_fill);
        assert_eq!(client_fill.qty(), 50);
        assert_eq!(client_fill.cum_qty(), 50);
        assert_eq!(client_fill.leaves_qty(), 0);
        assert_eq!(client_fill.status().clone(), Filled);
        assert_eq!(ex_fill.qty(), 50);
        assert_eq!(ex_fill.cum_qty(), 50);
        assert_eq!(ex_fill.leaves_qty(), 50);
        assert_eq!(ex_fill.status().clone(), PartialFill);
        assert_eq!(client_fill.secondary_cl_ord_id(), "id4");
        assert_eq!(ex_fill.secondary_cl_ord_id(), "test1");
        assert_eq!(ex_order.qty(), 50);
    }

    #[test]
    fn test_update_fills_order_qty_greater_than_available_qty() {
        let input = read_input("fifo_test_data/orders.txt");
        let mut order_book = OrderBook::default();
        create_order_book(&mut order_book, input);
        let cl_order = create_order_from_string("test1 IBM 150 601.1 Sell".to_string());
        let key = cl_order.get_order_book_key();
        let (buy, _) = order_book.get_orders_for_matching(Buy);
        let orders = buy.get(&key).unwrap();
        let mut ex_order = orders.clone().pop_front().unwrap();
        let mut fifo = FIFOMatcher::new();
        let mut client_fill = Fill::from(&cl_order);
        let mut ex_fill = Fill::from(&ex_order);

        fifo.update_fills(&cl_order, &mut ex_order, &mut client_fill, &mut ex_fill);

        assert_eq!(client_fill.qty(), 100);
        assert_eq!(client_fill.cum_qty(), 100);
        assert_eq!(client_fill.leaves_qty(), 50);
        assert_eq!(client_fill.status().clone(), PartialFill);
        assert_eq!(ex_fill.qty(), 100);
        assert_eq!(ex_fill.cum_qty(), 100);
        assert_eq!(ex_fill.leaves_qty(), 0);
        assert_eq!(ex_fill.status().clone(), Filled);
        assert_eq!(client_fill.secondary_cl_ord_id(), "id4");
        assert_eq!(ex_fill.secondary_cl_ord_id(), "test1");
        assert_eq!(ex_order.qty(), 0);
    }
}



