//! A lexer (tokenizer) for a subset of SMILES.
//!
//! Tutorial 02 spec: Claude's reference for types, docs and tests when filling in
//! after the step-by-step build in `README.md`. Not meant to be copied.
//!
//! A lexer turns a string into a flat list of tokens. It does not check that the
//! tokens make sense together (that is the parser's job, in tutorial 05). For
//! example, `"C)("` lexes fine but will not parse.
//!
//! Supported SMILES subset (enough for most organic molecules):
//!
//! | SMILES text            | token                                   |
//! |------------------------|-----------------------------------------|
//! | `B C N O P S F Cl Br I` | `Atom { element, aromatic: false }`     |
//! | `b c n o p s`          | `Atom { element, aromatic: true }`      |
//! | `[...]`                | `BracketAtom(BracketAtom { .. })`       |
//! | `- = # $ :`            | `Bond(BondKind::...)`                   |
//! | `/ \`                  | `Bond(BondKind::Up)` / `Bond(BondKind::Down)` |
//! | `(` `)`                | `BranchOpen` / `BranchClose`            |
//! | `0`-`9`, `%nn`         | `RingClosure(n)`                        |
//! | `.`                    | `Dot`                                   |
//!
//! Inside brackets we support: an optional isotope, a symbol (any capitalized
//! symbol known to `Element::from_symbol`, or one of the aromatic `b c n o p s`),
//! an optional chirality `@` or `@@`, an optional hydrogen count `H` or `Hn`,
//! an optional charge (`+`, `-`, `+n`, `-n`, and the old forms `++` and `--`),
//! and an optional atom class `:n`. Examples: `[NH4+]`, `[13CH3]`, `[C@@H]`,
//! `[O-]`, `[nH]`, `[Fe+2]`, `[CH3:7]`.

use crate::Element;

/// The kind of an explicit bond symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BondKind {
    /// `-`
    Single,
    /// `=`
    Double,
    /// `#`
    Triple,
    /// `$`
    Quadruple,
    /// `:`
    Aromatic,
    /// `/` (cis/trans marker)
    Up,
    /// `\` (cis/trans marker)
    Down,
}

/// Tetrahedral chirality marker inside brackets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chirality {
    /// `@`: looking from the first neighbor, the rest are counterclockwise.
    CounterClockwise,
    /// `@@`: looking from the first neighbor, the rest are clockwise.
    Clockwise,
}

/// The contents of a bracket atom like `[13CH3+:2]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BracketAtom {
    /// The mass number, e.g. `13` in `[13C]`.
    pub isotope: Option<u16>,
    /// The atomic number.
    pub element: u8,
    /// Whether the symbol was lowercase, e.g. `[nH]`.
    pub aromatic: bool,
    /// The chirality marker, `@` or `@@`.
    pub chirality: Option<Chirality>,
    /// The hydrogen count (`H` is 1, `H3` is 3, no `H` is 0).
    pub hydrogens: u8,
    /// The formal charge.
    pub charge: i8,
    /// The atom class, e.g. `7` in `[CH3:7]`.
    pub class: Option<u16>,
}

/// One SMILES token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// An organic-subset atom without brackets, e.g. `C` or `c`.
    #[allow(missing_docs)] // the field names say it all
    Atom { element: u8, aromatic: bool },
    /// A bracket atom, e.g. `[NH4+]`.
    BracketAtom(BracketAtom),
    /// An explicit bond symbol.
    Bond(BondKind),
    /// `(`
    BranchOpen,
    /// `)`
    BranchClose,
    /// A ring-closure digit, `1` or `%12`.
    RingClosure(u8),
    /// `.`, separating disconnected parts.
    Dot,
}

/// A lexing error, recording the character position where it happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexError {
    /// A character that cannot start any token, e.g. `X` or `!`.
    #[allow(missing_docs)] // the field names say it all
    UnexpectedCharacter { position: usize, character: char },
    /// The input ended in the middle of a token, e.g. `[CH4` or `%1`.
    UnexpectedEnd,
    /// A bracket atom whose symbol is not a known element, e.g. `[Xx]`.
    #[allow(missing_docs)] // the field names say it all
    UnknownElement { position: usize },
}

/// A cursor over the characters of a string.
///
/// This is modeled on `purr`'s `Scanner` (see
/// `~/code/sm/resources/chem/purr/src/read/scanner.rs`).
/// Working with a `Vec<char>` is simpler than with UTF-8 byte offsets; tutorial 03
/// redoes this with byte slices.
#[derive(Debug, Clone)]
pub struct Scanner {
    characters: Vec<char>,
    cursor: usize,
}

impl Scanner {
    /// Create a scanner positioned at the start of `input`.
    #[must_use]
    pub fn new(input: &str) -> Self {
        Self { characters: input.chars().collect(), cursor: 0 }
    }

    /// The position of the next character.
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// The next character, without consuming it.
    #[must_use]
    pub fn peek(&self) -> Option<char> {
        self.characters.get(self.cursor).copied()
    }

    /// Consume and return the next character.
    pub fn pop(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.cursor += 1;
        Some(character)
    }

    /// Consume the next character only if it equals `expected`, and report whether
    /// it did. This makes optional syntax like the second `@` in `@@` easy.
    pub fn take(&mut self, expected: char) -> bool {
        let matches = self.peek() == Some(expected);
        if matches {
            self.cursor += 1;
        }
        matches
    }

    /// Consume a run of ASCII digits and return their value, or `None` if the next
    /// character is not a digit. Very long runs saturate at `u32::MAX`; callers
    /// convert to their own integer type with `try_from`.
    pub fn take_number(&mut self) -> Option<u32> {
        let mut number = None;
        while let Some(digit) = self.peek().and_then(|c| c.to_digit(10)) {
            self.cursor += 1;
            number = Some(number.unwrap_or(0_u32).saturating_mul(10).saturating_add(digit));
        }
        number
    }
}

/// Split a SMILES string into tokens.
///
/// # Errors
///
/// Returns a [`LexError`] for unknown characters, unknown bracket elements, or
/// input that ends in the middle of a token.
pub fn lex(smiles: &str) -> Result<Vec<Token>, LexError> {
    let mut scanner = Scanner::new(smiles);
    let mut tokens = Vec::new();
    while let Some(token) = next_token(&mut scanner)? {
        tokens.push(token);
    }
    Ok(tokens)
}

/// Read one token, or return `Ok(None)` at the end of the input.
///
/// Suggested structure: `let Some(c) = scanner.pop() else { return Ok(None) };`
/// followed by one `match c { ... }` with an arm per token kind. Two-letter organic
/// symbols (`Cl`, `Br`) need a `scanner.take(...)` lookahead. Delegate brackets to
/// `read_bracket_atom` and `%nn` ring closures to a small helper.
fn next_token(scanner: &mut Scanner) -> Result<Option<Token>, LexError> {
    todo!()
}

/// Read a bracket atom. The opening `[` has already been consumed.
fn read_bracket_atom(scanner: &mut Scanner) -> Result<BracketAtom, LexError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(element: u8) -> Token {
        Token::Atom { element, aromatic: false }
    }

    fn aromatic(element: u8) -> Token {
        Token::Atom { element, aromatic: true }
    }

    fn bracket(element: u8) -> BracketAtom {
        BracketAtom {
            isotope: None,
            element,
            aromatic: false,
            chirality: None,
            hydrogens: 0,
            charge: 0,
            class: None,
        }
    }

    #[test]
    fn scanner_basics() {
        let mut s = Scanner::new("C@@12");
        assert_eq!(s.peek(), Some('C'));
        assert_eq!(s.pop(), Some('C'));
        assert!(s.take('@'));
        assert!(s.take('@'));
        assert!(!s.take('@'));
        assert_eq!(s.cursor(), 3);
        assert_eq!(s.take_number(), Some(12));
        assert_eq!(s.pop(), None);
        assert_eq!(s.take_number(), None);
    }

    #[test]
    fn empty() {
        assert_eq!(lex(""), Ok(vec![]));
    }

    #[test]
    fn ethanol() {
        assert_eq!(lex("CCO"), Ok(vec![atom(6), atom(6), atom(8)]));
    }

    #[test]
    fn two_letter_symbols() {
        assert_eq!(lex("ClCBr"), Ok(vec![atom(17), atom(6), atom(35)]));
    }

    #[test]
    fn bonds_and_branches() {
        assert_eq!(
            lex("CC(=O)O"),
            Ok(vec![
                atom(6),
                atom(6),
                Token::BranchOpen,
                Token::Bond(BondKind::Double),
                atom(8),
                Token::BranchClose,
                atom(8),
            ])
        );
        assert_eq!(
            lex("C#N"),
            Ok(vec![atom(6), Token::Bond(BondKind::Triple), atom(7)])
        );
    }

    #[test]
    fn ring_closures() {
        assert_eq!(
            lex("C1CC1"),
            Ok(vec![
                atom(6),
                Token::RingClosure(1),
                atom(6),
                atom(6),
                Token::RingClosure(1),
            ])
        );
        assert_eq!(
            lex("C%12C%12"),
            Ok(vec![
                atom(6),
                Token::RingClosure(12),
                atom(6),
                Token::RingClosure(12),
            ])
        );
    }

    #[test]
    fn aromatic_benzene() {
        let tokens = lex("c1ccccc1").expect("benzene lexes");
        assert_eq!(tokens.len(), 8);
        assert_eq!(tokens[0], aromatic(6));
        assert_eq!(tokens[1], Token::RingClosure(1));
    }

    #[test]
    fn cis_trans_bonds() {
        assert_eq!(
            lex("F/C=C\\F"),
            Ok(vec![
                atom(9),
                Token::Bond(BondKind::Up),
                atom(6),
                Token::Bond(BondKind::Double),
                atom(6),
                Token::Bond(BondKind::Down),
                atom(9),
            ])
        );
    }

    #[test]
    fn dot_disconnected() {
        assert_eq!(lex("[Na+].[Cl-]").map(|t| t.len()), Ok(3));
        assert_eq!(lex("C.C"), Ok(vec![atom(6), Token::Dot, atom(6)]));
    }

    #[test]
    fn bracket_simple() {
        assert_eq!(lex("[O-]"), Ok(vec![Token::BracketAtom(BracketAtom { charge: -1, ..bracket(8) })]));
        assert_eq!(
            lex("[NH4+]"),
            Ok(vec![Token::BracketAtom(BracketAtom { hydrogens: 4, charge: 1, ..bracket(7) })])
        );
        assert_eq!(lex("[CH]"), Ok(vec![Token::BracketAtom(BracketAtom { hydrogens: 1, ..bracket(6) })]));
    }

    #[test]
    fn bracket_full() {
        assert_eq!(
            lex("[13C@@H2+2:7]"),
            Ok(vec![Token::BracketAtom(BracketAtom {
                isotope: Some(13),
                chirality: Some(Chirality::Clockwise),
                hydrogens: 2,
                charge: 2,
                class: Some(7),
                ..bracket(6)
            })])
        );
    }

    #[test]
    fn bracket_charges() {
        let charge = |s: &str| match lex(s) {
            Ok(tokens) => match &tokens[..] {
                [Token::BracketAtom(b)] => b.charge,
                _ => panic!("expected one bracket atom"),
            },
            Err(e) => panic!("lex error {e:?}"),
        };
        assert_eq!(charge("[Fe+2]"), 2);
        assert_eq!(charge("[Fe++]"), 2);
        assert_eq!(charge("[O--]"), -2);
        assert_eq!(charge("[O-2]"), -2);
        assert_eq!(charge("[C]"), 0);
    }

    #[test]
    fn bracket_aromatic() {
        assert_eq!(
            lex("[nH]"),
            Ok(vec![Token::BracketAtom(BracketAtom { aromatic: true, hydrogens: 1, ..bracket(7) })])
        );
    }

    #[test]
    fn chiral_center() {
        let tokens = lex("N[C@](C)(F)C(=O)O").expect("lexes");
        assert_eq!(
            tokens[1],
            Token::BracketAtom(BracketAtom {
                chirality: Some(Chirality::CounterClockwise),
                ..bracket(6)
            })
        );
    }

    #[test]
    fn errors() {
        assert_eq!(
            lex("CX"),
            Err(LexError::UnexpectedCharacter { position: 1, character: 'X' })
        );
        assert_eq!(lex("[CH4"), Err(LexError::UnexpectedEnd));
        assert_eq!(lex("C%1"), Err(LexError::UnexpectedEnd));
        assert_eq!(lex("[Xx]"), Err(LexError::UnknownElement { position: 1 }));
    }
}
