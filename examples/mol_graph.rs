//! Build a small graph and print it.
//!
//! Run with `cargo run --example mol_graph`.
use protochem::mol_graph::{self};

fn main() {
    let mut g = mol_graph::MolGraph::new();
    let i1 = g.add_atom(6);
    let i2 = g.add_atom(8);
    println!("i1: {i1:?}");
    println!("i2: {i2:?}");
    println!("g: {g:?}");
}
