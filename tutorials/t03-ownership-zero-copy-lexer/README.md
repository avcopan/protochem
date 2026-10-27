# T03: Ownership, Borrowing and Lifetimes: a Zero-Copy Lexer

**Time:** ~3 h · **Builds:** `examples/borrow_puzzles.rs`, then a refactor of
`src/smiles_lexer.rs` · **Needs:** T02's lexer

## Why this matters

Ownership is the one Rust idea with no Python counterpart, and the borrow checker
will be part of every stage 2 algorithm. VF2++ mutates its state while reading the
graph. Color refinement reads the old coloring while writing the new one. The
SMILES parser wants tokens that point back into the input. Most stage 2 compile
errors will be some form of "cannot borrow `x` as mutable because it is also
borrowed as immutable". This tutorial makes that error familiar, along with the
three or four standard ways out.

Part A is puzzles on a toy graph. You read the compiler error, then fix it. Part B
uses lifetimes for real. The T02 lexer copies its input into a `Vec<char>` and
builds symbols with `format!`. You make it borrow the input instead. Each token
then comes with the slice of SMILES it was read from (`"[13CH4]"`, `"%12"`). The
T05 parser uses those slices for error messages.

## How this tutorial works

Same as T02: small steps, each with a check. Stop whenever the payoff drops and
say **"fill in"**. `borrow_puzzles.rs` and `smiles_lexer.rs` in this folder are
Claude's spec. You don't need to open them.

**Seeing compiler errors through pixi.** After a source change, `pixi run` first
rebuilds the project's conda package. A compile error then shows up as "failed to
build 'protochem'", with rustc's message buried in a long build log. While you're
working through errors, skip the rebuild with `--no-install`:

```sh
pixi run --no-install -e dev cargo run --example borrow_puzzles
pixi run --no-install -e dev cargo test smiles_lexer
```

(rust-analyzer's diagnostics in Zed show the same errors.)

## Reading (≈ 40 min)

- Rust book **ch. 4** (ownership, references, slices). This is the core of the
  tutorial, so read all of it.
- Rust book **10.3** "Validating References with Lifetimes". Skip "Generic Type
  Parameters, Trait Bounds, and Lifetimes Together" at the end.
- Optional warm-ups: FEI lesson 03's `tests/01_longest.rs`, `02_strip_prefix.rs`
  and `06_zerocopy_parsing.rs` (in `resources - rust/rust-course-fei/lessons/03/exercises/`).

---

## Part A: Borrow-checker puzzles (~45 min)

Create `examples/borrow_puzzles.rs` with this toy graph and an empty `main`:

```rust
//! T03 borrow-checker puzzles.

/// A toy graph: `adjacency[i]` lists the neighbors of node `i`.
#[derive(Debug, Clone)]
struct Graph {
    adjacency: Vec<Vec<usize>>,
}

fn main() {}
```

For each puzzle, type in the broken code and **predict the error before you
compile**. Then read the full message (the arrows matter) and fix it. Keep the
fixed version. The error code (`E0382`) opens a longer explanation with
`rustc --explain E0382`.

### A1: Moves vs. copies (E0382)

```rust
fn degree_sum(graph: Graph) -> usize {
    graph.adjacency.iter().map(Vec::len).sum()
}

// in main:
let graph = Graph { adjacency: vec![vec![1], vec![0, 2], vec![1]] };
let total = degree_sum(graph);
println!("degree sum {total} over {} nodes", graph.adjacency.len());
```

Passing `graph` by value **moves** it into `degree_sum`, and `main` can't use it
afterwards. Fix it by borrowing: `graph: &Graph` and `degree_sum(&graph)`.

Then write `fn double(n: usize) -> usize` and call it twice on the same `n`. No
error: `usize` is `Copy`, so passing it by value copies it. Python never asks you
this question because everything is a shared reference. In Rust, every
parameter is either owned or borrowed.

### A2: `&` vs `&mut` (E0502)

```rust
fn add_edge(graph: &mut Graph, i: usize, j: usize) {
    graph.adjacency[i].push(j);
    graph.adjacency[j].push(i);
}

// in main (make `graph` mutable: `let mut graph = graph;`):
let first = &graph.adjacency[0];
add_edge(&mut graph, 0, 2);
println!("{first:?}");
```

The rule: **many `&` or one `&mut`, never both at once.** `first` is a shared
borrow that's still alive when `add_edge` asks for an exclusive one. That's not
fussiness. `push` can reallocate `adjacency[0]` and leave `first` dangling.

Fix it two ways and keep one: move the `println!` up, or copy out what you need
before the mutation (`let first_degree = graph.adjacency[0].len();`). Note that
borrows end at their **last use**, not at the end of the scope.

### A3: Mutate while iterating (E0502, the big one)

Make a directed graph undirected: for every edge `i -> j`, add `j -> i` if it's
missing.

```rust
fn symmetrize(graph: &mut Graph) {
    for i in 0..graph.adjacency.len() {
        for &j in &graph.adjacency[i] {
            if !graph.adjacency[j].contains(&i) {
                graph.adjacency[j].push(i);
            }
        }
    }
}
```

The `for` loop holds a borrow of `graph.adjacency` for its whole body, and the
body tries to mutate it. In Python this is the "list changed size during
iteration" bug, found at runtime. Here it's found at compile time.

Write three fixes as three functions, `symmetrize_by_cloning`,
`symmetrize_by_index` and `symmetrize_by_taking`. Check that all three give
`[[1, 2], [0, 2], [1, 0]]` for `vec![vec![1, 2], vec![], vec![1]]`.

1. **Clone first:** `let neighbors = graph.adjacency[i].clone();`, then loop over
   it. It's simple and costs a copy, which is the usual first fix.
2. **Index:** `for k in 0..graph.adjacency[i].len() { let j = graph.adjacency[i][k]; ... }`.
   Each `graph.adjacency[i][k]` is a short borrow that ends right away. The
   pattern is common in graph code.
3. **Take it out:** `let neighbors = std::mem::take(&mut graph.adjacency[i]);`
   leaves an empty `Vec` behind. Loop over `neighbors`, then put it back. There's no
   copy, but while the list is out, node `i` looks like it has no neighbors.
   Try a self-loop `i -> i`. Putting the list back with
   `graph.adjacency[i] = neighbors;` still gets it right. Why? What goes wrong if
   you put it back with `graph.adjacency[i].extend(neighbors)` instead? Any code
   that reads `adjacency[i]` while the list is out has the same problem.

**New Rust:** `for symmetrize in [f, g, h]` loops over an array of functions
with the same signature. That's handy for running all three on clones
of one graph.

### A4: Returning a reference to a local (E0515)

```rust
fn hub_name(graph: &Graph) -> &str {
    let name = format!("atom {}", graph.adjacency.len());
    &name
}
```

`name` is dropped when the function returns, so a reference to it would dangle.
Return the `String` itself: an owned value moves out to the caller. (Python's
garbage collector hides this choice too.)

Compare: `fn graph_kind(graph: &Graph) -> &'static str` returning `"empty"` or
`"molecular"` is fine. String literals live in the binary for the whole program,
which is what `'static` means.

### A5: `&str` vs `String` parameters

```rust
fn label(name: &str) -> String {
    format!("<{name}>")
}
```

Call it with a literal `"C"`, with `&owned` where `let owned = String::from("Cl")`,
and with a slice `&owned[..1]`. All three work: `&String` coerces to `&str`
automatically. The rule of thumb: **take `&str`, return `String`** (when the
result is new). Taking `String` would force callers with a `&str` to allocate a
copy just to call you.

### A6: Lifetime parameters (E0106)

```rust
fn longer(a: &str, b: &str) -> &str {
    if a.len() >= b.len() { a } else { b }
}
```

The result borrows from `a` or from `b`, and the compiler won't guess which. Write
it out: `fn longer<'a>(a: &'a str, b: &'a str) -> &'a str`. This says "the result
lives no longer than the shorter-lived of the two." A lifetime parameter changes
nothing at runtime. It's a constraint the compiler checks at each call site.

Then break it from the caller's side (E0597):

```rust
let result;
{
    let bromine = String::from("Br");
    result = longer(&bromine, "C");
}
println!("{result}");
```

Why did A4 not need a lifetime parameter? With exactly one reference input,
**elision** ties the output to it automatically. Part B hinges on when elision
picks the wrong one.

**Check:** `cargo run --example borrow_puzzles` prints all the results, and
`pixi run lint` is clean (examples are linted too).

---

## Part B: A zero-copy lexer (~1.5 h)

You'll refactor `src/smiles_lexer.rs` in place. Every T02 test must keep passing
after every step. That's your safety net.

### Step 1: A `Scanner` that borrows (30 min)

**Write:** change `Scanner` to borrow its input instead of copying it:

```rust
struct Scanner<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> Scanner<'a> {
    fn new(input: &'a str) -> Self { ... }
    fn peek(&self) -> Option<u8> { ... }   // was Option<char>
    ...
}
```

- `peek` reads bytes: `self.input.as_bytes().get(self.cursor).copied()`.
- `pop`, `take`, `take_digit`, `take_if` and `digit` switch from `char` to `u8`.
  Match arms become byte literals: `b'C'`, `b'0'..=b'9'`, `b'\\'`. `u8` has the
  same `is_ascii_*` methods as `char`. A digit's value is `c - b'0'`.
- Functions that take the scanner gain an anonymous lifetime:
  `fn next_token(scanner: &mut Scanner<'_>) -> ...`. Plain `&mut Scanner` would
  still compile thanks to elision, but it hides that `Scanner` borrows something,
  so the template's `rust_2018_idioms` lint (`elided_lifetimes_in_paths`) warns
  on it. Write the `'_`.

**New Rust:**
- `Scanner<'a>` reads "a scanner that borrows something for `'a`." The struct
  can't outlive the string it points to, and the compiler enforces that wherever a
  `Scanner` is created.
- `impl<'a> Scanner<'a>` declares the lifetime before using it, just like
  `impl<T> Vec<T>`.
- Why bytes? Every SMILES character is ASCII, and indexing a `&str` by bytes is
  O(1). Indexing by `char` would mean decoding UTF-8 from the start. Positions in
  `LexError` become **byte offsets**. For anything that lexes, they equal character
  offsets.

**The catch:** `UnexpectedCharacter` needs a `char`, but a non-ASCII character
like `é` is two bytes. Add `fn char_at(&self, position: usize) -> Option<char>`
that decodes it from `&self.input[position..]` with `.chars().next()`. Slicing a
`&str` panics in the middle of a character. Why can't that happen here? (The
cursor only ever steps over bytes that matched an ASCII pattern.)

**Check:** all T02 tests pass. Add a test: `"Cé"` gives
`UnexpectedCharacter { position: 1, character: 'é' }`.

### Step 2: Slices that outlive the borrow (25 min)

**Write:** `fn slice(&self, start: usize) -> &str` on `Scanner`, returning
`&self.input[start..self.cursor]`. Use it to drop the allocations:

- Organic atoms: record `let start = scanner.cursor();` before `pop`, and pass
  `scanner.slice(start)` (e.g. `"Cl"` after `take(b'l')`) to `organic_atom`.
- Bracket symbols: take the letters with `take_if`, then
  `let symbol = scanner.slice(position);`. No `format!`.
- Aromatic symbols need capitalizing, which a slice can't do. Write
  `fn aromatic_symbol(symbol: &str) -> Option<&'static str>` that maps
  `"c"` to `Some("C")` and so on. It replaces `to_ascii_uppercase` *and* the
  `matches!` that set `aromatic`.
- `UnknownElement { symbol }` keeps a `String`: write `symbol.to_owned()`. This is
  the one copy left, and it's deliberate. An error that borrows the input can't be
  returned past the input's owner, or boxed into `Box<dyn Error>` (which needs
  `'static`). Errors own their data.

Leave `slice` with the elided lifetime for now. It compiles. Step 4 will show
why it's wrong.

**New Rust:** in `aromatic_symbol`, `'static` overrides elision. Without it, the
compiler would assume the output borrows `symbol`, and callers would be stuck
keeping `symbol` alive for no reason.

**Check:** all tests pass. `grep -n "format!\|to_string" src/smiles_lexer.rs` only
finds the tests.

### Step 3: `Lexeme<'a>` (10 min)

**Write:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lexeme<'a> {
    pub token: Token,
    pub text: &'a str,
    pub position: usize,
}
```

**New Rust:** a struct holding a reference needs a lifetime parameter, and so
does everything that holds the struct. `Copy` is fine: copying a `&str` copies a
pointer and a length, not the text.

### Step 4: A `Lexer` iterator (30 min)

**Write:**

```rust
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    scanner: Scanner<'a>,
    failed: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(smiles: &'a str) -> Self { ... }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Lexeme<'a>, LexError>;

    fn next(&mut self) -> Option<Self::Item> { ... }
}
```

`next` records the position, calls `next_token`, and wraps the token in a
`Lexeme` with `text: self.scanner.slice(position)`. After an error it sets
`failed` and returns `None` from then on, so a caller can't lex past garbage.
Then rewrite `lex` on top of it:

```rust
Lexer::new(smiles).map(|lexeme| lexeme.map(|lexeme| lexeme.token)).collect()
```

`collect` into `Result<Vec<_>, _>` stops at the first `Err`, like a loop with `?`.

**Now compile.** The elided `slice` from step 2 fails with "lifetime may not live
long enough". Read the message: `'1` is the lifetime of `&mut self` in `next`, and
`'a` is the input's. Elision tied `slice`'s output to `&self`, meaning the
*scanner*. But `Item` promises lexemes that borrow the *input*. Fix the signature:

```rust
fn slice(&self, start: usize) -> &'a str
```

This is the key idea of the tutorial. **A lifetime says what a reference borrows
from, not how long a value lives.** The scanner is a short-lived window onto the
input, and the slices it hands out belong to the input.

**Check:**
- All T02 tests pass through the new `lex`.
- `Lexer::new("C[13CH4]%12Cl")` gives texts `["C", "[13CH4]", "%12", "Cl"]` at
  positions `[0, 1, 8, 11]`.
- `Lexer::new("CXC")` gives `Ok`, then `Err`, then `None`, `None`.
- Lexemes outlive the lexer: create `smiles`, collect lexemes inside a block that
  owns the `Lexer`, and use them after the block.
- Lexemes can't outlive the input. Write this as a doc test on `Lexer` marked
  ` ```compile_fail,E0597 `. Rustdoc then checks that it *doesn't* compile:

  ```rust
  let lexemes: Vec<_> = {
      let smiles = String::from("CCO");
      Lexer::new(&smiles).collect()
  };
  println!("{lexemes:?}");
  ```

**New Rust:** `impl FusedIterator for Lexer<'_> {}` promises that `None` stays
`None`. `'_` means "some lifetime, I don't need its name."

## Stretch (optional, ~20 min)

Store `bytes: &'a [u8]` instead of `input: &'a str`. What do you need to get the
`&str` back in `slice` (`std::str::from_utf8`), and why is that safe to `expect`
here? Is it worth it? (The spec keeps `&str`.)

## API guidelines to check at the end

- **C-COMMON-TRAITS**: `Lexeme` is `Copy` even though it holds a reference. Why
  is that fine? Should `Lexer` be `Clone`? (Cloning gives a second cursor over the
  same input, which is cheap and sometimes handy for lookahead.)
- **C-ITER-TY**: `Lexer` is named for what it is, like `str::Chars`. `lex`
  stays as a convenience.
- **C-GOOD-ERR**: `LexError` still owns its data, so it's `'static + Send +
  Sync`. Check it: `fn assert_error<E: std::error::Error + Send + Sync + 'static>() {}`.
- **C-CALLER-CONTROL / C-GENERIC**: `Lexer::new` takes `&str`, not `String`.
  Callers keep their string, and the lexer needs nothing more.
- **C-INTERMEDIATE**: `Lexeme::position` and `text` hand out what the lexer
  computes anyway.

## Done?

Say **"done"** for a review, or **"fill in"** to have Claude finish the rest. In
the review I'll look at your puzzle fixes, the `slice` signature, and how `next`
handles the error state. Tell me which puzzles felt natural and which felt
arbitrary. The arbitrary ones are worth a follow-up before T04.
