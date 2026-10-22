// lib.rs is the root of this library crate.
// The crate docs are included from CRATE.md, which is also the crates.io page.
#![doc = include_str!("../CRATE.md")]

// Declare the `greet` module, whose code the compiler looks for in src/greet.rs or
// src/greet/mod.rs (or inline, as `mod greet { ... }` in this file).
mod greet;

// Re-export the greet function (path `greet::greet`, i.e. module::function): `use`
// brings it into scope and `pub` makes it public.
pub use greet::greet;

// The Python bindings are only compiled when the `python` feature is enabled.
#[cfg(feature = "python")]
mod python;
