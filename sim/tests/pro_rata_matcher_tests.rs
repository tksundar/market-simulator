use sim::matchers::matcher::Matcher;
use sim::matchers::prorata_matcher::ProrataMatcher;
use sim::model::domain::{Fill, OrderBook};
use sim::model::domain::Side::{Buy, Sell};
use sim::common::utils::{create_order_book,read_input};

#[test]
fn test_match_order_book() {
    let mut order_book = OrderBook::default();
    create_order_book(&mut order_book, read_input("test_data/orders.txt"));
    let mut pro = ProrataMatcher;
    let fills = pro.match_order_book(&mut order_book);

    let client_fills: Vec<Fill> = fills.clone().into_iter().filter(|f| f.side() == Buy).collect();
    let exchange_fills: Vec<Fill> = fills.clone().into_iter().filter(|f| f.side() == Sell).collect();

    assert_eq!(client_fills.len(), 4);
    assert_eq!(exchange_fills.len(), 4);
    assert_eq!(order_book.get_orders_for(Buy).len(), 3);
    assert_eq!(order_book.get_orders_for(Sell).len(), 1);
    order_book.print_market_depth_for("IBM");
}