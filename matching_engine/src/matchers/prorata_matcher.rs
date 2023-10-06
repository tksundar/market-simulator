use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{Receiver, Sender};
use std::thread::sleep;
use std::time::Duration;

use log::{error, info};

use crate::matchers::matcher::Matcher;
use crate::model::domain::{Fill, OrderBook, OrderSingle};
use crate::model::domain::Side::{Buy, Sell};
use crate::model::domain::Status::{Filled, PartialFill};
use crate::utils::{Aggregator, Sigma};

pub struct ProrataMatcher ;

impl ProrataMatcher {


    fn proportional_match(&mut self, buy_orders: &mut VecDeque<OrderSingle>, sell_orders: &mut VecDeque<OrderSingle>) -> Vec<Fill> {
        let mut fills = vec![];
        let mut c_map = self.create_cum_qty_map(buy_orders);
        let mut e_map = self.create_cum_qty_map(sell_orders);
        for sell_order in sell_orders.clone().iter_mut() {
            let sell_order_qty = sell_order.qty();
            let items: Vec<OrderSingle> = buy_orders.clone().into_iter().collect();
            let total = Aggregator::sigma(&items);
            for order in buy_orders.clone().iter_mut() {
                let ratio: f64 = order.qty() as f64 / total as f64;
                let qty: f64 = sell_order_qty as f64 * ratio;
                let fill_qty = qty.floor() as u32;
                fills.push(self.create_client_fill(order, sell_order.cl_ord_id(), &fill_qty, &mut c_map, buy_orders));
                fills.push(self.create_exchange_fill(sell_order, order.cl_ord_id(), &fill_qty, &mut e_map, sell_orders));
            }
            let total_filled_qty = Aggregator::sigma(&fills);
            if total_filled_qty >= sell_order.qty() {
                sell_orders.pop_front();
            } else {
                sell_order.set_qty(sell_order.qty() - total_filled_qty);
            }
        }

        fills
    }

    fn create_client_fill(&self, order: &mut OrderSingle, sec_id: &String,
                          fill_qty: &u32,
                          c_map: &mut HashMap<String, u32>,
                          buy_orders: &mut VecDeque<OrderSingle>) -> Fill {


        //println!("sell order qty {} fill qty {}", sell_order.qty(), fill_qty);
        let mut cl_fill = Fill::from(order);
        let mut cl_cum_qty = c_map.get(order.cl_ord_id()).unwrap().clone();
        // println!("order cl_ord_id {}, order cum_qty {}",order.qty(),cl_cum_qty);
        cl_fill.set_qty(fill_qty.clone());
        cl_cum_qty += fill_qty;
        cl_fill.set_cum_qty(cl_cum_qty);
        cl_fill.set_secondary_cl_ord_id(sec_id.clone());
        c_map.insert(order.cl_ord_id().clone(), cl_cum_qty);

        cl_fill.set_leaves_qty(order.qty() - fill_qty);
        if cl_fill.leaves_qty() == 0 {
            cl_fill.set_status(Filled);
            buy_orders.pop_front();
        } else {
            order.set_qty(order.qty() - fill_qty);
            cl_fill.set_status(PartialFill);
            buy_orders.pop_front();
            buy_orders.push_back(order.clone());
        }
        cl_fill
    }

    fn create_exchange_fill(&self, sell_order: &mut OrderSingle, sec_id: &String, fill_qty: &u32, e_map: &mut HashMap<String, u32>, sell_orders: &mut VecDeque<OrderSingle>) -> Fill {
        let mut ex_fill = Fill::from(sell_order);
        let mut ex_cum_qty = e_map.get(sell_order.cl_ord_id()).unwrap().clone();
        ex_cum_qty += fill_qty;
        ex_fill.set_qty(fill_qty.clone());
        ex_fill.set_cum_qty(ex_cum_qty);
        ex_fill.set_leaves_qty(sell_order.qty() - ex_cum_qty);
        ex_fill.set_secondary_cl_ord_id(sec_id.clone());
        e_map.insert(sell_order.cl_ord_id().clone(), ex_cum_qty);
        // self.set_ex_cum_qty(ex_fill.cum_qty());
        ex_cum_qty = ex_fill.cum_qty();
        if ex_fill.leaves_qty() == 0 {
            ex_fill.set_status(Filled);
            sell_orders.pop_front();
        } else {
            ex_fill.set_status(PartialFill);
        }
        ex_fill
    }

/*    fn create_cum_qty_map(&self, buy_orders: &VecDeque<OrderSingle>) -> HashMap<String, u32> {
        let mut map = HashMap::new();

        for order in buy_orders {
            map.insert(order.cl_ord_id().clone(), 0);
        }
        map
    }*/
}


impl Matcher for ProrataMatcher {
    fn start(&mut self, tx: &Sender<OrderBook>, rx: &Receiver<OrderBook>) {
        info!("Starting Prorata Matcher...");
        loop {
            let mut ob = match rx.recv() {
                Ok(ob) => ob,
                Err(e) => {
                    error!("Error receiving Order Book {}",e);
                    OrderBook::default()
                }
            };
            self.match_order_book(&mut ob);
            if let Err(e) = tx.send(ob) {
                error!("Error sending order book {}",e);
            }
            sleep(Duration::from_secs(1));
        }
    }

    /// Matches the [`OrderBook`] according to the ratios of buy side order quantities
    ///
    /// # Logic:
    /// Assume two buy orders o1 and o2 exist for quantities n1 and n2 received at time t1 and t2, t2 > t1
    /// Assume one sell order o3 exists for quantity n3 at t3 t3 > t2 > t1
    /// Then o1 will be get fills for quantity n1/(n1_n2) and o2 will get fills for quantity n2/(n1+m2)
    ///
    /// # Example:
    /// Buy Order o1 => n1 = 300;
    /// Buy order o2 => n2 = 100;
    /// Sell Order o3 -> n3 = 300;
    ///
    /// O1 fill = n1/(n1+n2) or 3/4th of 300  = 225
    /// 02 fill = n2/(n1+n2) or 1/4th of 300 = 75
    ///```rust
    /// use matching_engine::common::utils::{create_order_book, read_input};
    /// use matching_engine::matchers::fifo_matcher::FIFOMatcher;
    /// use matching_engine::matchers::matcher::Matcher;
    /// use matching_engine::matchers::prorata_matcher::ProrataMatcher;
    /// let input = read_input("test_data/orders.txt");
    /// let mut order_book = create_order_book(input);
    /// //create a matcher
    /// let mut  matcher = ProrataMatcher;
    /// // match the order book with the matcher to produce executions
    /// let mut fills = matcher.match_order_book(&mut order_book);
    /// ```
    fn match_order_book(&mut self, order_book: &mut OrderBook) -> Vec<Fill> {
        let mut buy_map = order_book.get_orders_for(Buy);
        let mut sell_map = order_book.get_orders_for(Sell);
        let mut all_fills = vec![];
        for (key, buy_orders) in buy_map.clone().iter_mut() {
            if sell_map.contains_key(key) {
                let mut clone = sell_map.clone();
                let mut sell_orders = clone.get_mut(key).unwrap();
                let fills = self.proportional_match(buy_orders, &mut sell_orders);
                for fill in fills {
                    all_fills.push(fill);
                }
                if sell_orders.is_empty() {
                    sell_map.remove(key);
                }
            } else {
                continue;
            }

            if buy_orders.is_empty() {
                buy_map.remove(key);
            }
        }

        order_book.update_order_book(buy_map, Buy);
        order_book.update_order_book(sell_map, Sell);
        Fill::pretty_print(&all_fills);
        all_fills
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::matchers::prorata_matcher::ProrataMatcher;
    use crate::model::domain::{Fill, Status};
    use crate::model::domain::Side::{Buy, Sell};
    use crate::model::domain::Status::{Filled, PartialFill};
    use crate::utils::create_order_from_string;

    #[test]
    fn test_proportional_match() {
        let mut buy_orders = VecDeque::new();
        buy_orders.push_back(create_order_from_string("id8 IBM 300 602.5 Buy".to_string()));
        buy_orders.push_back(create_order_from_string("id7 IBM 100 602.5 Buy".to_string()));
        let mut sell_orders = VecDeque::new();
        sell_orders.push_back(create_order_from_string("id9 IBM 300 602.5 Sell".to_string()));
        sell_orders.push_back(create_order_from_string("id10 IBM 100 602.5 Sell".to_string()));
        let mut pro = ProrataMatcher;
        let fills = pro.proportional_match(&mut buy_orders, &mut sell_orders);

        //assertions
        let _client_fills: VecDeque<Fill> = fills.clone().into_iter().filter(|f| f.side() == Buy).collect();

        let fills_for_id8: VecDeque<Fill> = fills.clone().into_iter().filter(|f| f.cl_ord_id() == "id8").collect();

        assert_eq!(fills_for_id8.len(), 2);
        for fill in fills_for_id8 {
            if fill.secondary_cl_ord_id() == "id9" {
                assert_fills(&fill, 225, 225, 75, PartialFill);
            } else if fill.secondary_cl_ord_id() == "id10" {
                assert_fills(&fill, 75, 300, 0, Filled);
            }
        }

        let fills_for_id7: VecDeque<Fill> = fills.clone().into_iter().filter(|f| f.cl_ord_id() == "id7").collect();
        for fill in fills_for_id7 {
            if fill.secondary_cl_ord_id() == "id9" {
                assert_fills(&fill, 75, 75, 25, PartialFill)
            } else if fill.secondary_cl_ord_id() == "id10" {
                assert_fills(&fill, 25, 100, 0, Filled)
            }
        }

        let exchange_fills: VecDeque<Fill> = fills.clone().into_iter().filter(|f| f.side() == Sell).collect();


        let ex_fills_for_id9: Vec<Fill> = exchange_fills.clone().into_iter().filter(|f| f.cl_ord_id() == "id9").collect();
        for fill in ex_fills_for_id9 {
            if fill.secondary_cl_ord_id() == "id8" {
                assert_fills(&fill, 225, 225, 75, PartialFill);
            } else if fill.secondary_cl_ord_id() == "id7" {
                assert_fills(&fill, 75, 300, 0, Filled);
            }
        }

        let ex_fills_for_id10: Vec<Fill> = exchange_fills.clone().into_iter().filter(|f| f.cl_ord_id() == "id10").collect();
        for fill in ex_fills_for_id10 {
            if fill.secondary_cl_ord_id() == "id8" {
                assert_fills(&fill, 75, 75, 25, PartialFill);
            } else if fill.secondary_cl_ord_id() == "id7" {
                assert_fills(&fill, 25, 100, 0, Filled);
            }
        }
    }

    fn assert_fills(fill: &Fill, fill_qty: u32, cum_qty: u32, leaves_qty: u32, status: Status) {
        assert_eq!(fill.qty(), fill_qty);
        assert_eq!(fill.cum_qty(), cum_qty);
        assert_eq!(fill.leaves_qty(), leaves_qty);
        assert_eq!(fill.status().clone(), status);
    }
}


