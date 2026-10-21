# T02: Enums, Structs and Errors: a SMILES Lexer

**Time:** ~3 h · **Builds:** `src/smiles_lexer.rs` · **Needs:** T01's `element.rs`

## Why this matters

Enums with data are Rust's most important modeling tool. They'll describe stereo
parities, bond changes (`Formed`/`Broken`/`Fleeting`), LS kinds, search frames in
VF2++, and errors everywhere. A lexer is the perfect place to practice them: every
token is a variant, and lexing is a big `match`. It's also the first stage of the
SMILES reader.

## Reading (≈ 45 min)

- Rust book **5** (structs, methods, `impl`). Skim if familiar.
- Rust book **6.1** (enums with data) and re-skim **6.2–6.3**.
- Rust book **9.2** "Recoverable Errors with `Result`" (the `?` operator).
- Rust book **19.3** "Pattern Syntax": ranges `'0'..='9'`, bindings `c @ 'a'..='z'`,
  guards `c if c.is_ascii_uppercase()`, struct and slice patterns.
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
- `Option` combinators you'll want: `ok_or`, `map_or`, `and_then`, `filter`.
  Converting `Option` → `Result` with `.ok_or(LexError::UnexpectedEnd)?` is the
  idiom for "running out of input is an error here".
- Integer conversions: `u8::try_from(x)` vs `x as u8`. Clippy pedantic will flag
  lossy `as` casts. Decide where an `unwrap()` is truly impossible to fail and where
  an error is needed.

## The exercise

1. Copy `smiles_lexer.rs` to `src/smiles_lexer.rs` and add
   `pub mod smiles_lexer;` to `lib.rs`.
2. Extend `element::atomic_number` (and `symbol`) to cover Br (35), I (53) and
   Fe (26). Optionally use a `const` table now that the list is growing. Keep T01's
   tests passing.
3. Implement `Scanner` (5 small methods), then `next_token`, then
   `read_bracket_atom`. Test incrementally:
   `pixi run -e dev cargo test smiles_lexer::tests::scanner_basics`, then
   `ethanol`, and so on. Leave the bracket tests for last.

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
- Implement `std::fmt::Display` for `LexError` with a helpful message, and
  `impl std::error::Error for LexError {}`. T04 relies on this pattern.

## Done?

Say **"done"**. In the review I'll pay particular attention to how you structured
the big `match` and the error handling. Tell me how enums, `Result`/`?`, and
patterns felt.
