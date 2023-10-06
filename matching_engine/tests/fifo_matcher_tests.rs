use matching_engine::common::utils::{create_order_book, read_input};
use matching_engine::matchers::fifo_matcher::FIFOMatcher;
use matching_engine::matchers::matcher::Matcher;
use matching_engine::model::domain::{Fill, OrderBook, OrderBookKey};
use matching_engine::model::domain::Side::{Buy, Sell};


#[test]

fn test_fifo_match_order_book(){
    let mut order_book = create_order_book(read_input("test_data/orders.txt"));
    let mut fifo = FIFOMatcher;
    fifo.match_order_book(&mut order_book);
    let buy = order_book.get_orders_for(Buy);
    let sell = order_book.get_orders_for(Sell);
    assert_eq!(buy.len(),3);
    assert_eq!(sell.len(),1);
}

#[test]
fn test_match_order_multiple_buy_orders_against_a_single_sell_order() {
    let input = read_input("test_data/test1.txt");
    let mut order_book = create_order_book(input);
    let key2 = OrderBookKey::new(602.5, "TATA".to_string());
    let buy = order_book.get_orders_for(Buy);
    let sell = order_book.get_orders_for(Sell);

    assert_eq!(buy.len(), 1);
    assert_eq!(sell.len(), 1);
    assert_eq!(buy.contains_key(&key2), true);
    assert_eq!(sell.contains_key(&key2), true);

    let buy_orders = buy.get(&key2).unwrap();
    assert_eq!(buy_orders.len(),2);
    let sell_orders = sell.get(&key2).unwrap();
    assert_eq!(sell_orders.len(),1);
    let mut fifo = FIFOMatcher;
    let fills = fifo.match_order_book(&mut order_book);
    Fill::pretty_print(&fills);
    let client_fills: Vec<Fill> = fills.clone().into_iter().filter(|f| f.side() == Buy).collect();
    let ex_fills: Vec<Fill> = fills.clone().into_iter().filter(|f| f.side() == Sell).collect();
}