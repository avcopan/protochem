//! A SMILES lexer.
//!
//! [`lex`] splits a SMILES string into [`Token`]s, checking each token on its own. Whether the
//! tokens fit together (balanced branches, closed rings) is the parser's job.
//!
//! A bracket atom is `[isotope? symbol chirality? hcount? charge? class?]`.
use crate::Element;
use std::fmt;

mod bracket_atom;
use bracket_atom::read_bracket_atom;
pub use bracket_atom::{BracketAtom, Chirality};

/// An organic-subset atom written without brackets, e.g. `C` or `c`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Atom {
    /// The atomic number, or `0` for the wildcard `*`.
    pub atomic_number: u8,
    /// Whether the symbol was lowercase.
    pub aromatic: bool,
}

/// An explicit bond symbol.
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
    /// `/`
    Up,
    /// `\`
    Down,
}

/// One SMILES token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Token {
    // Atoms
    /// `C`, `Cl`, `c`, ...
    Atom(Atom),
    /// `[...]`
    BracketAtom(BracketAtom),
    // Bonds
    /// `-`, `=`, `#`, ...
    Bond(BondKind),
    // Branches
    /// `(`
    BranchOpen,
    /// `)`
    BranchClose,
    // Ring closures
    /// `1` or `%12`
    RingClosure(u8),
    // Fragment separators
    /// `.`
    Dot,
}

/// A token plus the input text it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Lexeme<'a> {
    /// The token.
    pub token: Token,
    /// The token's text, borrowed from the input.
    pub text: &'a str,
    /// Byte offset of `text` in the input.
    pub position: usize,
}

/// A lexing error.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LexError {
    /// Byte offset into the input.
    pub position: usize,
    /// What went wrong.
    pub kind: LexErrorKind,
}

impl LexError {
    fn new(position: usize, kind: LexErrorKind) -> Self {
        Self { position, kind }
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at position {}", self.kind, self.position)
    }
}

impl std::error::Error for LexError {}

/// The kind of [`LexError`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LexErrorKind {
    /// An unexpected character occurred.
    UnexpectedCharacter(char),
    /// The input ended mid-token.
    UnexpectedEnd,
    /// The bracket-atom symbol isn't an element.
    UnknownElement(String),
    /// A number was out of range for its field.
    NumberOutOfRange,
}

impl fmt::Display for LexErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter(c) => write!(f, "unexpected character {c:?}"),
            Self::UnexpectedEnd => write!(f, "unexpected end of input"),
            Self::UnknownElement(s) => write!(f, "unknown element {s:?}"),
            Self::NumberOutOfRange => write!(f, "number out of range"),
        }
    }
}

/// An iterator over the lexemes of a SMILES string.
///
/// Each [`Lexeme`] borrows its `text` from the input and records its byte
/// `position`. The lexer stops at the first whitespace, the end of the string,
/// or after the first error.
///
/// # Examples
///
/// ```
/// use protochem::smiles_lexer::Lexer;
///
/// let texts: Vec<_> = Lexer::new("[CH2]CO").map(|lexeme| lexeme.unwrap().text).collect();
/// assert_eq!(texts, ["[CH2]", "C", "O"]);
/// ```
///
/// Lexemes borrow from the input, so they can't outlive it:
/// ```compile_fail,E0597
/// use protochem::smiles_lexer::Lexer;
///
/// let lexemes: Vec<_> = {
///     let smiles = String::from("[CH2]CO");
///     Lexer::new(&smiles).collect()
/// };
/// // error: `lexemes` borrow from `smiles`, which has gone out of scope
/// println!("{lexemes:?}");
/// ```
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    scanner: Scanner<'a>,
    done: bool,
}

impl<'a> Lexer<'a> {
    /// Create a lexer over `smiles`.
    #[must_use]
    pub fn new(smiles: &'a str) -> Self {
        let scanner = Scanner::new(smiles);
        Self {
            scanner,
            done: false,
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Lexeme<'a>, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let position = self.scanner.cursor;
        let result = next_token(&mut self.scanner);
        self.done = !matches!(result, Ok(Some(_)));
        let text = self.scanner.slice(position);

        result
            // Result<Option<T>, E> -> Option<Result<T, E>>
            .transpose()
            // Option::map (operates on r <- Some(r))
            .map(|result| {
                // Result::map (operates on t <- Ok(t))
                result.map(|token| Lexeme {
                    token,
                    text,
                    position,
                })
            })
    }
}

impl std::iter::FusedIterator for Lexer<'_> {}

/// Split a SMILES string into tokens.
///
/// Use [`Lexer`] instead to obtain the text and positions of each lexeme.
///
/// # Errors
///
/// Returns a [`LexError`] at the first thing that isn't a valid token, e.g. the `X` in `CX`, an
/// unknown element in `[Xx]`, or an out-of-range chirality class in `[C@TH3]`. The error gives
/// the byte position and a [`LexErrorKind`].
///
/// # Examples
///
/// ```
/// use protochem::smiles_lexer::{lex, Atom, LexErrorKind, Token};
///
/// let tokens = lex("CO").expect("valid SMILES");
/// let carbon = Atom { atomic_number: 6, aromatic: false };
/// let oxygen = Atom { atomic_number: 8, aromatic: false };
/// assert_eq!(tokens, [Token::Atom(carbon), Token::Atom(oxygen)]);
///
/// let error = lex("CX").unwrap_err();
/// assert_eq!(error.position, 1);
/// assert_eq!(error.kind, LexErrorKind::UnexpectedCharacter('X'));
/// ```
pub fn lex(smiles: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(smiles)
        .map(|result| result.map(|lexeme| lexeme.token))
        .collect()
}

fn next_token(scanner: &mut Scanner<'_>) -> Result<Option<Token>, LexError> {
    let start = scanner.cursor;
    // Whitespace or scanner end -> None
    let Some(c) = scanner.pop().filter(|c| !c.is_ascii_whitespace()) else {
        return Ok(None);
    };

    let token = match c {
        // Atoms
        c if b"*BCNOPSFIbcnops".contains(&c) => Token::Atom(read_organic_atom(scanner, c, start)),
        b'[' => Token::BracketAtom(read_bracket_atom(scanner)?),
        // Bonds
        b'-' => Token::Bond(BondKind::Single),
        b'=' => Token::Bond(BondKind::Double),
        b'#' => Token::Bond(BondKind::Triple),
        b'$' => Token::Bond(BondKind::Quadruple),
        b':' => Token::Bond(BondKind::Aromatic),
        b'/' => Token::Bond(BondKind::Up),
        b'\\' => Token::Bond(BondKind::Down),
        // Branches
        b'(' => Token::BranchOpen,
        b')' => Token::BranchClose,
        // Ring closures
        d @ b'0'..=b'9' => Token::RingClosure(d - b'0'),
        b'%' => {
            let tens = scanner.take_digit().ok_or_else(|| scanner.unexpected())?;
            let ones = scanner.take_digit().ok_or_else(|| scanner.unexpected())?;
            Token::RingClosure(10 * tens + ones)
        }
        // Fragment separators
        b'.' => Token::Dot,
        // Other...
        _ => {
            scanner.cursor = start; // un-pop the bad char so `unexpected` sees it
            return Err(scanner.unexpected());
        }
    };

    Ok(Some(token))
}

fn read_organic_atom(scanner: &mut Scanner<'_>, c: u8, start: usize) -> Atom {
    match c {
        b'B' => scanner.take_eq(b'r'),
        b'C' => scanner.take_eq(b'l'),
        _ => false,
    };
    atom_from_symbol(scanner.slice(start)).expect("organic subset symbols should all be elements")
}

fn atom_from_symbol(symbol: &str) -> Option<Atom> {
    // Handle wildcard atoms
    if symbol == "*" {
        return Some(Atom {
            atomic_number: 0,
            aromatic: false,
        });
    }
    // Determine aromaticity
    let (symbol, aromatic) = match aromatic_symbol(symbol) {
        Some(upper) => (upper, true),
        _ => (symbol, false),
    };
    // Check that the element is valid
    let element = Element::from_symbol(symbol)?;
    Some(Atom {
        atomic_number: element.atomic_number,
        aromatic,
    })
}

fn aromatic_symbol(symbol: &str) -> Option<&'static str> {
    match symbol {
        "b" => Some("B"),
        "c" => Some("C"),
        "n" => Some("N"),
        "o" => Some("O"),
        "p" => Some("P"),
        "s" => Some("S"),
        "se" => Some("Se"),
        "as" => Some("As"),
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct Scanner<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> Scanner<'a> {
    fn new(input: &'a str) -> Self {
        Scanner { input, cursor: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.cursor).copied()
    }

    fn peek_char(&self) -> Option<char> {
        self.input.get(self.cursor..)?.chars().next()
    }

    fn pop(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.cursor += 1;
        Some(c)
    }

    fn take_if(&mut self, predicate: impl Fn(u8) -> bool) -> Option<u8> {
        let c = self.peek().filter(|&c| predicate(c))?;
        self.cursor += 1;
        Some(c)
    }

    fn take_while(&mut self, predicate: impl Fn(u8) -> bool) -> &'a str {
        let start = self.cursor;
        while self.take_if(&predicate).is_some() {}
        self.slice(start)
    }

    fn take_eq(&mut self, expected: u8) -> bool {
        self.take_if(|c| c == expected).is_some()
    }

    fn take_str(&mut self, s: &str) -> bool {
        let matches = self.input[self.cursor..].starts_with(s);
        if matches {
            self.cursor += s.len();
        }
        matches
    }

    fn take_digit(&mut self) -> Option<u8> {
        self.take_if(|c| c.is_ascii_digit()).map(|d| d - b'0')
    }

    fn slice(&self, start: usize) -> &'a str {
        &self.input[start..self.cursor]
    }

    // Errors
    fn unexpected(&self) -> LexError {
        let kind = self.peek_char().map_or(
            LexErrorKind::UnexpectedEnd,
            LexErrorKind::UnexpectedCharacter,
        );
        LexError::new(self.cursor, kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use LexErrorKind::*;

    fn atom(atomic_number: u8) -> Token {
        Token::Atom(Atom {
            atomic_number,
            aromatic: false,
        })
    }

    fn aromatic(atomic_number: u8) -> Token {
        Token::Atom(Atom {
            atomic_number,
            aromatic: true,
        })
    }

    fn bond(kind: BondKind) -> Token {
        Token::Bond(kind)
    }

    // Atoms

    #[test]
    fn organic_atoms() {
        for (smiles, expected) in [
            ("B", vec![atom(5)]),
            ("C", vec![atom(6)]),
            ("N", vec![atom(7)]),
            ("O", vec![atom(8)]),
            ("P", vec![atom(15)]),
            ("S", vec![atom(16)]),
            ("F", vec![atom(9)]),
            ("I", vec![atom(53)]),
            ("Cl", vec![atom(17)]),
            ("Br", vec![atom(35)]),
            // `B`/`C` followed by anything other than `r`/`l` stand alone
            ("BC", vec![atom(5), atom(6)]),
            ("CC", vec![atom(6), atom(6)]),
            ("b", vec![aromatic(5)]),
            ("c", vec![aromatic(6)]),
            ("n", vec![aromatic(7)]),
            ("o", vec![aromatic(8)]),
            ("p", vec![aromatic(15)]),
            ("s", vec![aromatic(16)]),
            // Wildcard
            ("*", vec![atom(0)]),
            // Outside brackets these are two atoms, not scandium or caesium
            ("Sc", vec![atom(16), aromatic(6)]),
            ("Cs", vec![atom(6), aromatic(16)]),
        ] {
            assert_eq!(lex(smiles), Ok(expected), "{smiles}");
        }
    }

    // Bonds

    #[test]
    fn bonds() {
        for (smiles, expected) in [
            ("-", BondKind::Single),
            ("=", BondKind::Double),
            ("#", BondKind::Triple),
            ("$", BondKind::Quadruple),
            (":", BondKind::Aromatic),
            ("/", BondKind::Up),
            ("\\", BondKind::Down),
        ] {
            assert_eq!(lex(smiles), Ok(vec![bond(expected)]), "{smiles}");
        }
    }

    // Branches

    #[test]
    fn branches() {
        assert_eq!(lex("()"), Ok(vec![Token::BranchOpen, Token::BranchClose]));
        assert_eq!(
            lex("CC(=O)O"),
            Ok(vec![
                atom(6),
                atom(6),
                Token::BranchOpen,
                bond(BondKind::Double),
                atom(8),
                Token::BranchClose,
                atom(8),
            ])
        );
    }

    // Ring closures

    #[test]
    fn ring_closures() {
        for (smiles, expected) in [
            ("0", vec![0]),
            ("1", vec![1]),
            ("9", vec![9]),
            ("12", vec![1, 2]),
            ("%00", vec![0]),
            ("%12", vec![12]),
            ("%99", vec![99]),
            // `%` takes exactly two digits
            ("%123", vec![12, 3]),
        ] {
            let expected = expected.into_iter().map(Token::RingClosure).collect();
            assert_eq!(lex(smiles), Ok(expected), "{smiles}");
        }
    }

    // Fragment separators

    #[test]
    fn fragment_separators() {
        for (smiles, expected) in [
            (".", vec![Token::Dot]),
            ("C.C", vec![atom(6), Token::Dot, atom(6)]),
        ] {
            assert_eq!(lex(smiles), Ok(expected), "{smiles}");
        }
    }

    // Whitespace termination

    #[test]
    fn whitespace_termination() {
        for (smiles, expected) in [
            ("CC O", vec![atom(6), atom(6)]),
            ("C\tX", vec![atom(6)]),
            ("C\n", vec![atom(6)]),
            ("C", vec![atom(6)]),
        ] {
            assert_eq!(lex(smiles), Ok(expected), "{smiles}");
        }
    }

    // Whole SMILES

    #[test]
    fn whole_smiles() {
        assert_eq!(lex(""), Ok(vec![]));
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

        let ring = |n| Token::RingClosure(n);
        let c = aromatic(6);
        assert_eq!(
            lex("c1ccccc1"),
            Ok(vec![c, ring(1), c, c, c, c, c, ring(1)])
        );

        // L-alanine
        let alpha = Token::BracketAtom(BracketAtom {
            isotope: None,
            atomic_number: 6,
            aromatic: false,
            chirality: Some(Chirality::Clockwise),
            hydrogens: 1,
            charge: 0,
            class: None,
        });
        assert_eq!(
            lex("N[C@@H](C)C(=O)O"),
            Ok(vec![
                atom(7),
                alpha,
                Token::BranchOpen,
                atom(6),
                Token::BranchClose,
                atom(6),
                Token::BranchOpen,
                bond(BondKind::Double),
                atom(8),
                Token::BranchClose,
                atom(8),
            ])
        );
    }

    // Error

    #[test]
    fn lex_error_kinds() {
        for (smiles, position, kind) in [
            ("X", 0, UnexpectedCharacter('X')),
            ("A", 0, UnexpectedCharacter('A')),
            ("@", 0, UnexpectedCharacter('@')),
            ("H", 0, UnexpectedCharacter('H')),
            ("CX", 1, UnexpectedCharacter('X')),
            ("Cé", 1, UnexpectedCharacter('é')),
            ("%", 1, UnexpectedEnd),
            ("C%", 2, UnexpectedEnd),
            ("C%1", 3, UnexpectedEnd),
            ("C%1X", 3, UnexpectedCharacter('X')),
        ] {
            assert_eq!(lex(smiles), Err(LexError { position, kind }), "{smiles}");
        }
    }

    #[test]
    fn error_messages() {
        for (smiles, message) in [
            ("CX", "unexpected character 'X' at position 1"),
            ("C%", "unexpected end of input at position 2"),
            ("[Xx]", "unknown element \"Xx\" at position 1"),
            ("[C@TH3]", "number out of range at position 5"),
        ] {
            assert_eq!(lex(smiles).unwrap_err().to_string(), message, "{smiles}");
        }
    }

    #[test]
    fn lex_error_is_a_good_error() {
        fn assert_error<E: std::error::Error + Send + Sync + 'static>() {}
        assert_error::<LexError>();
    }

    // Lexer
    #[test]
    fn lexeme_texts_and_positions() {
        let lexemes: Vec<_> = Lexer::new("C[13CH4]%12Cl")
            .map(|result| result.expect("valid SMILES"))
            .map(|lexeme| (lexeme.text, lexeme.position))
            .collect();
        assert_eq!(lexemes, [("C", 0), ("[13CH4]", 1), ("%12", 8), ("Cl", 11)]);
    }

    #[test]
    fn lexer_stops_after_error() {
        let mut lexer = Lexer::new("CXC");
        assert!(matches!(lexer.next(), Some(Ok(_))));
        assert!(matches!(lexer.next(), Some(Err(_))));
        assert!(lexer.next().is_none());
        assert!(lexer.next().is_none());
    }

    #[test]
    fn lexer_stops_after_whitespace() {
        let mut lexer = Lexer::new("C C");
        assert!(matches!(lexer.next(), Some(Ok(_))));
        assert!(lexer.next().is_none());
        assert!(lexer.next().is_none());
        assert!(lexer.next().is_none());
    }

    #[test]
    fn lexemes_outlive_lexer() {
        let smiles = String::from("CCO");
        let lexemes: Vec<_> = {
            let lexer = Lexer::new(&smiles);
            lexer.map(|result| result.expect("valid SMILES")).collect()
        };
        assert_eq!(lexemes.len(), 3);
        let texts: Vec<_> = lexemes.iter().map(|l| l.text).collect();
        assert_eq!(texts, vec!["C", "C", "O"]);
    }
}
