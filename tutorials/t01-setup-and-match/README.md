# T01: Sandbox Setup and `match`

**Time:** ~2 h · **Builds:** `src/element.rs` (element symbols and the SMILES
implicit-hydrogen rule)

## Why this matters

Every SMILES atom like `C` or `Cl` carries *implicit* hydrogens, and
StereoMolGraph works with *explicit* hydrogens only. So the SMILES reader you'll
build in T02–T05 needs an element table and the organic-subset valence rule. That
is this tutorial's exercise. Along the way you set up the sandbox, get used to the
template's tooling, and practice `match`, which is where you stopped in the book.

## Reading (≈ 40 min)

- Rust book **6.2** "The `match` Control Flow Construct" and **6.3**
  "`if let` and `let...else`" (`resources - rust/book/src/ch06-02-match.md`,
  `ch06-03-if-let.md`).
- Rust book **7.1–7.5** (packages, crates, modules). Skim; the template already
  sets up `lib.rs` and modules, and you just need to recognize `mod`, `pub`, `use`.
- Rust book **11.1** "How to Write Tests" (skim).
- Optional refresher: Comprehensive Rust `pattern-matching/` (match guards,
  destructuring), and Rust book 3.2 if integer types (`u8`, `i8`, `usize`) feel rusty.

## Part 1: Name and create the sandbox

### Choose a name

The sandbox is a learning repo, separate from the real StereoMolGraph port (whose
name we'll pick at the start of stage 2). The package name must be a lowercase
Python identifier. Some options:

| name | note |
|---|---|
| **`protochem`** | what we went with: a prototype, clearly not the real package |
| `protochem` | Rust = iron oxide, "ferro" + chem |
| `smg_sandbox` | says exactly what it is |
| `rusty_rings` | a nod to ring closures in SMILES |

### Create it

From `~/code/sm/` (so it sits next to the other repos):

```sh
copier copy ~/code/sm/copier-pixi-rust protochem   # or gh:avcopan/copier-pixi-rust
```

Answers:
- `package_name`: your chosen name
- `description`: e.g. "Rust learning sandbox for StereoMolGraph"
- `github_org_name`: `avcopan`
- `prefix_channel`: anything (we won't publish), e.g. `avcopan`
- `polars_plugin`: **no** (keep the sandbox lean)

Skip the whole "Prepare Remote GitHub Repository" / publication section of the
template README. A GitHub remote is optional; pushing to a private repo is nice
for backup. Then:

```sh
cd protochem
pixi run init
```

Then move the tutorials into the sandbox:

```sh
mv ../tutorials ./tutorials
git add tutorials && git commit -m "Add initial tutorials"
```

(`PLAN.md` and `TODO.md` stay in `~/code/sm/`, outside the repo.)

### Tour the tooling (≈ 15 min)

Run each of these once and look at what it does:

```sh
pixi run -e dev cargo test     # Rust unit tests + doc tests (greet.rs has both)
pixi run test                  # Python tests: the compiled module, called from pytest
pixi run lint                  # clippy with the *pedantic* lint group, warnings = errors
pixi run fmt                   # rustfmt + ruff
```

Notes:
- **Clippy pedantic is your second tutor.** It's strict (e.g. `#[must_use]`, doc
  sections, casts), and its messages usually include a link explaining the
  idiom. Read them.
- **The pre-commit hooks run the tests.** Stub files with `todo!()` fail their
  tests, so either commit once the tests are green, or `pixi run hooks off` while
  you're mid-exercise (and `pixi run hooks on` afterwards).
- Read `src/lib.rs`, `src/greet.rs` and `src/python.rs`. That's the whole pattern:
  plain Rust in a module, re-exported from `lib.rs`, wrapped for Python in
  `python.rs`. [`WIRING.md`](WIRING.md) explains each piece line by line.

## Part 2: The exercise

1. Copy `tutorials/t01-setup-and-match/element.rs` to `src/element.rs`.
2. In `src/lib.rs` add `pub mod element;` (keep it `pub` so later modules and
   tests can use it as `crate::element::...`).
3. Make the tests pass: `pixi run -e dev cargo test element`.

The doc comments explain each function. Hints:

- **`symbol`:** one `match` with 18 arms plus `_ => None`. (Alternative to think
  about: a `const SYMBOLS: [&str; 18]` array indexed by `atomic_number - 1`. Which
  is better? Why does indexing need a guard but `match` doesn't?)
- **`atomic_number`:** `match symbol { "H" => Some(1), ... }` works on `&str`.
- **`organic_valences`:** match arms with `|`, returning `&[...]` literals. Notice
  that the return type `&'static [u8]` lets you return a reference to a literal
  without any allocation. That's the first appearance of lifetimes. You'll meet
  them properly in T03.
- **`implicit_hydrogens`:**

  ```rust
  let valences = organic_valences(atomic_number);
  // early return: `let ... else` needs a refutable pattern, e.g. a slice pattern
  let [_, ..] = valences else { return None };
  ```

  then loop to find the first valence `>= bond_order_sum`.
- Replace the `ignore` in the `symbol` doc example with a working doc test
  (doc tests are written from the crate user's point of view:
  `use protochem::element::symbol;`).

## Part 3: Expose one function to Python

In `src/python.rs`, wrap `implicit_hydrogens` as a `#[pyfunction]` (copy the
`greet` pattern, including the Google-style docstring), re-export it in
`python/<name>/__init__.py`, and add a pytest in `tests/test_<name>.py`:

```python
def test__implicit_hydrogens() -> None:
    assert protochem.implicit_hydrogens(6, 1) == 3
    assert protochem.implicit_hydrogens(26, 0) is None  # Option -> None
```

Run `pixi run test`. Notice how `Option<u8>` became `int | None` in the
generated stub (`pixi run stubs`, then look at `_core.pyi`).

## Part 4: Your own test

Write one more test of your own in `element.rs`: for example, check that
`implicit_hydrogens(z, 0)` equals the first organic valence for every organic
element. Then run `pixi run lint` and fix whatever clippy says.

## Done?

Say **"done"**. I'll review `src/element.rs`, `src/python.rs` and the tests. Tell
me how `match`, `Option` and the module system felt (comfortable / need more).
