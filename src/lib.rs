#![doc = include_str!("../CRATE.md")]

pub mod element;
pub use element::Element;

#[cfg(feature = "python")]
mod python;
