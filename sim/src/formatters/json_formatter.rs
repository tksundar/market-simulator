use serde_json::{from_str, to_string, Value};

use crate::formatters::formatter::Formatter;
use crate::model::domain::{Fill, OrderSingle};

// Import Serde and Serde JSON macros

///Provides implementation to convert OrderSingle and Fill to and from json format
pub struct JsonFormatter {}

impl JsonFormatter {
    pub fn is_valid_json(val: &str) -> bool {
        match from_str::<Value>(val) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
}

impl Formatter<Fill> for JsonFormatter {
    fn format_to(fill: Fill) -> String {
        to_string(&fill).expect("Error serializing fill")
    }

    fn format_from(data: String) -> Fill {
        from_str(&data).unwrap()
    }
}

impl Formatter<OrderSingle> for JsonFormatter {
    fn format_to(order: OrderSingle) -> String {
        to_string(&order).expect("Error serializing order")
    }
    fn format_from(data: String) -> OrderSingle {
        from_str(&data).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::to_string;

    use crate::formatters::formatter::Formatter;
    use crate::formatters::json_formatter::JsonFormatter;
    use crate::model::domain::Fill;
    use crate::utils::create_order_from_string;

    #[test]
    fn test_serialize_fill_and_order() {
        let order = create_order_from_string("id1 IBM 20 601.5 Buy".to_string());
        let string_val = to_string(&order).unwrap();
        println!("{string_val}");
        assert!(JsonFormatter::is_valid_json(&string_val));
        let fill = Fill::from(&order);
        let string_val = JsonFormatter::format_to(fill);
        println!("{string_val}");
        assert!(JsonFormatter::is_valid_json(&string_val))
    }
}