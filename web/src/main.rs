#![feature(decl_macro)]
#[macro_use]
extern crate rocket;

use std::{env, fs};
use std::str::FromStr;
use rocket::form::Form;
use rocket::fs::FileServer;
use rocket::http::Status;

use rocket::Request;
use serde_json::{from_str, to_string};
use sim::matchers::fifo_matcher::FIFOMatcher;
use sim::matchers::matcher::Matcher;
use sim::matchers::prorata_matcher::ProrataMatcher;
use sim::model::domain::{OrderBook, OrderSingle, OrderType, Side};
use web::{get_matcher, OB, Order, persist_order_book};

#[get("/")]
fn index() -> &'static str {
    "Please fill the form"
}

#[get("/order_book")]
fn  get_order_book() -> Result<String,Status>{
    Ok(fs::read_to_string("orderbook.json").unwrap())

}

#[post("/order_entry", data = "<order_form>")]
fn add_order(order_form: Form<Order>) -> Result<String,Status> {
    let order: Order = order_form.into_inner();
    let order_single = OrderSingle::new(order.qty(),
                                        order.symbol().clone(),
                                        order.price(),
                                        Side::from(order.side()),
                                        OrderType::from(order.order_type()),
                                        order.cl_ord_id().clone());

    let algo = match env::var("ALGO"){
        Ok(algo) => algo,
        Err(_) => "FIFO".to_string()
    };

    let content = fs::read_to_string("orderbook.json").unwrap();
    let mut fills = vec![];
    if !content.is_empty() {
        let ob: OB = from_str(&content).unwrap();
        let mut order_book: OrderBook = OB::to(&ob);
        order_book.add_order_to_order_book(order_single);
        let mut matcher = get_matcher(&algo);
        fills = matcher.match_order_book(&mut order_book);
        let ob: OB = OB::from(&order_book);
        persist_order_book(&ob);
    } else {
        let mut order_book = OrderBook::default();
        order_book.add_order_to_order_book(order_single);
        let ob = OB::from(&order_book);
        persist_order_book(&ob);
    }
    let fills_str = to_string(&fills).unwrap();
    Ok(fills_str)
}

#[catch(404)]
fn not_found(req: &Request) -> String {
    format!("Oh no! We couldn't find the requested path '{}'", req.uri())
}

#[catch(422)]
fn malformed(req: &Request) -> String {
    // println!("{:#?}",req);
    format!("the submitted data could not be processed! '{:#?}'", req)
}

#[launch]
fn rocket() -> _ {
    rocket::build().
        register("/", catchers![malformed, not_found]).
        mount("/", routes![index,add_order,get_order_book]).
        mount("/", FileServer::from("static/"))
}


