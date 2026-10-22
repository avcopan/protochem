# T02: Enums, Structs and Errors: a SMILES Lexer

**Time:** ~2 h · **Builds:** `src/smiles_lexer.rs` · **Needs:** T01's `element.rs`

## Why this matters

Enums with data are Rust's most important modeling tool. They'll describe stereo
parities, bond changes (`Formed`/`Broken`/`Fleeting`), LS kinds, search frames in
VF2++, and errors everywhere. A lexer is the perfect place to practice them: every
token is a variant, and lexing is a big `match`. It's also the first stage of the
SMILES reader.

## Reading (≈ 30 min)

- Rust book **6.1** (enums with data) and **9.2** "Recoverable Errors with
  `Result`" (the `?` operator). Skip chapter 5 (structs): you'll pick it up from
  the stub's `Scanner`.
- Rust book **19.3** "Pattern Syntax", as a reference to skim: ranges `'0'..='9'`,
  bindings `c @ 'a'..='z'`, guards `c if c.is_ascii_uppercase()`, struct and slice
  patterns.
- Look at how `purr` does it: `resources - rust-graph-and-cheminformatics-info/purr/src/read/scanner.rs`
  (the `Scanner`), `read_organic.rs`, and `read_bracket.rs`. Read for ideas, but
  write your own.
- OpenSMILES grammar (optional): `resources - rust-graph-and-cheminformatics-info/OpenSMILES/opensmiles.html`,
  section "Grammar" and the "Atoms" section on bracket atoms.

## Concepts to notice while you work

- `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`: which derives are possible
  for which types, and why `Token` can't be `Copy` but `BondKind` can (`BracketAtom`
  could be `Copy` too. Should it be?).
- **Struct update syntax** in the tests: `BracketAtom { charge: -1, ..bracket(8) }`.
- **Slice patterns** in `bracket_charges`: `match &tokens[..] { [Token::BracketAtom(b)] => ... }`.
- **Iterators and closures** work like Python's generators and lambdas, and you
  can use them freely from here on (T06 is folded into T04). For small `Copy`
  items, call `.copied()` right after `.iter()`, as in T01's `implicit_hydrogens`.
  `Scanner::take_number` shows `while let` with `and_then`.
- `Option` combinators you'll want: `ok_or`, `map_or`, `and_then`, `filter`.
  Converting `Option` → `Result` with `.ok_or(LexError::UnexpectedEnd)?` is the
  idiom for "running out of input is an error here".
- Integer conversions: `take_number` returns a `u32`, and tokens store `u8`/`u16`.
  Use `u8::try_from(x)`, not `x as u8`. Clippy pedantic will flag
  lossy `as` casts. Decide where an `unwrap()` is truly impossible to fail and where
  an error is needed.

## The exercise

1. Copy `smiles_lexer.rs` to `src/smiles_lexer.rs` and add
   `pub mod smiles_lexer;` to `lib.rs`.
2. Read the given `Scanner` (a plain struct with an `impl` block; that's all of
   Book chapter 5 you need). Then implement `next_token`, then
   `read_bracket_atom`. Test incrementally:
   `pixi run -e dev cargo test smiles_lexer::tests::ethanol`, and so on. Leave
   the bracket tests for last. Look up elements with `Element::from_symbol` from
   T01 (aromatic symbols like `c` need capitalizing first).
3. Implement `std::fmt::Display` for `LexError` with a helpful message, and
   `impl std::error::Error for LexError {}` (C-GOOD-ERR, below). T04 relies on
   this pattern.

Suggested order inside `read_bracket_atom` (OpenSMILES bracket atom grammar):
`isotope? symbol chiral? hcount? charge? class? ']'`. Each optional piece is
"peek, and if it matches, consume". That's what `take` and `take_number` are for.

Error positions: `UnexpectedCharacter.position` is the index of the offending
character. `UnknownElement.position` is where the symbol starts (just after `[`
and any isotope).

## Stretch (optional, ~20 min)

- Make `lex` return an iterator instead of a `Vec` (preview of T06):
  `pub fn tokens(smiles: &str) -> impl Iterator<Item = Result<Token, LexError>>`.
  Hint: `std::iter::from_fn`.

## API guidelines in this tutorial

Reviews cite the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html)
by code. The ones this tutorial touches:

- **C-COMMON-TRAITS**: public types derive every common trait that applies
  (`Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default`). For each
  type in the stub, ask why a trait is *missing*: should `BondKind` be `Hash`?
  Does `Token` have a meaningful order?
- **C-GOOD-ERR**: error types implement `std::error::Error` (via `Display` +
  `Debug`). `Display` messages are lowercase with no trailing period, e.g.
  `"unexpected character 'X' at position 1"`. The error carries what the caller
  needs to act on it, which here is the position.
- **C-FAILURE**: `lex` documents its `# Errors`. Clippy pedantic
  (`missing_errors_doc`) checks that the section exists, and review checks that
  it's accurate.
- **C-QUESTION-MARK**: if you add a doc example to `lex`, use `?` with a hidden
  `# fn main() -> Result<(), protochem::smiles_lexer::LexError> {`, not
  `unwrap()`. Examples get copied verbatim.
- **C-STRUCT-PRIVATE vs. passive data**: `BracketAtom` has public fields. That's
  fine for passive data with no invariants (it's a record of what the text said).
  `Scanner` keeps its fields private because `cursor` must stay within
  `characters`.
- **C-CUSTOM-TYPE**: `aromatic: bool` is fine as a *field*, since the name reads
  at the use site. As a function *argument* (`atom(6, true)`) it wouldn't be.
- **C-HIDDEN**: is `Scanner` part of the public API, or an implementation detail
  that should be `pub(crate)`? Decide, and say why in the review.
- **Book 7.4 `use` paths**: types are imported directly (`use crate::Element;`,
  then `Element::from_symbol(...)`), while functions are called through their
  module (`element::is_organic(...)`).

## Done?

Say **"done"**. In the review I'll pay particular attention to how you structured
the big `match`, the error handling, and the guidelines above. Tell me how enums, `Result`/`?`, and
patterns felt.
