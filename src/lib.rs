#![doc = include_str!("../CRATE.md")] // the crate docs are the crates.io page
// lib.rs is the root of this library crate

mod greet; // `mod greet` tells the compiler to look for a
// src/greet.rs or src/greet/mod.rs with the module code
// (or inline mod greet { ... } in this file)

pub use greet::greet; // `greet::greet` is the path to the greet function
// `use` pulls this function into scope and `pub` makes it public

// Python bindings (only compiled when the `python` feature is enabled)
#[cfg(feature = "python")]
mod python;
