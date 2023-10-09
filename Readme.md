
A simple market simulator project with the following features

* Accept an order
* Accept orders from a file
* match an order using FIFO matcher or Pro-rata
  matcher (https://corporatefinanceinstitute.com/resources/career-map/sell-side/capital-markets/matching-orders/)

Module matching_engine is the back end engine that has all the matching functionalities with a CLI. 

<H3>Matching Engine </H3>

The matching engine exposes the API required to create an order book and  to match the order book to produce executions or Fills. A typical use case will be to create the order book fro a file containing orders, one order per line as given below and then use the matching engine to run the matching algorithm as so. Please refer to the CLI section for the order format.

use matching_engine::common::utils::{create_order_book, read_input}; <br>
use matching_engine::matchers::fifo_matcher::FIFOMatcher;<br>
use matching_engine::matchers::matcher::Matcher;</p>

let input = read_input("test_data/orders.txt");<br>
let mut order_book = create_order_book(input);<br>

//create a matcher<br>
 let mut  matcher = FIFOMatcher;// or Prorata Matcher<br>
 
// match the order book with the matcher to produce executions<br>
 let mut fills = matcher.match_order_book(&mut order_book);<br>
</code>

The api is published  at https://crates.io/crates/matching_engine

<h3>CLI:</h3>

The codebase also contains a CLI interface which can be executed as follows

execute cargo run -- -h or <br>

matching_engine -h for complete usage help

Examples:

For the order file below

id1 IBM 300 602.5 Buy<br>
id2 IBM 300 602.5 Sell<br>
id3 IBM 100 602.5 Buy<br>
id4 IBM 100 602.5 Sell<br>
id5 IBM 300 602 Buy<br>
id6 IBM 300 601.9 Buy<br>
id4 IBM 100 602.1 Sell<br>

executing <i> cargo run -- prorata_test_data/orders.txt</i> will produce the following output<br>

<p><img src="images/fifo.png"/> </p>

if we executed the command using ProrataMatcher as such

executing <i> cargo run -- prorata_test_data/orders.txt PRO </i> will produce the following output<br>

<p><img src="images/prorata.png?raw=true"/> </p>











