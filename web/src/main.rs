#![feature(decl_macro)]
#[macro_use]
extern crate rocket;

use std::{env, fs};
use std::str::FromStr;

use log::Log;
use rocket::{Data, Request, State};
use rocket::get;
use rocket::data::ByteUnit;
use rocket::form::Form;
use rocket::fs::FileServer;
use rocket::http::Status;
use rocket::response::content::RawHtml;
use serde_json::{from_str, to_string};

use sim::common::utils::{create_order_from_string, log};
use sim::model::domain::{Fill, OrderBook, OrderSingle, OrderType, Side};
use web::{create_order_book_table, get_matcher, OB, Order, persist_order_book};

const ORDER_BOOK_FILE: &str = "orderbook.json";
static LOG_FILE: &str = "web/logs/web.log";

struct LOGGER;

#[get("/")]
fn index() -> &'static str {
    "Please fill the form"
}

#[get("/order_book/<format>")]
fn get_order_book(format: &str) -> Result<RawHtml<String>, Status> {
    let  content : String = fs::read_to_string(ORDER_BOOK_FILE).unwrap();
    let mut res = RawHtml(content.clone());
    if format == "pretty" {
        let ob: OB = from_str(&content).unwrap();
        let order_book = OB::to(&ob);
        res = create_order_book_table(&order_book);
    }
    Ok(res)
}

#[post("/order_entry", data = "<order_form>")]
fn add_order(order_form: Form<Order> ) -> Result<String, Status> {
    let order: Order = order_form.into_inner();
    log(&format!("Received order {}", to_string(&order).unwrap()), LOG_FILE);
    let order_single = OrderSingle::new(order.qty(),
                                        order.symbol().clone(),
                                        order.price(),
                                        Side::from(order.side()),
                                        OrderType::from(order.order_type()),
                                        order.cl_ord_id().clone());

    let algo = match env::var("ALGO") {
        Ok(algo) => algo,
        Err(_) => "FIFO".to_string()
    };

    let content = match fs::read_to_string(ORDER_BOOK_FILE){
        Ok(data) => data,
        Err(_) => String::new(),
    };
    let mut fills = vec![];
    if !content.is_empty() {
        let ob: OB = from_str(&content).unwrap();
        let mut order_book: OrderBook = OB::to(&ob);
        order_book.add_order_to_order_book(order_single);
        let mut matcher = get_matcher(&algo);
        fills = matcher.match_order_book(&mut order_book);
        let ob: OB = OB::from(&order_book);
        persist_order_book(&ob,ORDER_BOOK_FILE);
    } else {
        let mut order_book = OrderBook::default();
        order_book.add_order_to_order_book(order_single);
        let ob = OB::from(&order_book);
        persist_order_book(&ob,ORDER_BOOK_FILE);
    }
    let mut fill_str = to_string(&fills).unwrap();
    if order.format() == "pretty" {
        fill_str = Fill::pretty_print(&fills);
    }
    Ok(fill_str)
}

fn match_order() {}

#[get("/reset")]
fn reset() -> Result<String,Status>{
        let mut message = String::new();
        if let Err(err) = fs::remove_file(ORDER_BOOK_FILE) {
            eprintln!("Error deleting file: {}", err);
        } else {
           message.push_str("Order book deleted successfully")
        }

       Ok(message)
}


#[post("/upload", data = "<data>")]
async fn upload<'a>(mut data: Data<'a>) -> Result<RawHtml<String>, Status> {
    let ds = data.open(ByteUnit::Kilobyte(1024));
    let val = ds.into_string().await.unwrap().value;
    let raw_data: Vec<&str> = val.split("\n").collect();
    let mut orders = vec![];
    for line in raw_data {
        let temp = line.trim_end_matches('\r');
        let trimmed: Vec<&str> = temp.split(' ').collect();
        if trimmed.len() == 5 {
            orders.push(temp);
        }
    }
    let mut order_book = OrderBook::default();
    for line in orders {
        let order = create_order_from_string(line.to_string());
        order_book.add_order_to_order_book(order);
    }
    let ob: OB = OB::from(&order_book);
    persist_order_book(&ob, ORDER_BOOK_FILE);

    Ok(create_order_book_table(&order_book))
}

#[catch(404)]
fn not_found(req: &Request) -> String {
    format!("The requested path {} , is not available ", req.uri())
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
        mount("/", routes![index,add_order,get_order_book,reset,upload]).
        mount("/", FileServer::from("web/static/")).manage(LOGGER)
}


