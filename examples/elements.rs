//! Look up common elements and print their properties and simplest hydrides.
//!
//! Run with `cargo run --example elements`.
use protochem::element::{self, Element};

fn main() {
    // Elements can be looked up by atomic number or by symbol
    let carbon = Element::from_atomic_number(6).expect("carbon is in the table");
    println!("{carbon:#?}");

    // Filling a neutral atom's lowest valence with hydrogens gives CH4, NH3, H2O and HF
    for symbol in ["C", "N", "O", "F"] {
        let element = Element::from_symbol(symbol).expect("symbol is in the table");
        let hydrogens = element::implicit_hydrogens(element.atomic_number, 0)
            .expect("symbol is in the organic subset");
        println!(
            "{symbol}: mass {:.3}, {hydrogens} hydrogen(s)",
            element.mass
        );
    }
}
