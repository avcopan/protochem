//! Build a small graph and print it.
//!
//! Run with `cargo run --example mol_graph`.
use protochem::mol_graph::{AtomIndex, GraphError, MolGraph};

fn main() -> Result<(), GraphError> {
    // Cyclohexane heavy atoms
    let g = MolGraph::from_bonds(
        &[6, 6, 6, 6, 6, 6],
        &[[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5, 0]],
    )?;
    println!("g: {g:?}");
    for i in [0, 1, 2] {
        let neighbors: Vec<_> = g.neighbors(AtomIndex::new(i)).collect();
        println!("neighbors: {neighbors:?}");
    }
    println!("formula: {}", g.formula());

    for layer in g.bfs_layers(AtomIndex::new(0)) {
        println!("layer: {layer:?}");
    }

    Ok(())
}
