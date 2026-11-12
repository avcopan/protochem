#![doc = include_str!("../CRATE.md")]

pub mod element;
pub use element::Element;

pub mod smiles_lexer;

pub mod mol_graph;
pub use mol_graph::{AtomIndex, BondIndex, GraphError, MolGraph};

#[cfg(feature = "python")]
mod python;
