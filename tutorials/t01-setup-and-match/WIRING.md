# How `src/*.rs` is wired together

A companion to Tutorial 1, Part 1: how `lib.rs`, `greet.rs` and `python.rs` fit
together, with Python comparisons where they help.

## Attributes: `#[...]` and `#![...]`

`#[something]` is an **attribute**. It annotates the item directly below it (a
function, module, const, etc.). It looks like a Python decorator, but it acts at
**compile time**: it can change what gets compiled, generate code, or turn
warnings on and off. Nothing wraps your function at runtime.

`#![something]` (with `!`) annotates the item it sits *inside*. In `lib.rs:1`,
`#![doc = include_str!("../CRATE.md")]` applies to the whole crate: the crate's
top-level docs are read in from `CRATE.md`.

## `greet.rs`

```rust
/// Greet a person by their name.        <- doc comment (Markdown), becomes rustdoc
/// # Examples
/// ```
/// assert_eq!(protochem::greet("World"), "Hello, World!");   <- doc test
/// ```
#[must_use]
pub fn greet(name: &str) -> String { ... }
```

- **`///`** is a doc comment. Code blocks inside it are **doc tests**: `cargo
  test` compiles and runs them as if they were outside code calling your crate.
  That's why the example says `protochem::greet`, not just `greet`.
- **`#[must_use]`** makes the compiler warn if a caller drops the return value,
  as in `greet("Jim");` on its own line. For a function with no side effects,
  ignoring the result is almost always a bug. Clippy pedantic
  (`must_use_candidate`) asks for this on every such public function, so you'll
  be adding it a lot.
- **`pub`** makes the function visible outside its module. Everything in Rust is
  private by default. Python is the reverse: everything is public, and `_` is
  only a convention.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() { assert_eq!(greet("Jim"), "Hello, Jim!"); }
}
```

- **`#[cfg(...)]`** is conditional compilation. The item below it only exists if
  the condition holds; otherwise the compiler drops it as if it were never
  written. `test` is true only under `cargo test`, so the test module is left out
  of normal builds.
- **`mod tests { ... }`** is an **inline module**, a namespace written inside the
  file. The tests sit in the same file as the code they test, which is the normal
  Rust convention for unit tests.
- **`use super::*`**: `super` is the parent module (here, `greet`), and `*`
  imports everything from it. It's like `from .. import *`. A child module can
  see its parent's **private** items too: privacy in Rust means "visible in this
  module and its descendants". So unit tests can test private helpers.
- **`#[test]`** marks a function for the test runner. pytest finds tests by their
  `test_` prefix; Rust uses this attribute instead (the `test_` in the name is
  just habit). A test passes if it returns normally and fails if it panics, which
  is what `assert_eq!` does on a mismatch.

## `lib.rs`: the crate root

```rust
mod greet;               // declare the module; its body lives in src/greet.rs
pub use greet::greet;    // re-export: makes it available as protochem::greet

#[cfg(feature = "python")]
mod python;              // body in src/python.rs, compiled only with the feature
```

- **Files aren't modules until they're declared.** In Python, any `.py` file in a
  package can be imported. In Rust, `src/greet.rs` is ignored until the crate
  root says `mod greet;`. Then the compiler looks for `src/greet.rs` (or
  `src/greet/mod.rs`). Modules form a tree that starts at `lib.rs`.
- **`mod greet;` is private, and `pub use greet::greet;` re-exports the
  function.** Outside code sees `protochem::greet` but not the module layout.
  It's like putting `from ._greet import greet` in an `__init__.py`. The name
  `greet::greet` is *module*`::`*function*. They can share a name because modules
  and functions are looked up in separate namespaces.
- **`#[cfg(feature = "python")]`**: features are on/off switches declared in
  `Cargo.toml`:
  ```toml
  [features]
  python = ["dep:pyo3"]          # turning on "python" pulls in the optional pyo3 dependency
  ```
  - maturin turns it on (`features = ["python"]` under `[tool.maturin]` in
    `pyproject.toml`).
  - Plain `cargo test` does not turn it on, so the pure-Rust library builds and
    tests without Python or pyo3.
  - `pixi run lint` passes `--all-features`, so clippy still checks `python.rs`.

**Why there's both a `mod greet` and a `mod python`:** it separates the logic
from the bindings.

- `greet` (and later `element`, `smiles_lexer`, ...) is plain Rust that knows
  nothing about Python.
- `python` is a thin translation layer on top.
- Neither module is `pub`. Rust users get the re-exports. Python reaches the
  bindings through a symbol pyo3 generates and exports from the compiled library
  no matter what Rust's privacy rules say.

## `python.rs`

```rust
use pyo3::prelude::*;            // glob import of pyo3's common names (the "prelude")

#[pymodule]                      // this Rust module *is* a Python module...
mod _core {                      // ...named `_core`, matching module-name = "protochem._core"
    use pyo3::prelude::*;        // imports don't carry into child modules, so import again

    #[pymodule_export]           // expose this const as a module attribute
    #[allow(non_upper_case_globals)]   // silence the "consts should be UPPER_CASE" lint
    const __version__: &str = env!("CARGO_PKG_VERSION");

    #[pyfunction]                // generate a Python-callable wrapper
    fn greet(name: &str) -> String {
        crate::greet(name)       // call the real implementation
    }
}
```

- **`#[pymodule]` on `mod _core`**: pyo3 generates the C entry point
  (`PyInit__core`) that Python looks for when it imports the compiled `.so`.
  Every `#[pyfunction]` and `#[pymodule_export]` inside the module is registered
  automatically.
- **The second `use pyo3::prelude::*`** is needed because a `use` only applies to
  the module it's written in. `_core` is a new module, so it imports again.
  (`use super::*` would also work.)
- **`env!("CARGO_PKG_VERSION")`**: the `!` marks a **macro**, as with `format!`
  and `assert_eq!`. `env!` reads an environment variable **at compile time**.
  Cargo sets `CARGO_PKG_VERSION` from `Cargo.toml`, so `__version__` always
  matches the crate version, and `tbump` only has to update one place.
- **`#[pyfunction]`** generates the glue code. It converts the Python `str`
  argument to `&str`, calls your function, converts the `String` result back to a
  Python `str`, and turns errors into exceptions. Your function stays plain Rust.
- **`crate::greet(name)`** is an absolute path starting from the crate root, i.e.
  the re-export in `lib.rs`. The `crate::` is required: inside `_core`, a bare
  `greet` means the wrapper itself, so it would call itself recursively forever.
- The doc comments here use **Google docstring style** (`Args:` / `Returns:`)
  instead of rustdoc Markdown. `pixi run stubs` copies them into `_core.pyi`,
  where Python tools show them as docstrings.

## How a Python call reaches your Rust code

```
import protochem
  → python/protochem/__init__.py:  from ._core import __version__, greet
  → _core.abi3.so  (built by maturin from lib.rs with feature "python";
                    crate-type "cdylib" = a loadable shared library)
  → #[pymodule] mod _core  →  #[pyfunction] greet  →  crate::greet  →  src/greet.rs
```

`Cargo.toml` asks for two build outputs, `crate-type = ["cdylib", "rlib"]`:

- **`cdylib`** is the shared library Python loads.
- **`rlib`** is the normal Rust library that `cargo test`, doc tests and other
  Rust crates link against.

## Adding a new module (T01 and later)

1. Write `src/element.rs`.
2. Add `mod element;` and a `pub use` in `lib.rs` (or make it `pub mod element;`
   if you want the `protochem::element::symbol` path the tutorial uses).
3. Add a `#[pyfunction]` wrapper inside `_core` that calls `crate::...`.
4. Run `pixi run stubs` and add the name to `python/protochem/__init__.py`.
