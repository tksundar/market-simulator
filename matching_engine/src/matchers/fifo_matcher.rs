use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{Receiver, Sender};
use std::vec;
use colored::Colorize;

use log::{error, info, trace};

use crate::matchers::matcher::Matcher;
use crate::model::domain::{Fill, OrderBook, OrderBookKey, OrderSingle, Side};
use crate::model::domain::Side::{Buy, Sell};
use crate::model::domain::Status::{Filled, PartialFill};
use crate::utils::{Aggregator, Sigma};

#[derive(Debug)]
pub struct FIFOMatcher;

impl FIFOMatcher {


     fn sum_of_filled_quantities(&self, fills: &Vec<Fill>, side: Side) -> u32 {
        let fills_for_sum: Vec<Fill> = fills.clone().into_iter().filter(|f| f.side() == side).collect();
        Aggregator::sigma(&fills_for_sum)
    }

    ///create client side and exchange side fills from client order and exchange order
    fn update_fills(&mut self, client_order: &OrderSingle, exchange_order: &mut OrderSingle,
                    client_fill: &mut Fill, ex_fill: &mut Fill, cl_cum_map:&mut HashMap<String,u32>, ex_cum_map:&mut HashMap<String,u32>) {

        //set the secondary ids
        client_fill.set_secondary_cl_ord_id(exchange_order.cl_ord_id().clone());
        ex_fill.set_secondary_cl_ord_id(client_order.cl_ord_id().clone());
        let order_qty = client_order.qty();
        let avail_qty = exchange_order.qty();
        let leaves_qty = client_fill.leaves_qty();
        let cl_cum_qty = cl_cum_map.get(client_order.cl_ord_id()).unwrap().clone();
        if leaves_qty <= avail_qty {
            self.exchange_partial_fill(cl_cum_qty, avail_qty, leaves_qty, client_fill, ex_fill, exchange_order,ex_cum_map);
        } else {
            self.client_order_partial_fill(order_qty, avail_qty, client_fill, ex_fill, exchange_order,cl_cum_map);
        }
    }

    fn client_order_partial_fill(&self, order_qty: u32, avail_qty: u32, client_fill: &mut Fill, ex_fill: &mut Fill, exchange_order: &mut OrderSingle,cl_cum_map:&mut HashMap<String,u32>) {
        let mut cl_cum_qty = cl_cum_map.get(client_fill.cl_ord_id()).unwrap().to_owned();
        cl_cum_qty += exchange_order.qty();
        let leaves_qty = order_qty - cl_cum_qty;
        // trace!(" order qty {} cl_cum_qty {} leaves qty {}",order_qty, cl_cum_qty,leaves_qty);
        client_fill.set_qty(avail_qty); //  100
        client_fill.set_cum_qty(cl_cum_qty); //100
        client_fill.set_leaves_qty(leaves_qty); // 50
        client_fill.set_status(PartialFill);
        cl_cum_map.insert(client_fill.cl_ord_id().to_owned(),cl_cum_qty);

        ex_fill.set_qty(avail_qty);
        ex_fill.set_cum_qty(avail_qty);
        ex_fill.set_leaves_qty(0);
        ex_fill.set_status(Filled);
        exchange_order.set_qty(0);
    }

    fn exchange_partial_fill(&mut self, mut cl_cum_qty: u32, avail_qty: u32, leaves_qty: u32,
                             client_fill: &mut Fill, ex_fill: &mut Fill, exchange_order: &mut OrderSingle, ex_cum_map:&mut HashMap<String,u32>) {

        let mut ex_cum_qty = ex_cum_map.get(ex_fill.cl_ord_id()).unwrap().to_owned();

        ex_cum_qty += leaves_qty;

        cl_cum_qty = cl_cum_qty + leaves_qty;
        client_fill.set_qty(leaves_qty);
        client_fill.set_cum_qty(cl_cum_qty);
        client_fill.set_leaves_qty(0);
        client_fill.set_status(Filled);

        ex_fill.set_qty(leaves_qty);
        ex_fill.set_cum_qty(ex_cum_qty);
        ex_fill.set_leaves_qty(avail_qty - ex_fill.qty());
        ex_cum_qty += leaves_qty;
        if ex_fill.leaves_qty() == 0 {
            ex_fill.set_status(Filled);
        } else {
            ex_fill.set_status(PartialFill);
        }
        exchange_order.set_qty(avail_qty - leaves_qty);
       ex_cum_map.insert(ex_fill.cl_ord_id().to_owned(),ex_cum_qty);
    }
    fn print_fills(&self, fills: Vec<Fill>, side: Side) {
        let exchange_side = if side == Buy { Sell } else { Buy };
        let client_fills: Vec<Fill> = fills.clone().into_iter().filter(|f| f.side() == side).collect();
        let exchange_fills: Vec<Fill> = fills.into_iter().filter(|f| f.side() == exchange_side).collect();
        info!("Client Fills {:#?}",client_fills);
        info!("Exchange Fills {:#?}",exchange_fills);
    }

    fn get_fills_for(&mut self, matching_map: &mut HashMap<OrderBookKey, VecDeque<OrderSingle>>, cl_cum_map:&mut HashMap<String, u32>,
                     order: &OrderSingle) -> Vec<Fill> {
        let mut fills = vec![];
        let key = order.get_order_book_key();

        trace!("order book key {:?}",key);
        if matching_map.contains_key(&key) {

            let deque = matching_map.get(&key).unwrap().clone();
            let mut e_map = self.create_cum_qty_map(&deque);
            let mut client_fill = Fill::from(order);

            // trace!("client fill {:#?}",client_fill);
            for avail in deque.iter() {
                let mut exchange_order = avail.clone();
                let mut ex_fill = Fill::from(avail);
                self.update_fills(order, &mut exchange_order, &mut client_fill, &mut ex_fill,cl_cum_map, &mut e_map);
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



    /// The logic iterates over the [`OrderBook`] buy side orders and tries to match them
    /// with any order available on the sell side. It matches to the fullest
    /// extent possible before attempting a match for the next order in the queue
    /// # Example:
    /// ```rust
    /// use matching_engine::common::utils::{create_order_book, read_input};
    /// use matching_engine::matchers::fifo_matcher::FIFOMatcher;
    /// use matching_engine::matchers::matcher::Matcher;
    /// let input = read_input("test_data/orders.txt");
    /// let mut order_book = create_order_book(input);
    /// //create a matcher
    /// let mut  matcher = FIFOMatcher;
    /// // match the order book with the matcher to produce executions
    /// let mut fills = matcher.match_order_book(&mut order_book);
    /// ```
     fn match_order_book(&mut self, order_book: &mut OrderBook) -> Vec<Fill> {
        let buy = order_book.get_orders_for(Buy);
        let mut sell = order_book.get_orders_for(Sell);
        let mut fills = vec![];

        let mut temp = HashMap::new();

        for (key, deque) in buy.iter() {
            let mut deque_clone = deque.clone();
            let mut c_map = self.create_cum_qty_map(&deque);
            for order in deque.iter() {
                trace!("Matching order with cl_ord_id {}", order.cl_ord_id());
                let sub_fills: Vec<Fill> = self.get_fills_for(&mut sell, &mut c_map,order);
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
    use crate::model::domain::Side::{Buy, Sell};
    use crate::model::domain::Status::{Filled, PartialFill};
    use crate::utils::{create_order_book, create_order_from_string, read_input};

    #[test]
    fn test_update_fills_order_qty_eq_available_qty() {
        let mut fifo = FIFOMatcher;
        let input = read_input("test_data/orders.txt");
        let mut order_book = create_order_book( input);
        let cl_order = create_order_from_string("test1 IBM 100 601.1 Sell".to_string());
        let key = cl_order.get_order_book_key();
        order_book.add_order_to_order_book(cl_order.clone());
        //let (sell, buy) = order_book.get_orders_for_matching(Buy);
        let buy = order_book.get_orders_for(Buy);
        let sell = order_book.get_orders_for(Sell);
        let sell_orders = sell.get(&key).unwrap();
        let buy_orders = buy.get(&key).unwrap();
        let mut ex_cum_map = fifo.create_cum_qty_map(&buy_orders);
        let mut cl_cum_map = fifo.create_cum_qty_map(&sell_orders);
        let mut ex_order = buy_orders.clone().pop_front().unwrap();

        let mut client_fill = Fill::from(&cl_order);
        let mut ex_fill = Fill::from(&ex_order);

        fifo.update_fills(&cl_order, &mut ex_order, &mut client_fill, &mut ex_fill,&mut cl_cum_map,&mut ex_cum_map);


        assert_eq!(client_fill.qty(), 100);
        assert_eq!(client_fill.cum_qty(), 100);
        assert_eq!(client_fill.leaves_qty(), 0);
        assert_eq!(client_fill.status().clone(), Filled);
        assert_eq!(ex_fill.qty(), 100);
        assert_eq!(ex_fill.cum_qty(), 100);
        assert_eq!(ex_fill.leaves_qty(), 0);
        assert_eq!(ex_fill.status().clone(), Filled);
        assert_eq!(client_fill.secondary_cl_ord_id(), "id8");
        assert_eq!(ex_fill.secondary_cl_ord_id(), "test1");
        assert_eq!(ex_order.qty(), 0);
    }


    #[test]
    fn test_order_match_after_order_book_match() {

        let input = read_input("test_data/orders.txt");
        let mut order_book = create_order_book( input.clone());
        let mut fifo = FIFOMatcher;
        fifo.match_order_book(&mut order_book);
        let cl_order = create_order_from_string("test1 IBM 100 601.1 Sell".to_string());
        order_book.add_order_to_order_book(cl_order.clone());
        let key = cl_order.clone().get_order_book_key();
        let mut fifo = FIFOMatcher;
       // let (sell, buy) = order_book.get_orders_for_matching(Buy);
        let client_orders_map = order_book.get_orders_for(Sell);
        let ex_orders_map = order_book.get_orders_for(Buy);
        let client_orders = client_orders_map.get(&key).unwrap();
        let ex_orders = ex_orders_map.get(&key).unwrap();

        let mut ex_cum_map = fifo.create_cum_qty_map(ex_orders);
        let mut cl_cum_map = fifo.create_cum_qty_map(client_orders);
        //trace!("{:#?}",buy);
    //    let orders = sell.get(&key).unwrap();
        let mut ex_order = ex_orders.clone().pop_front().unwrap();

        let mut client_fill = Fill::from(&cl_order);
        let mut ex_fill = Fill::from(&ex_order);

        fifo.update_fills(&cl_order, &mut ex_order, &mut client_fill, &mut ex_fill, &mut cl_cum_map,&mut ex_cum_map);
        assert_eq!(client_fill.qty(), 100);
        assert_eq!(client_fill.cum_qty(), 100);
        assert_eq!(client_fill.leaves_qty(), 0);
        assert_eq!(client_fill.status().clone(), Filled);
        assert_eq!(ex_fill.qty(), 100);
        assert_eq!(ex_fill.cum_qty(), 100);
        assert_eq!(ex_fill.leaves_qty(), 0);
        assert_eq!(ex_fill.status().clone(), Filled);
        assert_eq!(client_fill.secondary_cl_ord_id(), "id8");
        assert_eq!(ex_fill.secondary_cl_ord_id(), "test1");
        assert_eq!(ex_order.qty(), 0);


    }
    #[test]
    fn test_update_fills_order_qty_less_than_available_qty() {
        let input = read_input("test_data/orders.txt");
        let mut order_book = create_order_book(input);
        let cl_order = create_order_from_string("test1 IBM 50 601.1 Sell".to_string());
        order_book.add_order_to_order_book(cl_order.clone());
        let key = cl_order.get_order_book_key();
        let mut fifo = FIFOMatcher;
        let client_order_map = order_book.get_orders_for(Sell);
        let ex_order_map = order_book.get_orders_for(Buy);
        let cl_orders = client_order_map.get(&key).unwrap();
        let ex_orders = ex_order_map.get(&key).unwrap();
        let mut ex_cum_map = fifo.create_cum_qty_map(ex_orders);
        let mut cl_cum_map = fifo.create_cum_qty_map(cl_orders);
        let mut ex_order = ex_orders.clone().pop_front().unwrap();
        let mut fifo = FIFOMatcher;
        let mut client_fill = Fill::from(&cl_order);
        let mut ex_fill = Fill::from(&ex_order);

        fifo.update_fills(&cl_order, &mut ex_order, &mut client_fill, &mut ex_fill,&mut cl_cum_map,&mut ex_cum_map);
        assert_eq!(client_fill.qty(), 50);
        assert_eq!(client_fill.cum_qty(), 50);
        assert_eq!(client_fill.leaves_qty(), 0);
        assert_eq!(client_fill.status().clone(), Filled);
        assert_eq!(ex_fill.qty(), 50);
        assert_eq!(ex_fill.cum_qty(), 50);
        assert_eq!(ex_fill.leaves_qty(), 50);
        assert_eq!(ex_fill.status().clone(), PartialFill);
        assert_eq!(client_fill.secondary_cl_ord_id(), "id8");
        assert_eq!(ex_fill.secondary_cl_ord_id(), "test1");
        assert_eq!(ex_order.qty(), 50);
    }

    #[test]
    fn test_update_fills_order_qty_greater_than_available_qty() {
        let input = read_input("test_data/orders.txt");
        let mut order_book = create_order_book(input);
        let cl_order = create_order_from_string("test1 IBM 150 601.1 Sell".to_string());
        order_book.add_order_to_order_book(cl_order.clone());
        let key = cl_order.get_order_book_key();
        let mut fifo = FIFOMatcher;
        let client_order_map = order_book.get_orders_for(Sell);
        let ex_order_map = order_book.get_orders_for(Buy);
        let cl_orders = client_order_map.get(&key).unwrap();
        let ex_orders = ex_order_map.get(&key).unwrap();
        let mut ex_cum_map = fifo.create_cum_qty_map(ex_orders);
        let mut cl_cum_map = fifo.create_cum_qty_map(cl_orders);
        let mut ex_order = ex_orders.clone().pop_front().unwrap();
        let mut fifo = FIFOMatcher;
        let mut client_fill = Fill::from(&cl_order);
        let mut ex_fill = Fill::from(&ex_order);

        fifo.update_fills(&cl_order, &mut ex_order, &mut client_fill, &mut ex_fill,&mut cl_cum_map,&mut ex_cum_map);

        assert_eq!(client_fill.qty(), 100);
        assert_eq!(client_fill.cum_qty(), 100);
        assert_eq!(client_fill.leaves_qty(), 50);
        assert_eq!(client_fill.status().clone(), PartialFill);
        assert_eq!(ex_fill.qty(), 100);
        assert_eq!(ex_fill.cum_qty(), 100);
        assert_eq!(ex_fill.leaves_qty(), 0);
        assert_eq!(ex_fill.status().clone(), Filled);
        assert_eq!(client_fill.secondary_cl_ord_id(), "id8");
        assert_eq!(ex_fill.secondary_cl_ord_id(), "test1");
        assert_eq!(ex_order.qty(), 0);
    }
}



