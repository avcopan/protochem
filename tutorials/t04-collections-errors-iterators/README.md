# T04: Collections, Errors and Iterators: `MolGraph` v0

**Time:** ~3.5 h plus drills · **Builds:** `src/mol_graph.rs` · **Needs:** T03

## Why this matters

Every stage 2 algorithm runs on a graph: color refinement, VF2++, canonization,
bond-order assignment. This tutorial builds the graph they'll use, with the layout
stage 2 settled on (`PLAN.md` → "Preparing for bond-order assignment"): atoms and
bonds in `Vec`s, dense `AtomIndex`/`BondIndex` newtypes, and adjacency lists of
`(neighbor, bond)` pairs. That's petgraph's `Graph` layout. You'll also write the
first two graph algorithms, `bfs_layers` (VF2++ uses it for its matching order) and
`components`.

The main goal is fluency with Rust's functional side: `Option`/`Result`
combinators, iterator chains, and functions that return iterators. Graph code is
full of "look this up, it might not be there" and "for each atom, for each
neighbor". You'll write both as chains, and you'll learn when a plain loop is the
better choice. You know most of this from `itertools` and `more-itertools`. The
tables below map it over.

## How this tutorial works

Same as T03: write `src/mol_graph.rs` from scratch in small steps, each with a
check. Stop whenever the payoff drops and say **"fill in"**. `mol_graph.rs` in this
folder is Claude's spec. You don't need to open it.

**Drills** come from FEI lesson 04 (and one from 06), done in your
`rust-course-fei-work` fork (`cargo test --test <name>` from
`lessons/04/exercises/`). Each one appears at the step where it pays off.

| When | Drill | Why |
|---|---|---|
| after step 1 (optional) | FEI 06 `01_newtype_wrapper` | a `macro_rules!` that generates newtypes like `AtomIndex` |
| after step 3 | FEI 04 `08_range` | validating constructor → `Result`, `intersect` → `Option` |
| after step 3 | your FEI 02 `05_srl_validator` | rewrite `SRL::new` with combinators (see step 3) |
| before step 5 | FEI 04 `04_iter_exercises`, `07_adjacent_diff` | return `impl Iterator`; build chains from adapters |
| before step 8 | FEI 04 `03_fibonacci` | `impl Iterator` by hand, then as a `successors` one-liner |
| before step 9 (optional) | FEI 04 `10_ringbuffer` | what `VecDeque` does inside |

Not now: `05_split_items` is T03's `Lexer` again. `01_shape`, `02_case_insensitive_cmp`,
`06_cumulative_sum` and `09_poker_hand_value` are about traits and generic bounds,
so they go with T05.

## Reading (≈ 40 min, skim)

- Rust book **ch. 8** (`Vec`, `String`, `HashMap`). Skim it. The `entry` API in 8.3 is
  the part that's new.
- Rust book **ch. 9** (errors). 9.2 on `?` is the important part. You know
  `Result` from T02.
- Rust book **13.1–13.2** (closures, iterators). The `Fn`/`FnMut`/`FnOnce` part of
  13.1 explains step 8.
- Keep the method lists at the top of the [`Option`](https://doc.rust-lang.org/std/option/enum.Option.html),
  [`Result`](https://doc.rust-lang.org/std/result/enum.Result.html) and
  [`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html) docs open
  while you work.

---

## Step 1: Newtype indices (15 min)

**Write:** create `src/mol_graph.rs` and add `pub mod mol_graph;` to `lib.rs`. Then:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtomIndex(u32);

impl AtomIndex {
    pub fn new(index: usize) -> Self { ... }
    pub fn index(self) -> usize { ... }
}
```

and the same for `BondIndex`. `new` converts with `u32::try_from(index)` and
`expect`s. `index` is `self.0 as usize`. These two names are petgraph's
(`NodeIndex::new`, `.index()`).

**New Rust:**
- A **newtype** is a one-field tuple struct. It costs nothing at runtime (same size
  as a `u32`), but the compiler now keeps atom and bond indices apart.
  `graph.atomic_number(bond)` becomes a type error. Python's `NewType` does this for the
  type checker only.
- The field is private, so outside the module the only way in is `new`, and the
  only way out is `index`. That's C-NEWTYPE-HIDE. You did the same with the private
  fields of FEI 02's `SRL`.
- `u32`, not `usize`: half the memory in adjacency lists. `u32::try_from(usize)` returns
  a `Result` because the conversion can fail. `as` never fails, so it's fine for
  `u32` → `usize` (on 64-bit targets), but never use it to narrow.
- `fn index(self)`, not `&self`: the type is `Copy` and 4 bytes, so taking it by
  value is idiomatic (C-CONV, like `char::is_alphabetic(self)`).

**Check:** a test that `AtomIndex::new(3).index() == 3` and
`AtomIndex::new(1) < AtomIndex::new(2)`. After step 4, add a `compile_fail,E0308`
doc test on `AtomIndex` showing that `graph.atomic_number(BondIndex::new(0))` doesn't compile.

**Drill (optional):** FEI 06 `01_newtype_wrapper` writes a `macro_rules!` that
generates exactly this kind of type. `AtomIndex` and `BondIndex` are copy-paste
twins, which is what such a macro is for. The spec keeps them hand-written.

## Step 2: `MolGraph` and `add_atom` (15 min)

**Write:**

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MolGraph {
    atomic_numbers: Vec<u8>,
    bonds: Vec<[AtomIndex; 2]>,
    adjacency: Vec<Vec<(AtomIndex, BondIndex)>>,
}
```

with `new` (calls `Self::default()`), `add_atom(&mut self, atomic_number: u8) -> AtomIndex`
(pushes to `atomic_numbers` *and* an empty list to `adjacency`), `atom_count`,
`bond_count`, and

```rust
pub fn atomic_number(&self, atom: AtomIndex) -> Option<u8>
```

which is `self.atomic_numbers.get(atom.index()).copied()`.

**File order** (top-down, like the lexer): module docs → `MolGraph` and its `impl`
(constructors, `add_*`, lookups, iterators, algorithms) → `AtomIndex`, `BondIndex` →
`GraphError` → private helpers. Tests mirror that order.

**New Rust:**
- Indexing with `v[i]` panics when out of range. `v.get(i)` returns `Option<&T>`,
  and `.copied()` turns `Option<&u8>` into `Option<u8>`. The convention, used by
  petgraph and in std: lookups by index return `Option` (`node_weight`,
  `edge_endpoints`), and per-atom operations panic (`neighbors`), with a `# Panics`
  doc section.
- Clippy's pedantic `must_use_candidate` will ask for `#[must_use]` on pure
  functions like `atom_count`. Add it.

**Check:** two `add_atom` calls return `AtomIndex::new(0)` and `AtomIndex::new(1)`.
`atomic_number` gives `Some(8)` for an oxygen and `None` past the end.

## Step 3: `GraphError` and `add_bond` (40 min)

This is the combinator step. Read the table first and keep it open.

### `Option`/`Result` combinators

| Rust | Python equivalent | Notes |
|---|---|---|
| `opt.map(f)` | `f(x) if x is not None else None` | also on `Result` (maps `Ok`) |
| `opt.and_then(f)` | same, where `f` returns `Optional` | flattens; `?` inside a fn often reads better |
| `opt.filter(p)` | `x if x is not None and p(x) else None` | used in the lexer's `next_token` |
| `opt.unwrap_or(d)` | `x if x is not None else d` | `d` is evaluated even if unused |
| `opt.unwrap_or_else(\|\| d)`, `unwrap_or_default()` | (lazy versions) | |
| `opt.map_or(d, f)` | `f(x) if x is not None else d` | `map` + `unwrap_or` in one |
| `opt.ok_or(e)`, `ok_or_else(\|\| e)` | `if x is None: raise e` | `Option` → `Result`, then `?` |
| `res.ok()` | `try: ... except: None` | `Result` → `Option`, drops the error |
| `res.map_err(f)` | `except A as e: raise B(e) from e` | convert error types (T05) |
| `x?` | early `return None` / re-raise | works on `Option` too, in a fn returning `Option` |
| `opt.is_some_and(p)` | `x is not None and p(x)` | |
| `opt.copied()`, `opt.as_deref()` | – | `Option<&u8>` → `Option<u8>`; `Option<String>` → `Option<&str>` |
| `res.transpose()` | – | `Result<Option<T>>` ⇄ `Option<Result<T>>` (T03's `next`) |
| `cond.then_some(x)`, `cond.then(\|\| x)` | `x if cond else None` | on `bool` |

`ok_or` vs `ok_or_else`: use the lazy `_else` form when building the value costs
something (a `String`, a call). Clippy flags both mistakes (`or_fun_call`,
`unnecessary_lazy_evaluations`).

### Write

```rust
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GraphError {
    AtomOutOfRange(AtomIndex),
    SelfLoop(AtomIndex),
    DuplicateBond(BondIndex),   // the existing bond
}
```

with `Display` (`"atom 5 is out of range"`, `"atom 1 can't bond to itself"`,
`"bond 0 already joins these atoms"`) and `impl std::error::Error`. Unlike
`LexError`, there's no field every error shares (a position), so a plain enum is
enough.

Then, in this order:

1. **`find_bond(&self, a, b) -> Option<BondIndex>`.** Write it as a loop first: if `a`
   is out of range return `None`, else scan `a`'s `(neighbor, bond)` pairs and return
   the bond whose neighbor is `b`. Then rewrite it as one chain from three pieces:
   `self.adjacency.get(a.index())?`, `.iter().find_map(...)`, and
   `(neighbor == b).then_some(bond)` inside. `find_map` is Python's
   `next((f(x) for x in xs if ...), None)`.
2. **`bond_atoms(&self, bond) -> Option<[AtomIndex; 2]>`**, like `atomic_number`.
3. **`add_bond(&mut self, a, b) -> Result<BondIndex, GraphError>`:**
   - Range check: `for atom in [a, b] { self.atomic_number(atom).ok_or(GraphError::AtomOutOfRange(atom))?; }`.
     `ok_or` turns "missing" into an error, and `?` returns it.
   - Self-loop: a plain `if a == b { return Err(...) }`.
   - Duplicate: `if let Some(bond) = self.find_bond(a, b) { return Err(...) }`.
     The combinator version is `self.find_bond(a, b).map_or(Ok(()), |bond| Err(...))?`.
     Write it, compare, and keep the `if let`. **Rule of thumb:** use a combinator when
     the result is a *value* you go on using. Use `if let`/`match` when the point is
     *control flow* (an early return).
   - Then push `[a, b]` to `bonds` and `(b, bond)`/`(a, bond)` to the two adjacency
     lists.

**Check** (on a graph with C0 and O1):
- `add_bond(0, 1)` → `Ok(BondIndex::new(0))`
- `add_bond(0, 5)` and `add_bond(5, 0)` → `AtomOutOfRange(5)`
- `add_bond(1, 1)` → `SelfLoop(1)`
- `add_bond(0, 1)` again, and `add_bond(1, 0)` → `DuplicateBond(0)`
- `bond_count()` is still 1 afterwards
- `find_bond(9, 0)` → `None` (no panic), and the error messages match. Add the
  `assert_error::<GraphError>()` test from T03.

**Drills:**
- FEI 04 `08_range`. The constructor returns `Result`, and `intersect` returns
  `Option`. Try writing `intersect` with no `if`: `max` of the starts, `min` of the ends,
  then `Range1D::new(...).ok()`.
- Your FEI 02 `05_srl_validator`. Rewrite `SRL::new` in place (git has the old one)
  with combinators where they give values, e.g. `input.split_once("://")` →
  `map_or((None, input), |(p, a)| (Some(p), a))`. Keep `if let` for the early
  returns. Does it read better? Commit whichever you prefer.

## Step 4: `from_bonds`, a loop with `?` (10 min)

**Write:**

```rust
pub fn from_bonds(atomic_numbers: &[u8], bonds: &[[usize; 2]]) -> Result<Self, GraphError>
```

`add_atom` for each atomic number, then `add_bond` for each pair with `?`. Use plain
`for` loops (`for &[a, b] in bonds` destructures the array). Later tests use this to
build molecules in one line.

**Why not a chain?** `bonds.iter().try_for_each(|&[a, b]| graph.add_bond(...).map(|_| ()))`
works, but the loop says the same thing more plainly. A side effect plus `?` is what
`for` is for. A chain pays off when it *builds a value*. Compare `lex` in T03:
`collect()` into `Result<Vec<_>, _>` stops at the first `Err`, just like `?` in a loop.

**Check:** `from_bonds(&[6, 6], &[[0, 1], [1, 1], [0, 7]])` → `Err(SelfLoop(1))`.
Now add the `compile_fail,E0308` doc test from step 1.

## Step 5: Returning iterators (25 min)

**Drills first:** FEI 04 `04_iter_exercises` and `07_adjacent_diff`. Both return
`impl Iterator` built from adapters. `adjacent_diff` is `slice.windows(2).map(...)`,
which is `itertools.pairwise`.

**Write:**

```rust
pub fn neighbors(&self, atom: AtomIndex) -> impl Iterator<Item = AtomIndex> + '_
pub fn degree(&self, atom: AtomIndex) -> usize
pub fn atoms(&self) -> impl Iterator<Item = AtomIndex> + use<>
pub fn bonds(&self) -> impl Iterator<Item = BondIndex> + use<>
```

`neighbors` maps `|&(neighbor, _)| neighbor` over the adjacency list. `atoms` maps
`AtomIndex::new` over `0..self.atom_count()`.

**New Rust:**
- `impl Iterator<Item = AtomIndex>` means "some iterator type, I won't name it."
  The real type is something like `Map<slice::Iter<'_, (AtomIndex, BondIndex)>, {closure}>`.
  The Python equivalent is returning a generator, but here it costs nothing: no
  allocation and no boxing, and the whole chain is inlined.
- **What the iterator borrows.** `neighbors`' iterator reads `self.adjacency`, so it
  borrows the graph. In **edition 2021**, you had to say so with `+ '_`, or the
  compiler rejected it. That's why you'll see `+ '_` all over library code. In **edition
  2024** (ours), a returned `impl Trait` captures every lifetime in scope by default, so
  `+ '_` is optional. The spec keeps it on `neighbors` as documentation.
- The 2024 default has a cost. `atoms()` borrows nothing (the range is computed
  up front), but it would still be treated as borrowing `&self`, so you couldn't call
  `add_bond` while iterating. `+ use<>` ("captures nothing") opts out.

**Check:**
- ethanol with explicit H, as in the spec's test helper:
  `from_bonds(&[6, 6, 8, 1, 1, 1, 1, 1, 1], &[[0, 1], [1, 2], [0, 3], [0, 4], [0, 5], [1, 6], [1, 7], [2, 8]])`.
  `neighbors(1)` → `[0, 2, 6, 7]`, and degrees over `atoms()` → `[4, 4, 2, 1, 1, 1, 1, 1, 1]`.
- Write this test *without* `use<>` on `atoms` first and read the E0502. Then add it:

  ```rust
  let mut graph = MolGraph::from_bonds(&[6, 6, 6], &[]).unwrap();
  for atom in graph.atoms().skip(1) {
      graph.add_bond(AtomIndex::new(0), atom).unwrap();
  }
  assert_eq!(graph.degree(AtomIndex::new(0)), 2);
  ```

## Step 6: Nested loops → chains (20 min)

This is the exercise you asked for after T03's `symmetrize`. Write a test that checks
the adjacency lists agree with the bond list: each bond once, as `[lower, higher]`.

**Write it as a loop first:**

```rust
let mut from_adjacency = Vec::new();
for atom in graph.atoms() {
    for neighbor in graph.neighbors(atom) {
        if atom < neighbor {
            from_adjacency.push([atom, neighbor]);
        }
    }
}
```

**Then as a chain.** The recipe: the outer `for` becomes `flat_map`, and the inner
body becomes a chain on the inner iterator (`if` → `filter`, `push(x)` → `map` to
`x`), then `collect`. This is Python's
`[[a, n] for a in atoms for n in neighbors(a) if a < n]`, or
`chain.from_iterable(...)`. Leave out `move` first and read the error (E0373: the
inner closures borrow `atom`, which dies when the outer closure returns, while the
iterator they're part of lives on). `move` copies `atom` into them.

Then the other side: `graph.bonds().filter_map(|bond| graph.bond_atoms(bond))`, sort
each pair, sort both lists, `assert_eq!`. Run it on ethanol and a cyclohexane ring.

**When to keep the loop.** Use a chain when you're *building a value* (a `Vec`, a
count, a `bool`, a `String`) from one pass with no side effects. Keep the loop when:
- the body returns early or uses `?` for the *enclosing* function (step 4);
- the body updates two or more things (step 9's queue and component);
- the next iteration depends on what this one did, beyond a simple `fold`;
- the chain needs closures inside closures inside closures. Two levels, as here, is fine.

### `itertools`/`more-itertools` → Rust

std covers most of it. The rest is in the `itertools` crate (T08 adds the first
dependency, so for now just know where things are).

| Python | Rust std | `itertools` crate |
|---|---|---|
| `enumerate`, `zip`, `map`, `filter`, `any`, `all`, `sum` | same names | |
| comprehension with `if` + transform | `filter_map` | |
| `chain.from_iterable`, nested comprehension | `flatten`, `flat_map` | |
| `next((x for x in xs if p(x)), None)` | `find`, `find_map`, `position` | |
| `functools.reduce` | `fold`, `reduce` | |
| `accumulate` | `scan` | |
| `islice`, `takewhile`, `dropwhile` | `skip`/`take`/`step_by`, `take_while`, `skip_while` | |
| `pairwise`, `batched` | `slice.windows(2)`, `slice.chunks(n)` | `tuple_windows`, `chunks` |
| `count()`, `repeat` | `0..`, `iter::repeat` | |
| `more_itertools.iterate` | `iter::successors` (step 8) | `iterate` |
| `max(xs, key=f)` | `max_by_key` | |
| `more_itertools.partition` | `partition` | `partition_map` |
| `Counter` | `HashMap` + `entry` (step 7) | `counts` |
| `groupby` | – | `chunk_by` |
| `product`, `combinations` | – | `cartesian_product`, `combinations`, `tuple_combinations` |
| `unique_everseen` | – | `unique` |
| `sorted` | `collect` + `sort` | `sorted` |
| `more_itertools.one` | – | `exactly_one` |

## Step 7: `formula`: `HashMap` (20 min)

**Write:** `pub fn formula(&self) -> String` in Hill order: carbon first, then
hydrogen, then the rest alphabetically. With no carbon, everything is
alphabetical. Count 1 is left out (`CH4`, not `C1H4`).

1. Count symbols: `let mut counts: HashMap<&str, usize> = HashMap::new();` and for
   each atom `*counts.entry(symbol).or_default() += 1;`. That's
   `Counter`/`defaultdict(int)`. A private `fn symbol(atomic_number: u8) -> &'static str`
   uses `Element::from_atomic_number(z).map_or("*", |element| element.symbol)`. The
   wildcard `0` isn't an element.
2. Sort: collect `counts` into a `Vec<(&str, usize)>`, and `sort_unstable_by_key` with
   a tuple key like Python's `sorted(key=lambda ...)`. A private
   `fn hill_key(symbol, has_carbon) -> (u8, &str)` gives `(0, "C")`, `(1, "H")`, or
   `(2, symbol)`. `match` with guards (`"C" if has_carbon =>`) reads well.
3. Join: `.map(|(symbol, count)| ...)` to `String`s, then `.collect::<String>()`.
   That's `"".join(...)`.

**New Rust:**
- `entry(key)` looks the key up once and gives you a slot to fill or update.
  `or_default()` inserts `0` if it's missing and returns `&mut usize`, hence the `*`.
- A `HashMap` iterates in **random order**, different on every run. Python dicts keep
  insertion order, but Rust's maps don't. That's why the sort is needed. A `BTreeMap`
  iterates in key order instead. T07 is about this.

**Check:** `[]` → `""`, `CH4`, ethanol → `C2H6O`, water → `H2O`, NaCl → `ClNa`,
`[17, 6, 1, 1, 1, 35]` → `CH3BrCl`, `[0, 6]` → `C*`.

## Step 8: `bfs_layers`: closures with state (25 min)

**Drill first:** FEI 04 `03_fibonacci`. Implement `Iterator` by hand (a struct with
`next`), as the exercise asks. Then write it again as one expression with
`std::iter::successors(Some((0u64, 1)), |&(a, b)| Some((b, a + b))).map(|(a, _)| a)`.
That's `more_itertools.iterate`.

**Write:**

```rust
pub fn bfs_layers(&self, start: AtomIndex) -> Vec<Vec<AtomIndex>>
```

Layer 0 is `[start]`, layer `k + 1` is the unvisited neighbors of layer `k`, and it stops
at the first empty layer.

1. A private `struct Visited(Vec<bool>)` with `new(atom_count)` and
   `fn insert(&mut self, atom) -> bool`, which returns `true` if the atom was new,
   like `HashSet::insert`. (Python: `if n not in seen: seen.add(n)`.) One line:
   `!std::mem::replace(&mut self.0[atom.index()], true)`. A `Vec<bool>` beats a
   `HashSet` when indices are dense.
2. Next layer from the current one:
   `layer.iter().flat_map(|&atom| self.neighbors(atom)).filter(|&n| visited.insert(n)).collect()`.
3. Write the outer loop as a `loop` that pushes layers until one is empty. Then
   replace it with `successors(Some(vec![start]), |layer| { ...; (!next.is_empty()).then_some(next) }).collect()`.
   Keep whichever you find clearer. The spec uses `successors`.

**New Rust:** `filter(|&n| visited.insert(n))` *mutates* `visited`. That's legal
because `filter` takes an `FnMut`: a closure that may change what it captures. The
`successors` closure captures `visited` by `&mut` too, so it's `FnMut` as well.
(Book 13.1 covers `Fn`/`FnMut`/`FnOnce`.) The filter now depends on order, which is
fine here because BFS order is the point. Python's equivalent hack,
`[n for n in xs if n not in seen and not seen.add(n)]`, is frowned on. In Rust it's
normal.

**Check:**
- cyclohexane (`[6; 6]`, ring bonds `[0,1] … [5,0]`) from 0 →
  `[[0], [1, 5], [2, 4], [3]]`
- ethanol from 8 → `[[8], [2], [1], [0, 6, 7], [3, 4, 5]]`
- two unbonded atoms, from 1 → `[[1]]`

## Step 9: `components`: `VecDeque`, keeping the loop (20 min)

**Write:** `pub fn components(&self) -> Vec<Vec<AtomIndex>>`. For each atom not yet
visited, run a BFS from it with a queue, and collect the atoms reached into one
component. One `Visited` is shared by all the searches.

```rust
let mut queue = VecDeque::from([root]);
while let Some(atom) = queue.pop_front() { ... queue.push_back(neighbor); ... }
```

`VecDeque` is `collections.deque` (`popleft`/`append`). `while let` loops as long as
the pattern matches.

**Try a chain first.** `self.atoms().filter(|&root| visited.insert(root)).map(|root| ...)`.
The `map` closure needs `visited` for its BFS too, and you get E0499: two closures
can't both hold `&mut visited`. You could squeeze everything into one `filter_map`,
but the inner BFS stays a loop anyway, because its queue grows while it runs. This
is the "body updates two or more things" case from step 6. Keep the loops.

**Check:** components are ordered by lowest atom, each in BFS order from it:
- `[6, 6, 8]` with `[[0, 1]]` → `[[0, 1], [2]]`
- `[6, 8, 6]` with `[[0, 2]]` → `[[0, 2], [1]]`
- NaCl, no bonds → `[[0], [1]]`
- four carbons with `[[3, 1], [1, 2], [2, 0]]` → `[[0, 2, 1, 3]]`
- empty graph → `[]`

## Wrap-up: docs and lint (15 min)

Module docs, a summary line on every public item, `# Errors` on `add_bond`/`from_bonds`,
`# Panics` on everything that indexes or `expect`s, and one `# Examples` on `MolGraph`
(ethanol's heavy atoms; `neighbors` and `formula`). Optionally re-export `MolGraph` at
the crate root, like `Element`. `pixi run lint` clean, `cargo test` green.

## API guidelines to check at the end

- **C-NEWTYPE / C-NEWTYPE-HIDE**: indices are newtypes with private fields. Could
  outside code still forge an `AtomIndex` that's out of range? (Yes, with `new`. That's
  why lookups return `Option`.)
- **C-COMMON-TRAITS**: `MolGraph` derives `Default`, so `new` is just `Self::default()`.
  Should `MolGraph` be `Hash`? (Not for now. Graph hashing is T07's topic, and derived
  `Eq` compares atom order, not structure.)
- **C-GOOD-ERR**: `GraphError` is `'static + Send + Sync` and has a lowercase,
  unpunctuated `Display`.
- **C-FAILURE**: every function that can panic says when, under `# Panics`.
- **C-ITER**: `atoms`/`bonds`/`neighbors` follow petgraph's names (`node_indices`,
  `edge_indices`, `neighbors`). Their siblings `atomic_number`/`bond_atoms` follow
  `node_weight`/`edge_endpoints`, and `find_bond` follows `find_edge`.

## Done?

Say **"done"** for a review, or **"fill in"** to have Claude finish the rest. In the
review I'll look at where you chose combinators vs. `if let`, your step 6 chain, and
`bfs_layers`. Tell me which loop-to-chain conversions felt natural and which felt
forced.
