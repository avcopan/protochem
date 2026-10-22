//! Look up an element and print it.
//!
//! Run with `pixi run -e dev cargo run --example elements`.
use protochem::element::{self, Element};

fn main() {
    let carbon = Element::from_atomic_number(6).expect("carbon is in the table");
    println!("{carbon:#?}");

    let nitrogen = Element::from_symbol("N").expect("nitrogen is in the table");
    println!("{nitrogen:#?}");

    for element in [carbon, nitrogen] {
        println!(
            "Organic valences of {}: {:?}",
            element.symbol,
            element::organic_valences(element.atomic_number)
        );
    }
}
