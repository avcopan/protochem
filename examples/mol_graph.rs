//! Build a graph for ethanol and water, then print its formula, neighbors, BFS layers and
//! components.
//!
//! Run with `cargo run --example mol_graph`.
use protochem::{AtomIndex, GraphError, MolGraph};

fn main() -> Result<(), GraphError> {
    // Heavy atoms only: ethanol's C0-C1-O2, and a separate water oxygen O3
    let graph = MolGraph::from_bonds(&[6, 6, 8, 8], &[[0, 1], [1, 2]])?;
    println!("formula: {}", graph.formula());
    println!("symbols: {:?}", graph.symbols());

    let neighbors: Vec<_> = graph
        .neighbors(AtomIndex::new(1))
        .map(AtomIndex::index)
        .collect();
    println!("neighbors of atom 1: {neighbors:?}");

    for (distance, layer) in graph.bfs_layers(AtomIndex::new(0)).iter().enumerate() {
        let layer: Vec<_> = layer.iter().map(|atom| atom.index()).collect();
        println!("atoms {distance} bond(s) from atom 0: {layer:?}");
    }

    for component in graph.components() {
        let component: Vec<_> = component.iter().map(|atom| atom.index()).collect();
        println!("component: {component:?}");
    }

    Ok(())
}
