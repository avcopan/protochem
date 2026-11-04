//! Build a small graph and print it.
//!
//! Run with `cargo run --example mol_graph`.
use protochem::mol_graph;

fn main() {
    let mut g = mol_graph::MolGraph::new();
    let a1 = g.add_atom(6);
    let a2 = g.add_atom(8);
    println!("a1: {a1:?}");
    println!("a2: {a2:?}");
    println!("g: {g:?}");

    let b = g.add_bond(a1, a2);
    println!("b: {b:?}");
}
