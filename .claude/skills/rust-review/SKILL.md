---
name: rust-review
description: >
  Review Rust code for idiom and API design against the Rust API Guidelines
  (C-* codes) and the Rust Book's style conventions (e.g. idiomatic `use`
  paths, a convenient public API). Use when the user asks for a review of Rust
  code or says a piece of Rust work is "done", and when designing or writing
  public Rust APIs, doc comments, or example code. Covers what Clippy cannot
  check; run the linter first.
metadata:
  version: "1.0"
  tags:
    - rust
    - code-review
    - api-design
---

# Rust review: API Guidelines and idiom

Sources (cite them by code in review comments, e.g. "C-GETTER"):

- Rust API Guidelines: <https://rust-lang.github.io/api-guidelines/checklist.html>
- Rust Book 7.4, idiomatic `use` paths:
  <https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html#creating-idiomatic-use-paths>
- Rust Book 14.2, doc comments and a convenient public API:
  <https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html#exporting-a-convenient-public-api>

## Procedure

1. **Run the linter first**: `pixi run lint` (Clippy pedantic with `-D warnings`)
   and `pixi run test`. Report failures verbatim. Don't repeat in prose what a
   lint already reports; the checklist below is for what lints *can't* catch.
2. **Walk the judgment checklist** over every changed public item, then skim
   private code for the idiom items.
3. **Write the review** grouped by severity: correctness → API design →
   idiom/style → docs. Each comment names the item, the guideline code, and a
   concrete fix. When a guideline is a judgment call ("within reason"), say so
   and say which way you'd lean and why.
4. **Project conventions win.** If the project's `CLAUDE.md` or planning docs
   say otherwise (e.g. "the user writes all code, so only suggest fixes"), follow
   them.

## Already enforced by lints (don't re-review)

Configured in `Cargo.toml` (`[lints]`), so `pixi run lint` covers them:

| Lint | Guideline |
|---|---|
| `missing_docs` | C-CRATE-DOC, C-EXAMPLE (presence only, not quality) |
| `missing_debug_implementations` | C-DEBUG |
| `clippy::cargo_common_metadata` | C-METADATA |
| `clippy::missing_errors_doc`, `missing_panics_doc` (pedantic) | C-FAILURE |
| `clippy::wrong_self_convention` | C-CONV (receiver vs. prefix) |
| `clippy::from_over_into` | C-CONV-TRAITS |
| `clippy::new_without_default` | C-COMMON-TRAITS (`new` + `Default`) |
| `clippy::needless_pass_by_value` (pedantic) | C-CALLER-CONTROL (half of it) |
| `clippy::iter_without_into_iter` (pedantic) | C-ITER |
| `clippy::fn_params_excessive_bools` (pedantic) | C-CUSTOM-TYPE (partly) |
| `clippy::result_unit_err` | C-GOOD-ERR (no `()` errors) |
| `clippy::must_use_candidate` (pedantic) | (not a guideline, but expected) |

## Judgment checklist

### Naming

- **C-CASE**: acronyms are one word in `UpperCamelCase` (`Uuid`, not `UUID`;
  `HttpClient`, not `HTTPClient`). No `-rs`/`-rust` in crate names.
- **C-CONV**: `as_` = free, borrowed→borrowed; `to_` = costly, or Copy→Copy;
  `into_` = consumes self. Name by *cost*, not just by type. A single-value
  wrapper exposes `into_inner()`.
- **C-GETTER**: no `get_` prefix (`name()`, not `get_name()`); `_mut`
  suffix for the mutable twin. `get` alone only for fallible keyed lookup.
- **C-ITER / C-ITER-TY**: `iter`/`iter_mut`/`into_iter` for homogeneous
  collections; iterator types named after the method (`Iter`, `Keys`,
  `Neighbors`). Non-homogeneous things get descriptive names (`nodes()`,
  `edges()`), like `str::chars`.
- **C-WORD-ORDER**: one word order for error types across the crate
  (std uses verb-object-error: `ParseAddrError`).
- **C-FEATURE**: feature names without `use-`/`with-`; additive only.

### Interoperability

- **C-COMMON-TRAITS**: every public type derives what applies, in this order:
  `Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default`.
  Ask for each: why *not*? `Copy` for small plain-data types (fieldless
  enums, small value structs). `Ord` only if there's a meaningful order.
  `Display` for anything with a canonical text form.
- **C-CONV-TRAITS**: `From`/`TryFrom` for conversions (never `Into`). `TryFrom`
  when it can fail. `FromStr` for parsing text (enables `"x".parse::<MyType>()`).
- **C-COLLECT**: collection types implement `FromIterator` and `Extend`.
- **C-SERDE**: data structures get `Serialize`/`Deserialize` behind an optional
  feature named exactly `serde`.
- **C-GOOD-ERR**: error types implement `std::error::Error + Send + Sync`,
  `Display` messages are lowercase with no trailing period, and they carry
  the context needed to act on them (position, offending input).

### Predictability

- **C-CTOR**: constructors are inherent static methods; `new` is the primary
  one; secondary ones `with_*`; conversions `from_*` (or a `From` impl when the
  source type alone determines the meaning).
- **C-METHOD**: an operation with a clear receiver is a method, not a free
  function.
- **C-NO-OUT**: return tuples/structs, not `&mut` out-parameters (exception:
  reusing a caller's buffer).
- **C-CONV-SPECIFIC**: conversions live on the more specific type.
- **C-OVERLOAD / C-DEREF**: operators only where they mean what they look like;
  `Deref` only on smart pointers (never to "inherit" methods from a wrapped type).

### Flexibility

- **C-CALLER-CONTROL**: take ownership only if you keep the value; otherwise
  borrow. Don't take `&T` then `.clone()` inside.
- **C-GENERIC**: accept the weakest thing that works: `&str` over `&String`,
  `&[T]` over `&Vec<T>`, `impl IntoIterator<Item = T>` when you only iterate,
  `impl AsRef<Path>` for paths. Don't over-generalize internal code.
- **C-INTERMEDIATE**: if a function computes something useful on the way
  (a position, a partial result), consider returning it.

### Type safety

- **C-NEWTYPE**: distinct meanings of the same primitive get distinct types
  (node index vs. edge index; meters vs. seconds).
- **C-CUSTOM-TYPE**: no `bool`/`Option` arguments whose meaning isn't obvious at
  the call site; use an enum.
- **C-BUILDER**: many optional inputs → a builder (non-consuming preferred).

### Dependability and future-proofing

- **C-VALIDATE**: prefer types that make bad input unrepresentable; else
  validate at the boundary and return `Result`; `debug_assert!` for costly
  internal invariants; `_unchecked` siblings only with a measured need.
- **C-STRUCT-PRIVATE**: struct fields private unless the struct is passive data;
  invariants live behind constructors and accessors.
- **C-NEWTYPE-HIDE**: don't leak iterator adaptor chains or internal
  containers (e.g. a dependency's collection types) in public signatures; return a newtype or
  `impl Iterator`.
- **C-STRUCT-BOUNDS**: no derivable-trait bounds on struct definitions.
- **C-SEALED**: traits that downstream crates shouldn't implement are sealed.
- **C-STABLE**: watch for dependency types leaking into the public API
  (including through `From` impls on public error types).

### Documentation

- **C-EXAMPLE**: public items have a doctest that shows *why* you'd use it, not
  only how to call it. A shared example linked from related items is fine.
- **C-QUESTION-MARK**: examples use `?` with a hidden
  `# fn main() -> Result<(), Box<dyn std::error::Error>> {`, never `unwrap()`.
- **C-FAILURE**: `# Errors`, `# Panics`, `# Safety` sections (lints check
  presence; review whether the content is accurate and complete).
- **Doc comments (14.2)**: `//!` at the top of `lib.rs` and of each public module
  explains the purpose and how its parts fit together (here `lib.rs` includes
  `CRATE.md`). `///` on items. The usual sections are `# Examples`, `# Errors`,
  `# Panics`, and `# Safety`.
- **C-LINK**: intra-doc links (``[`MyType`]``) in prose.
- **C-HIDDEN**: `pub(crate)` for crate-internal helpers; `#[doc(hidden)]` on
  impls users can't use.

## `use` paths, module layout, and the public API (Rust Book 7.4–7.5, 14.2)

- **Functions: import the parent module**, call as `module::function()`
  (`use crate::parse::lexer;` then `lexer::tokenize(s)`). The call site shows
  the function isn't local.
- **Types, traits, enums, structs: import the full path**
  (`use std::collections::HashMap;`, `use crate::graph::Graph;`).
- **Name clashes: import the parent modules** (`fmt::Result` vs. `io::Result`)
  **or alias with `as`** (`use std::io::Result as IoResult;`). Both are
  idiomatic.
- **Nested paths** to merge imports with a shared prefix:
  `use std::{cmp::Ordering, io};` and `use std::io::{self, Write};`.
- **Globs (`*`)**: only for `use super::*;` in `#[cfg(test)] mod tests` and for
  preludes (e.g. `use pyo3::prelude::*;`). Nowhere else.
- **A convenient public API (14.2): `pub use` re-exports** shape the public API
  to the user's mental model, not the developer's file layout. Users shouldn't
  have to learn the module tree to find the important items:
  - Re-export central *types* at the root (`pub use graph::Graph;`, so users
    write `use my_crate::Graph;`, not `my_crate::graph::Graph`).
  - Re-export functions too when their module is just an organizing bucket
    (14.2's `art::utils::mix` becomes `art::mix`).
  - Keep a `pub mod` when it is a meaningful namespace that users would name
    themselves (like `std::mem::swap`).
  - Flag modules that are `pub` only because of the file layout, and items that
    are hard to find from the front page of `cargo doc --open`. Re-exports are
    listed there, so check the rendered docs from the user's side.
- **Visibility**: smallest that works (`pub(crate)` over `pub` for internals).
  A `pub enum` makes all variants public; a `pub struct` keeps fields private
  unless each is marked `pub` (see C-STRUCT-PRIVATE).
- **Files**: one module per file (`src/graph.rs`); for submodules prefer
  `src/parse.rs` + `src/parse/lexer.rs` over `src/parse/mod.rs`.

## This template's layout

- **Python bindings** (`src/python.rs`) follow Python conventions (Google
  docstrings, `snake_case` functions, `__version__`), not these guidelines.
  Review the Rust API and the Python API separately.
- **Crate docs** come from `CRATE.md` (included by `lib.rs`), which is also the
  crates.io page. `pixi run docs-rust` fails on rustdoc warnings such as broken
  intra-doc links.
- **Doctests** run with `cargo test`, so ask for them on public items (C-EXAMPLE,
  C-QUESTION-MARK).
