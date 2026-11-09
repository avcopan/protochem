//! Build a small graph and print it.
//!
//! Run with `cargo run --example mol_graph`.
use protochem::mol_graph::{AtomIndex, GraphError, MolGraph};

fn main() -> Result<(), GraphError> {
    let g = MolGraph::from_bonds(&[6, 6, 8], &[[0, 1], [1, 2]])?;
    println!("g: {g:?}");
    for i in [0, 1, 2] {
        let neighbors: Vec<_> = g.neighbors(AtomIndex::new(i)).collect();
        println!("neighbors: {neighbors:?}");
    }

    Ok(())
}
