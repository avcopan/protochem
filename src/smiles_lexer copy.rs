//! A lexer for a subset of SMILES.
//!
//! [`lex`] turns a SMILES string into a flat list of [`Token`]s. It doesn't check
//! that the tokens make sense together; that's the parser's job. For example,
//! `"C)("` lexes fine but won't parse.
//!
//! | SMILES text             | token                                         |
//! |-------------------------|-----------------------------------------------|
//! | `B C N O P S F Cl Br I` | `Atom { atomic_number, aromatic: false }`     |
//! | `b c n o p s`           | `Atom { atomic_number, aromatic: true }`      |
//! | `[...]`                 | `BracketAtom(BracketAtom { .. })`             |
//! | `- = # $ :`             | `Bond(BondKind::...)`                         |
//! | `/ \`                   | `Bond(BondKind::Up)` / `Bond(BondKind::Down)` |
//! | `(` `)`                 | `BranchOpen` / `BranchClose`                  |
//! | `0`-`9`, `%nn`          | `RingClosure(n)`                              |
//! | `.`                     | `Dot`                                         |
//!
//! A bracket atom is `[isotope? symbol chirality? hcount? charge? class?]`, e.g.
//! `[13CH3]`, `[C@@H]`, `[NH4+]`, `[Fe+2]` or `[CH3:7]`. Two-letter aromatic
//! symbols like `[se]` are not supported, the hydrogen count is a single digit
//! (`[CH12]` is an error), and chirality is tetrahedral only (`@TH1` and `@SP1` are
//! errors).
use crate::Element;

/// The kind of an explicit bond symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// `/`, a cis/trans marker.
    Up,
    /// `\`, a cis/trans marker.
    Down,
}

/// A tetrahedral chirality marker inside brackets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chirality {
    /// `@`: looking from the first neighbor, the rest are counterclockwise.
    CounterClockwise,
    /// `@@`: looking from the first neighbor, the rest are clockwise.
    Clockwise,
}

/// The contents of a bracket atom like `[13CH3+:2]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct BracketAtom {
    /// The mass number, e.g. `13` in `[13C]`.
    pub isotope: Option<u16>,
    /// The atomic number.
    pub atomic_number: u8,
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum Token {
    /// An organic-subset atom without brackets, e.g. `C` or `c`.
    #[expect(missing_docs)] // the field names say it all
    Atom { atomic_number: u8, aromatic: bool },
    /// A bracket atom, e.g. `[NH4+]`.
    BracketAtom(BracketAtom),
    /// An explicit bond symbol.
    Bond(BondKind),
    /// `(`
    BranchOpen,
    /// `)`
    BranchClose,
    /// `.`, separating disconnected parts.
    Dot,
    /// A ring-closure label, `1` or `%12`.
    RingClosure(u8),
}

/// A lexing error, recording the character position where it happened.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LexError {
    /// A character that can't appear here, e.g. `X` in `CX`.
    #[expect(missing_docs)] // the field names say it all
    UnexpectedCharacter { position: usize, character: char },
    /// The input ended in the middle of a token, e.g. `[CH4` or `C%1`.
    #[expect(missing_docs)] // the field names say it all
    UnexpectedEnd { position: usize },
    /// A bracket atom symbol that isn't an element, e.g. `[Xx]`.
    #[expect(missing_docs)] // the field names say it all
    UnknownElement { position: usize, symbol: String },
    /// A number too large for its field, e.g. `[99999C]`.
    #[expect(missing_docs)] // the field names say it all
    NumberOutOfRange { position: usize },
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedCharacter {
                position,
                character,
            } => write!(
                f,
                "unexpected character {character:?} at position {position}"
            ),
            Self::UnexpectedEnd { position } => {
                write!(f, "unexpected end of input at position {position}")
            }
            Self::UnknownElement { position, symbol } => {
                write!(f, "unknown element {symbol:?} at position {position}")
            }
            Self::NumberOutOfRange { position } => {
                write!(f, "number out of range at position {position}")
            }
        }
    }
}

impl std::error::Error for LexError {}

/// A cursor over the characters of a string.
///
/// Its methods know nothing about SMILES; the grammar lives in the `read_*`
/// functions.
struct Scanner {
    characters: Vec<char>,
    cursor: usize,
}

impl Scanner {
    /// Create a scanner positioned at the start of `input`.
    fn new(input: &str) -> Self {
        Self {
            characters: input.chars().collect(),
            cursor: 0,
        }
    }

    /// The next character, without consuming it.
    fn peek(&self) -> Option<char> {
        self.characters.get(self.cursor).copied()
    }

    /// Consume and return the next character.
    fn pop(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.cursor += 1;
        Some(c)
    }

    /// The position of the next character.
    fn cursor(&self) -> usize {
        self.cursor
    }

    /// Consume the next character only if it is `expected`, and report whether it was.
    fn take(&mut self, expected: char) -> bool {
        let matches = self.peek() == Some(expected);
        if matches {
            self.cursor += 1;
        }
        matches
    }

    /// Consume the next character only if it is a digit, and return its value.
    fn take_digit(&mut self) -> Option<u8> {
        let d = digit(self.peek()?)?;
        self.cursor += 1;
        Some(d)
    }

    /// Consume the next character only if `predicate` accepts it, and return it.
    fn take_if(&mut self, predicate: impl Fn(char) -> bool) -> Option<char> {
        let c = self.peek().filter(|&c| predicate(c))?;
        self.cursor += 1;
        Some(c)
    }

    /// Consume a run of digits and return its value, saturating at `u32::MAX`, or
    /// `None` if no digit is next.
    fn take_number(&mut self) -> Option<u32> {
        let mut number = u32::from(self.take_digit()?);
        while let Some(d) = self.take_digit() {
            number = number.saturating_mul(10).saturating_add(u32::from(d));
        }
        Some(number)
    }
}

/// The token for an organic-subset symbol, which must be an element.
fn organic_atom(symbol: &str, aromatic: bool) -> Token {
    let element = Element::from_symbol(symbol).expect("organic subset symbols are elements");
    Token::Atom {
        atomic_number: element.atomic_number,
        aromatic,
    }
}

/// The value of a decimal digit, or `None` for any other character.
fn digit(c: char) -> Option<u8> {
    u8::try_from(c.to_digit(10)?).ok()
}

/// The error for whatever the scanner is looking at, which the caller didn't expect.
fn unexpected(scanner: &Scanner) -> LexError {
    match scanner.peek() {
        None => LexError::UnexpectedEnd {
            position: scanner.cursor(),
        },
        Some(c) => LexError::UnexpectedCharacter {
            position: scanner.cursor(),
            character: c,
        },
    }
}

/// Read one required digit.
fn read_digit(scanner: &mut Scanner) -> Result<u8, LexError> {
    scanner.take_digit().ok_or_else(|| unexpected(scanner))
}

/// Read an optional number and convert it to `T`, e.g. `u16` for an isotope.
fn read_number<T: TryFrom<u32>>(scanner: &mut Scanner) -> Result<Option<T>, LexError> {
    let position = scanner.cursor();
    scanner
        .take_number()
        .map(|n| T::try_from(n).map_err(|_| LexError::NumberOutOfRange { position }))
        .transpose()
}

/// Read an optional charge: `+`, `-`, `+n`, `-n`, or a repeated sign like `++`.
fn read_charge(scanner: &mut Scanner) -> Result<i8, LexError> {
    let position = scanner.cursor();
    let Some(sign_character) = scanner.take_if(|c| c == '+' || c == '-') else {
        return Ok(0);
    };
    let sign: i8 = if sign_character == '+' { 1 } else { -1 };
    if let Some(magnitude) = read_number::<i8>(scanner)? {
        return Ok(sign * magnitude);
    }
    // The old forms `++` and `--` repeat the sign
    let mut charge = sign;
    while scanner.take(sign_character) {
        charge = charge
            .checked_add(sign)
            .ok_or(LexError::NumberOutOfRange { position })?;
    }
    Ok(charge)
}

/// Read a bracket atom. The opening `[` has already been consumed.
fn read_bracket_atom(scanner: &mut Scanner) -> Result<BracketAtom, LexError> {
    // 0. read the isotope
    let isotope = read_number(scanner)?;
    // 1. read the symbol
    let position = scanner.cursor();
    let first = scanner
        .take_if(|c| c.is_ascii_alphabetic())
        .ok_or_else(|| unexpected(scanner))?;
    let second = scanner.take_if(|c| c.is_ascii_lowercase());
    let symbol = match second {
        Some(c) => format!("{first}{c}"),
        None => first.to_string(),
    };
    // 2. determine if it is aromatic
    let aromatic = matches!(symbol.as_str(), "b" | "c" | "n" | "o" | "p" | "s");
    // 3. check that it is a valid element
    let element = if aromatic {
        Element::from_symbol(&symbol.to_ascii_uppercase())
    } else {
        Element::from_symbol(&symbol)
    };
    let element = element.ok_or(LexError::UnknownElement { position, symbol })?;
    // 4. read the chirality
    let chirality = if scanner.take('@') {
        Some(if scanner.take('@') {
            Chirality::Clockwise
        } else {
            Chirality::CounterClockwise
        })
    } else {
        None
    };
    // 5. determine the number of hydrogens
    let hydrogens = if scanner.take('H') {
        scanner.take_digit().unwrap_or(1)
    } else {
        0
    };
    // 6. read the charge
    let charge = read_charge(scanner)?;
    // 7. read the atom class
    let class = if scanner.take(':') {
        Some(read_number(scanner)?.ok_or_else(|| unexpected(scanner))?)
    } else {
        None
    };
    if !scanner.take(']') {
        return Err(unexpected(scanner));
    }

    Ok(BracketAtom {
        isotope,
        atomic_number: element.atomic_number,
        aromatic,
        chirality,
        hydrogens,
        charge,
        class,
    })
}

/// Read one token, or return `Ok(None)` at the end of the input.
fn next_token(scanner: &mut Scanner) -> Result<Option<Token>, LexError> {
    let Some(c) = scanner.pop() else {
        return Ok(None);
    };
    let token = match c {
        '-' => Token::Bond(BondKind::Single),
        '=' => Token::Bond(BondKind::Double),
        '#' => Token::Bond(BondKind::Triple),
        '$' => Token::Bond(BondKind::Quadruple),
        ':' => Token::Bond(BondKind::Aromatic),
        '/' => Token::Bond(BondKind::Up),
        '\\' => Token::Bond(BondKind::Down),
        '(' => Token::BranchOpen,
        ')' => Token::BranchClose,
        '.' => Token::Dot,
        // Br and B
        'B' => organic_atom(if scanner.take('r') { "Br" } else { "B" }, false),
        // Cl and C
        'C' => organic_atom(if scanner.take('l') { "Cl" } else { "C" }, false),
        // N, O, P, S, F, I
        c @ ('N' | 'O' | 'P' | 'S' | 'F' | 'I') => organic_atom(&c.to_string(), false),
        c @ ('b' | 'c' | 'n' | 'o' | 'p' | 's') => {
            organic_atom(&c.to_ascii_uppercase().to_string(), true)
        }
        // Ring closures
        d @ '0'..='9' => Token::RingClosure(digit(d).expect("matched 0-9")),
        '%' => {
            let tens = read_digit(scanner)?;
            let ones = read_digit(scanner)?;
            Token::RingClosure(10 * tens + ones)
        }
        '[' => {
            let bracket_atom = read_bracket_atom(scanner)?;
            Token::BracketAtom(bracket_atom)
        }
        _ => {
            return Err(LexError::UnexpectedCharacter {
                position: scanner.cursor() - 1,
                character: c,
            });
        }
    };
    Ok(Some(token))
}

/// Split a SMILES string into tokens.
///
/// # Examples
///
/// ```
/// use protochem::smiles_lexer::{self, Token};
///
/// # fn main() -> Result<(), smiles_lexer::LexError> {
/// let tokens = smiles_lexer::lex("C=O")?;
/// assert_eq!(tokens.len(), 3);
/// assert_eq!(tokens[0], Token::Atom { atomic_number: 6, aromatic: false });
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// Returns a [`LexError`] for a character that can't appear where it is, input
/// that ends in the middle of a token, an unknown bracket element, or a number too
/// large for its field.
pub fn lex(smiles: &str) -> Result<Vec<Token>, LexError> {
    let mut scanner = Scanner::new(smiles);
    let mut tokens = Vec::new();
    while let Some(token) = next_token(&mut scanner)? {
        tokens.push(token);
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(atomic_number: u8) -> Token {
        Token::Atom {
            atomic_number,
            aromatic: false,
        }
    }

    fn aromatic(atomic_number: u8) -> Token {
        Token::Atom {
            atomic_number,
            aromatic: true,
        }
    }

    // A plain bracket atom; set other fields with `BracketAtom { charge: -1, ..bracket_atom(8) }`
    fn bracket_atom(atomic_number: u8) -> BracketAtom {
        BracketAtom {
            isotope: None,
            atomic_number,
            aromatic: false,
            chirality: None,
            hydrogens: 0,
            charge: 0,
            class: None,
        }
    }

    fn bracket(atomic_number: u8, hydrogens: u8) -> Token {
        Token::BracketAtom(BracketAtom {
            hydrogens,
            ..bracket_atom(atomic_number)
        })
    }

    #[test]
    fn bonds_and_branches() {
        assert_eq!(lex(""), Ok(vec![]));

        let tokens = vec![
            Token::Bond(BondKind::Single),
            Token::Bond(BondKind::Double),
            Token::Bond(BondKind::Triple),
            Token::Bond(BondKind::Quadruple),
            Token::Bond(BondKind::Aromatic),
            Token::Bond(BondKind::Up),
            Token::Bond(BondKind::Down),
            Token::BranchOpen,
            Token::BranchClose,
            Token::Dot,
        ];
        assert_eq!(lex("-=#$:/\\()."), Ok(tokens));
    }

    #[test]
    fn organic_atoms() {
        assert_eq!(lex("CCO"), Ok(vec![atom(6), atom(6), atom(8)]));
        assert_eq!(
            lex("BNPSFI"),
            Ok(vec![
                atom(5),
                atom(7),
                atom(15),
                atom(16),
                atom(9),
                atom(53)
            ])
        );
    }

    #[test]
    fn two_letter_symbols() {
        assert_eq!(lex("ClCBr"), Ok(vec![atom(17), atom(6), atom(35)]));
        // `B` followed by something other than `r` is boron
        assert_eq!(lex("BC"), Ok(vec![atom(5), atom(6)]));
    }

    #[test]
    fn aromatic_atoms() {
        assert_eq!(
            lex("bcnops"),
            Ok(vec![
                aromatic(5),
                aromatic(6),
                aromatic(7),
                aromatic(8),
                aromatic(15),
                aromatic(16)
            ])
        );
    }

    #[test]
    fn atoms_with_bonds_and_branches() {
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
    }

    #[test]
    fn unexpected_character() {
        assert_eq!(
            lex("(X"),
            Err(LexError::UnexpectedCharacter {
                position: 1,
                character: 'X'
            })
        );
        assert_eq!(
            lex("X("),
            Err(LexError::UnexpectedCharacter {
                position: 0,
                character: 'X'
            })
        );
    }

    #[test]
    fn ring_closures() {
        let tokens = vec![
            atom(6),
            Token::RingClosure(1),
            atom(6),
            atom(6),
            Token::RingClosure(1),
        ];
        assert_eq!(lex("C1CC1"), Ok(tokens));
        assert_eq!(
            lex("C%12C%12"),
            Ok(vec![
                atom(6),
                Token::RingClosure(12),
                atom(6),
                Token::RingClosure(12)
            ])
        );
        // `%` takes exactly two digits, so the `3` is a ring closure of its own
        assert_eq!(
            lex("%123"),
            Ok(vec![Token::RingClosure(12), Token::RingClosure(3)])
        );
        assert_eq!(lex("c1ccccc1").map(|tokens| tokens.len()), Ok(8));
    }

    #[test]
    fn ring_closure_errors() {
        assert_eq!(lex("C%"), Err(LexError::UnexpectedEnd { position: 2 }));
        assert_eq!(lex("C%1"), Err(LexError::UnexpectedEnd { position: 3 }));
        assert_eq!(
            lex("C%1X"),
            Err(LexError::UnexpectedCharacter {
                position: 3,
                character: 'X'
            })
        );
    }

    #[test]
    fn bracket_atoms() {
        let tokens = vec![atom(6), bracket(6, 1), atom(6), atom(8)];
        assert_eq!(lex("C[CH]CO"), Ok(tokens));

        assert_eq!(lex("[Fe]"), Ok(vec![bracket(26, 0)]));
        assert_eq!(lex("[Cl]"), Ok(vec![bracket(17, 0)]));
        assert_eq!(lex("[CH4]"), Ok(vec![bracket(6, 4)]));
        assert_eq!(
            lex("[nH]"),
            Ok(vec![Token::BracketAtom(BracketAtom {
                aromatic: true,
                hydrogens: 1,
                ..bracket_atom(7)
            })])
        );
    }

    #[test]
    fn bracket_isotope_and_class() {
        assert_eq!(
            lex("[13C]"),
            Ok(vec![Token::BracketAtom(BracketAtom {
                isotope: Some(13),
                ..bracket_atom(6)
            })])
        );
        // The symbol `H` is hydrogen itself, not a hydrogen count
        assert_eq!(
            lex("[2H]"),
            Ok(vec![Token::BracketAtom(BracketAtom {
                isotope: Some(2),
                ..bracket_atom(1)
            })])
        );
        assert_eq!(
            lex("[CH3:7]"),
            Ok(vec![Token::BracketAtom(BracketAtom {
                hydrogens: 3,
                class: Some(7),
                ..bracket_atom(6)
            })])
        );
    }

    #[test]
    fn bracket_chirality() {
        let chiral = |chirality| {
            Token::BracketAtom(BracketAtom {
                chirality: Some(chirality),
                hydrogens: 1,
                ..bracket_atom(6)
            })
        };
        assert_eq!(lex("[C@H]"), Ok(vec![chiral(Chirality::CounterClockwise)]));
        assert_eq!(lex("[C@@H]"), Ok(vec![chiral(Chirality::Clockwise)]));

        let tokens = lex("N[C@](C)(F)C(=O)O").expect("alanine-like SMILES lexes");
        assert_eq!(
            tokens[1],
            Token::BracketAtom(BracketAtom {
                chirality: Some(Chirality::CounterClockwise),
                ..bracket_atom(6)
            })
        );
    }

    #[test]
    fn bracket_charges() {
        let charge = |smiles: &str| match lex(smiles) {
            Ok(tokens) => match &tokens[..] {
                [Token::BracketAtom(atom)] => atom.charge,
                _ => panic!("expected one bracket atom"),
            },
            Err(error) => panic!("lex error {error:?}"),
        };
        assert_eq!(charge("[C]"), 0);
        assert_eq!(charge("[NH4+]"), 1);
        assert_eq!(charge("[O-]"), -1);
        assert_eq!(charge("[Fe+2]"), 2);
        assert_eq!(charge("[O-2]"), -2);
        assert_eq!(charge("[Fe++]"), 2);
        assert_eq!(charge("[O--]"), -2);
        assert_eq!(lex("[Na+].[Cl-]").map(|tokens| tokens.len()), Ok(3));
    }

    #[test]
    fn bracket_all_fields() {
        assert_eq!(
            lex("[13C@@H2+2:7]"),
            Ok(vec![Token::BracketAtom(BracketAtom {
                isotope: Some(13),
                chirality: Some(Chirality::Clockwise),
                hydrogens: 2,
                charge: 2,
                class: Some(7),
                ..bracket_atom(6)
            })])
        );
    }

    #[test]
    fn bracket_errors() {
        assert_eq!(
            lex("[Xx]"),
            Err(LexError::UnknownElement {
                position: 1,
                symbol: "Xx".to_string()
            })
        );
        // The symbol starts after the isotope
        assert_eq!(
            lex("[13Xx]"),
            Err(LexError::UnknownElement {
                position: 3,
                symbol: "Xx".to_string()
            })
        );
        assert_eq!(lex("[C"), Err(LexError::UnexpectedEnd { position: 2 }));
        assert_eq!(
            lex("[]"),
            Err(LexError::UnexpectedCharacter {
                position: 1,
                character: ']'
            })
        );
        // A class needs a number after the `:`
        assert_eq!(
            lex("[CH3:]"),
            Err(LexError::UnexpectedCharacter {
                position: 5,
                character: ']'
            })
        );
        assert_eq!(
            lex("[99999C]"),
            Err(LexError::NumberOutOfRange { position: 1 })
        );
        assert_eq!(
            lex("[C+999]"),
            Err(LexError::NumberOutOfRange { position: 3 })
        );
    }

    #[test]
    fn error_messages() {
        assert_eq!(
            lex("CX").unwrap_err().to_string(),
            "unexpected character 'X' at position 1"
        );
        assert_eq!(
            lex("[CH4").unwrap_err().to_string(),
            "unexpected end of input at position 4"
        );
        assert_eq!(
            lex("[Xx]").unwrap_err().to_string(),
            "unknown element \"Xx\" at position 1"
        );
        assert_eq!(
            lex("[99999C]").unwrap_err().to_string(),
            "number out of range at position 1"
        );
        // `LexError` works as a standard error
        let _: Box<dyn std::error::Error> = Box::new(LexError::UnexpectedEnd { position: 0 });
    }
}
