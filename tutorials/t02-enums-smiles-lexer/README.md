# T02: Enums, Structs and Errors: a SMILES Lexer

**Time:** ~2 h · **Builds:** `src/smiles_lexer.rs` · **Needs:** T01's `element.rs`

## Why this matters

Enums with data are Rust's most important modeling tool. They'll describe stereo
parities, bond changes (`Formed`/`Broken`/`Fleeting`), LS kinds, search frames in
VF2++, and errors everywhere. A lexer is the perfect place to practice them: every
token is a variant, and lexing is a big `match`. It's also the first stage of the
SMILES reader.

A lexer turns a string into a flat list of tokens. It doesn't check that the
tokens make sense together; that's the parser's job, in T05. For example, `"C)("`
lexes fine but won't parse.

## How this tutorial works

You write `src/smiles_lexer.rs` from scratch, one step at a time. Each step adds
one kind of token, introduces the Rust it needs, and ends with a check: inputs
and the tokens they should produce. Write a test for each check before moving on.

Stop whenever the remaining steps stop teaching you much, and say **"fill in"**.
Claude then finishes the rest: remaining steps, doc comments, tests, and a
lint pass. `smiles_lexer.rs` in this folder is Claude's spec for that (types, docs
and the full test list). You don't need to open it.

Run the tests with `pixi run -e dev cargo test smiles_lexer`.

## Reading (≈ 30 min)

- Rust book **6.1** (enums with data) and **9.2** "Recoverable Errors with
  `Result`" (the `?` operator). **5.1–5.3** (structs and `impl` blocks) if you
  haven't seen them yet: you write a small struct in step 2.
- Rust book **19.3** "Pattern Syntax", as a reference to skim.
- Optional: how `purr` does it, in `resources - rust-graph-and-cheminformatics-info/purr/src/read/`
  (`scanner.rs`, `read_organic.rs`, `read_bracket.rs`).

## Step 0: Wire up the module (2 min)

Create `src/smiles_lexer.rs` with a one-line module doc (`//! A lexer for a subset
of SMILES.`), and add `pub mod smiles_lexer;` to `lib.rs`.

## Step 1: Bonds, branches and dots (15 min)

**Write:**
- `pub enum BondKind` with one variant per bond symbol: `-` `=` `#` `$` `:` `/` `\`
  (`Single`, `Double`, `Triple`, `Quadruple`, `Aromatic`, `Up`, `Down`).
- `pub enum Token` with `Bond(BondKind)`, `BranchOpen`, `BranchClose` and `Dot`.
- `pub fn lex(smiles: &str) -> Vec<Token>`, which loops over `smiles.chars()` and
  matches each character to a token. For now, `panic!` on anything else.

**New Rust:** an enum variant can carry data (`Bond(BondKind)`), like a tagged
union. Derives give types the standard traits:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BondKind { ... }
```

`Debug` is needed to print the type, and `PartialEq` to use `assert_eq!`. Give
`Token` the same derives without `Copy`: it will hold a `BracketAtom` struct in
step 5, which you might not want copied implicitly.

**Check:** `"(=)"` gives `[BranchOpen, Bond(Double), BranchClose]`.
`"/\\."` gives `[Bond(Up), Bond(Down), Dot]` (`\\` is one backslash in a Rust string).

## Step 2: A `Scanner` and the error type (25 min)

Some tokens need lookahead (`Cl`, `%12`, `@@`), and a `for c in chars()` loop can't
peek. Replace it with a small cursor struct.

**Write:**
- `struct Scanner` with fields `characters: Vec<char>` and `cursor: usize`, and an
  `impl Scanner` block with:
  - `new(input: &str) -> Self`
  - `peek(&self) -> Option<char>`: the next character, without consuming it.
  - `pop(&mut self) -> Option<char>`: consume and return the next character.
  - `cursor(&self) -> usize`
- `pub enum LexError` with one variant for now:
  `UnexpectedCharacter { position: usize, character: char }`.
- `fn next_token(scanner: &mut Scanner) -> Result<Option<Token>, LexError>`. It
  returns `Ok(None)` at the end of the input, `Ok(Some(token))` for a token, and
  `Err(...)` for a bad character. Move step 1's `match` here.
- Change `lex` to return `Result<Vec<Token>, LexError>` and loop with
  `while let Some(token) = next_token(&mut scanner)? { ... }`.

**New Rust:**
- `&self` vs `&mut self`: `peek` only reads, while `pop` moves the cursor.
- `self.characters.get(i)` returns `Option<&char>`. Add `.copied()` to get
  `Option<char>`.
- `?` returns early from the function on `Err`, and on `None` in a function that
  returns `Option`. `pop` can be written with it: `let c = self.peek()?;`.
- `let ... else` pulls the character out of an `Option`, or returns:
  `let Some(c) = scanner.pop() else { return Ok(None); };`
- The `?` in `lex`'s loop unwraps `Result<Option<Token>>` to `Option<Token>`, and
  `while let Some(token)` stops at `None`.

**Check:** step 1's tests still pass with `Ok(vec![...])`. `"(X"` gives
`Err(UnexpectedCharacter { position: 1, character: 'X' })`, where `position` is
the index of the bad character. `""` gives `Ok(vec![])`.

## Step 3: Organic atoms (20 min)

**Write:**
- A `Token::Atom { element: u8, aromatic: bool }` variant (a struct-like variant).
- Match arms in `next_token` for the organic subset `B C N O P S F Cl Br I` and the
  aromatic `b c n o p s`. Look up atomic numbers with `Element::from_symbol` from T01.
- A `take(&mut self, expected: char) -> bool` method on `Scanner`. It consumes the
  next character only if it equals `expected`, and reports whether it did. Use it
  for the second letter of `Cl` and `Br`.

**New Rust:**
- Or-patterns: `'B' | 'N' | 'O' => ...`.
- Arm order matters: put `'C'` (which checks for `Cl`) before the general arm.
- `Element::from_symbol` takes `&str`. Build the string with `c.to_string()`, or
  with `format!("{c}l")` for `Cl`. For aromatic symbols, capitalize first:
  `c.to_ascii_uppercase()`.
- `Element::from_symbol` returns `Option`. For organic-subset letters, it can't
  fail. Write `.expect("organic subset element")` there, rather than inventing an
  error that can never happen.

**Check:** `"CCO"` gives three carbon/oxygen atoms. `"ClCBr"` gives Cl, C, Br.
`"c1"` comes in the next step, but `"cc"` already gives two aromatic carbons.

## Step 4: Ring closures (20 min)

**Write:**
- `Token::RingClosure(u8)`.
- A digit arm: `'0'` through `'9'` become `RingClosure(0)` through `RingClosure(9)`.
- A `%` arm: exactly two digits follow (`%12`). If the input ends after `%` or
  `%1`, that's a new error variant: `LexError::UnexpectedEnd`.
- A `take_number(&mut self) -> Option<u32>` method on `Scanner`. It consumes a run
  of digits and returns its value, or `None` if no digit is next. Step 5 uses it
  too. (For `%nn`, it's simplest to pop exactly two digits, so `%123` is `%12`
  followed by `3`.)

**New Rust:**
- Ranges and `@` bindings: `d @ '0'..='9' => ...`.
- `c.to_digit(10)` returns `Option<u32>`.
- Integer conversions: tokens store `u8`, and digits come out as `u32`. Use
  `u8::try_from(x)`, not `x as u8` (Clippy pedantic flags lossy casts). Where a
  value can't overflow (a single digit), `.expect(...)` is fine.
- `Option` → `Result`: `scanner.pop().ok_or(LexError::UnexpectedEnd)?` is the
  idiom for "running out of input is an error here."

**Check:** `"C1CC1"` and `"C%12C%12"` give ring closures 1 and 12. `"C%1"` gives
`Err(UnexpectedEnd)`. `"c1ccccc1"` gives 8 tokens.

## Step 5: Bracket atoms (40 min)

This is the biggest step. A bracket atom has the grammar
`'[' isotope? symbol chirality? hcount? charge? class? ']'`, as in `[13C@@H2+2:7]`.

**Write:**
- `pub enum Chirality { CounterClockwise, Clockwise }` (`@` and `@@`).
- `pub struct BracketAtom` with public fields: `isotope: Option<u16>`,
  `element: u8`, `aromatic: bool`, `chirality: Option<Chirality>`,
  `hydrogens: u8`, `charge: i8`, `class: Option<u16>`.
- `Token::BracketAtom(BracketAtom)`, and a `'['` arm that calls
  `fn read_bracket_atom(scanner: &mut Scanner) -> Result<BracketAtom, LexError>`.
- A new error: `LexError::UnknownElement { position: usize }`, where `position` is
  where the symbol starts.

Build it up in this order, adding a test after each:

1. **Symbol and `]`**: `[Fe]`, `[C]`, `[n]`. The symbol is an uppercase letter
   plus an optional lowercase one, or one of the aromatic `b c n o p s`. `[Xx]`
   gives `UnknownElement { position: 1 }`, and `[C` gives `UnexpectedEnd`.
2. **Isotope**: `[13C]`. Use `take_number`, then `try_from`.
3. **Hydrogens**: `[CH4]`, `[CH]` (no number means 1), `[nH]`.
4. **Chirality**: `[C@H]`, `[C@@H]`. Use `take('@')` twice.
5. **Charge**: `[O-]`, `[NH4+]`, `[Fe+2]`, `[O-2]`, and the old forms `[Fe++]` and `[O--]`.
6. **Class**: `[CH3:7]`.

**New Rust:**
- Each optional piece is "peek, and if it matches, consume", which is exactly what
  `take` and `take_number` do.
- `Option` combinators save `if let` boilerplate:
  `scanner.take_number().map(u16::try_from)` gives `Option<Result<u16, _>>`, and
  `.transpose()` flips it to `Result<Option<u16>, _>`. Write it with `match` first
  if that reads better.
- Struct update syntax keeps the tests short: build a default carbon once, then
  write `BracketAtom { charge: -1, ..carbon }`.

**Check:** `[13C@@H2+2:7]` gives every field set. `N[C@](C)(F)C(=O)O` lexes.

## Step 6: Make `LexError` a real error (10 min)

**Write:** `impl std::fmt::Display for LexError` with a helpful message per variant,
and `impl std::error::Error for LexError {}` (it's empty: the trait's methods all
have defaults). T04 relies on this pattern.

**New Rust:** your first trait implementation. The shape:

```rust
impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedEnd => write!(f, "..."),
            ...
        }
    }
}
```

Messages are lowercase with no trailing period, e.g.
`"unexpected character 'X' at position 1"`.

**Check:** `LexError::UnexpectedEnd.to_string()` gives your message.

## Stretch (optional, ~20 min)

Make `lex` return an iterator instead of a `Vec` (a preview of T04):
`pub fn tokens(smiles: &str) -> impl Iterator<Item = Result<Token, LexError>>`.
Hint: `std::iter::from_fn`.

## API guidelines to check at the end

Reviews cite the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html)
by code. The ones this tutorial touches:

- **C-COMMON-TRAITS**: public types derive every common trait that applies
  (`Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default`). For each
  type, ask why a trait is *missing*: should `BondKind` be `Hash`? Does `Token`
  have a meaningful order?
- **C-GOOD-ERR**: error types implement `std::error::Error`, with lowercase
  `Display` messages that carry what the caller needs (here, the position).
- **C-FAILURE**: `lex` documents its errors under a `# Errors` heading.
- **C-STRUCT-PRIVATE vs. passive data**: `BracketAtom` has public fields because
  it's a plain record of what the text said. `Scanner` keeps its fields private
  because `cursor` must stay within `characters`.
- **C-HIDDEN**: is `Scanner` part of the public API, or an implementation
  detail? Decide, and say why in the review.

## Done?

Say **"done"** for a review, or **"fill in"** at any point to have Claude finish
the rest. In the review I'll look at how you structured the big `match`, the error
handling, and the guidelines above. Tell me how enums, `Result`/`?`, and patterns
felt.
