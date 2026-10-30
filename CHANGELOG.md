# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

## [0.0.2] - 2026-10-30

### Added

- `element` module with an `Element` type (re-exported at the crate root), looked up by
  symbol or atomic number from a table of all 118 elements, and `organic_valences` for
  the SMILES organic subset
- `scripts/elements_data.py` (`pixi run elements-data`), which generates the element
  table from the mendeleev database (now a dev dependency)
- `examples/elements.rs`

### Removed

- The template's `greet` example, in Rust and Python

## [0.0.1] - 2026-10-21
