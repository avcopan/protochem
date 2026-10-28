//! A SMILES lexer.
//!
//! [`lex`] splits a SMILES string into [`Token`]s, checking each token on its own. Whether the
//! tokens fit together (balanced branches, closed rings) is the parser's job.
//!
//! A bracket atom is `[isotope? symbol chirality? hcount? charge? class?]`.
use crate::Element;

mod bracket_atom;
use bracket_atom::read_bracket_atom;
pub use bracket_atom::{BracketAtom, Chirality};

/// An organic-subset atom written without brackets, e.g. `C` or `c`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Atom {
    /// The atomic number.
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

/// A lexing error.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LexError {
    /// Any error, until the specific variants are in.
    GenericPlaceholder,
}

/// Split a SMILES string into tokens.
///
/// # Errors
///
/// Returns a [`LexError`] if the input contains something that isn't a valid token, e.g. the
/// `X` in `CX`, an unknown element in `[Xx]`, or an out-of-range chirality class in `[C@TH3]`.
///
/// # Examples
///
/// ```
/// use protochem::smiles_lexer::{lex, Atom, Token};
///
/// let tokens = lex("CO").expect("valid SMILES");
/// let carbon = Atom { atomic_number: 6, aromatic: false };
/// let oxygen = Atom { atomic_number: 8, aromatic: false };
/// assert_eq!(tokens, [Token::Atom(carbon), Token::Atom(oxygen)]);
/// assert!(lex("CX").is_err());
/// ```
pub fn lex(smiles: &str) -> Result<Vec<Token>, LexError> {
    let mut scanner = Scanner::new(smiles);
    let mut tokens = Vec::new();
    while let Some(token) = next_token(&mut scanner)? {
        tokens.push(token);
    }
    Ok(tokens)
}

fn next_token(scanner: &mut Scanner<'_>) -> Result<Option<Token>, LexError> {
    let start = scanner.cursor;
    // Whitespace or scanner end -> None
    let Some(c) = scanner.pop().filter(|c| !c.is_ascii_whitespace()) else {
        return Ok(None);
    };

    let token = match c {
        // Atoms
        c if b"BCNOPSFIbcnops".contains(&c) => Token::Atom(read_organic_atom(scanner, c, start)?),
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
            let tens = scanner.take_digit().ok_or(LexError::GenericPlaceholder)?;
            let ones = scanner.take_digit().ok_or(LexError::GenericPlaceholder)?;
            Token::RingClosure(10 * tens + ones)
        }
        // Fragment separators
        b'.' => Token::Dot,
        // Other...
        _ => {
            return Err(LexError::GenericPlaceholder);
        }
    };

    Ok(Some(token))
}

fn read_organic_atom(scanner: &mut Scanner<'_>, c: u8, start: usize) -> Result<Atom, LexError> {
    match c {
        b'B' => scanner.take_eq(b'r'),
        b'C' => scanner.take_eq(b'l'),
        _ => false,
    };
    let Some(atom) = atom_from_symbol(scanner.slice(start)) else {
        return Err(LexError::GenericPlaceholder);
    };
    Ok(atom)
}

fn atom_from_symbol(symbol: &str) -> Option<Atom> {
    let (symbol, aromatic) = match aromatic_symbol(symbol) {
        Some(upper) => (upper, true),
        _ => (symbol, false),
    };
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

    fn take_while(&mut self, predicate: impl Fn(u8) -> bool) -> &str {
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

    fn slice(&self, start: usize) -> &str {
        &self.input[start..self.cursor]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            // Outside brackets these are two atoms, not scandium or caesium
            ("Sc", vec![atom(16), aromatic(6)]),
            ("Cs", vec![atom(6), aromatic(16)]),
        ] {
            assert_eq!(lex(smiles), Ok(expected), "{smiles}");
        }
    }

    #[test]
    fn organic_atom_errors() {
        for smiles in ["X", "CX", "A", "@", "H"] {
            assert!(lex(smiles).is_err(), "{smiles}");
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

    #[test]
    fn ring_closure_errors() {
        for smiles in ["%", "C%", "C%1", "C%1X"] {
            assert!(lex(smiles).is_err(), "{smiles}");
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
}
