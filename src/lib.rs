#![doc = include_str!("../CRATE.md")] // the crate docs are the crates.io page

mod greet;

pub use greet::greet;

// Python bindings (only compiled when the `python` feature is enabled)
#[cfg(feature = "python")]
mod python;
