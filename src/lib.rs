#![doc = include_str!("../CRATE.md")]

pub mod element;
pub use element::Element;

pub mod smiles_lexer;

#[cfg(feature = "python")]
mod python;
