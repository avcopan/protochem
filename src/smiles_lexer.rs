use crate::Element;

#[derive(Debug)]
pub struct Atom {
    pub atomic_number: u8,
    pub aromatic: bool,
}

#[derive(Debug)]
pub enum Chirality {
    CounterClockwise,
    Clockwise,
    Tetrahedral(u8),
    Allene(u8),
    SquarePlanar(u8),
    TrigonalBipyramidal(u8),
    Octahedral(u8),
}

#[derive(Debug)]
pub struct BracketAtom {
    pub isotope: Option<u16>,
    pub atomic_number: u8,
    pub aromatic: bool,
    pub chirality: Option<Chirality>,
    pub hydrogens: u8,
    pub charge: i8,
    pub class: Option<u16>,
}

#[derive(Debug)]
pub enum BondKind {
    Single,
    Double,
    Triple,
    Quadruple,
    Aromatic,
    Up,
    Down,
}

#[derive(Debug)]
pub enum Token {
    // Atoms
    Atom(Atom),
    BracketAtom(BracketAtom),
    // Bonds
    Bond(BondKind),
    // Branches
    BranchOpen,
    BranchClose,
    // Ring closures
    RingClosure(u8),
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

#[derive(Debug)]
pub enum LexError {
    GenericPlaceholder,
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

fn read_number<T: std::str::FromStr>(scanner: &mut Scanner<'_>) -> Result<Option<T>, LexError> {
    let digits = scanner.take_while(|c| c.is_ascii_digit());
    if digits.is_empty() {
        return Ok(None);
    }
    digits
        .parse()
        .map(Some)
        .map_err(|_| LexError::GenericPlaceholder)
}

fn read_symbol(scanner: &mut Scanner<'_>) -> Result<Atom, LexError> {
    let start = scanner.cursor;
    scanner.take_if(|c| c.is_ascii_alphabetic());
    scanner.take_if(|c| c.is_ascii_lowercase());
    let Some(atom) = atom_from_symbol(scanner.slice(start)) else {
        return Err(LexError::GenericPlaceholder);
    };
    Ok(atom)
}

const CLASSES: [(&str, fn(u8) -> Chirality, u8); 5] = [
    ("TH", Chirality::Tetrahedral, 2),
    ("AL", Chirality::Allene, 2),
    ("SP", Chirality::SquarePlanar, 3),
    ("TB", Chirality::TrigonalBipyramidal, 20),
    ("OH", Chirality::Octahedral, 30),
];

fn read_chirality(scanner: &mut Scanner<'_>) -> Result<Option<Chirality>, LexError> {
    // No marker -> None
    if !scanner.take_eq(b'@') {
        return Ok(None);
    }
    // @@ -> Clockwise
    if scanner.take_eq(b'@') {
        return Ok(Some(Chirality::Clockwise));
    }
    // No TH, AL, SP, TB, OH -> CounterClockwise
    let Some(&(_, make, max)) = CLASSES.iter().find(|(prefix, ..)| scanner.take_str(prefix)) else {
        return Ok(Some(Chirality::CounterClockwise));
    };
    // Handle TH, AL, SP, TB, OH
    let n = read_number(scanner)?.ok_or(LexError::GenericPlaceholder)?;
    if !(1..=max).contains(&n) {
        return Err(LexError::GenericPlaceholder);
    }
    Ok(Some(make(n)))
}

fn read_charge(scanner: &mut Scanner<'_>) -> Result<i8, LexError> {
    let (sign, symbol) = match scanner.peek() {
        Some(b'+') => (1, b'+'),
        Some(b'-') => (-1, b'-'),
        _ => return Ok(0),
    };
    let repeats = scanner.take_while(|c| c == symbol).len();
    let magnitude: i8 = match read_number(scanner)? {
        Some(n) if repeats == 1 => n,
        Some(_) => return Err(LexError::GenericPlaceholder),
        None => i8::try_from(repeats).map_err(|_| LexError::GenericPlaceholder)?,
    };

    Ok(sign * magnitude)
}

fn read_bracket_atom(scanner: &mut Scanner<'_>) -> Result<BracketAtom, LexError> {
    // Read the isotope
    let isotope = read_number(scanner)?;
    // Read the atomic symbol
    let atom = read_symbol(scanner)?;
    // Read chirality
    let chirality = read_chirality(scanner)?;
    // Read hydrogens
    let hydrogens = if scanner.take_eq(b'H') {
        scanner.take_digit().unwrap_or(1)
    } else {
        0
    };
    // Read charge
    let charge = read_charge(scanner)?;
    // Read class
    let class = if scanner.take_eq(b':') {
        Some(read_number(scanner)?.ok_or(LexError::GenericPlaceholder)?)
    } else {
        None
    };
    // Read the closing bracket
    if !scanner.take_eq(b']') {
        return Err(LexError::GenericPlaceholder);
    }

    Ok(BracketAtom {
        isotope,
        atomic_number: atom.atomic_number,
        aromatic: atom.aromatic,
        chirality,
        hydrogens,
        charge,
        class,
    })
}

fn next_token(scanner: &mut Scanner<'_>) -> Result<Option<Token>, LexError> {
    let start = scanner.cursor;
    let Some(c) = scanner.pop() else {
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
        // Other...
        _ => {
            return Err(LexError::GenericPlaceholder);
        }
    };

    Ok(Some(token))
}

pub fn lex(smiles: &str) -> Result<Vec<Token>, LexError> {
    let mut scanner = Scanner::new(smiles);
    let mut tokens = Vec::new();
    while let Some(token) = next_token(&mut scanner)? {
        tokens.push(token)
    }
    Ok(tokens)
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     fn atom(atomic_number: u8) -> Token {
//         Token::Atom {
//             atomic_number,
//             aromatic: false,
//         }
//     }

//     fn aromatic(atomic_number: u8) -> Token {
//         Token::Atom {
//             atomic_number,
//             aromatic: true,
//         }
//     }

//     // A plain bracket atom; set other fields with `BracketAtom { charge: -1, ..bracket_atom(8) }`
//     fn bracket_atom(atomic_number: u8) -> BracketAtom {
//         BracketAtom {
//             isotope: None,
//             atomic_number,
//             aromatic: false,
//             chirality: None,
//             hydrogens: 0,
//             charge: 0,
//             class: None,
//         }
//     }

//     fn bracket(atomic_number: u8, hydrogens: u8) -> Token {
//         Token::BracketAtom(BracketAtom {
//             hydrogens,
//             ..bracket_atom(atomic_number)
//         })
//     }

//     #[test]
//     fn bonds_and_branches() {
//         assert_eq!(lex(""), Ok(vec![]));

//         let tokens = vec![
//             Token::Bond(BondKind::Single),
//             Token::Bond(BondKind::Double),
//             Token::Bond(BondKind::Triple),
//             Token::Bond(BondKind::Quadruple),
//             Token::Bond(BondKind::Aromatic),
//             Token::Bond(BondKind::Up),
//             Token::Bond(BondKind::Down),
//             Token::BranchOpen,
//             Token::BranchClose,
//             Token::Dot,
//         ];
//         assert_eq!(lex("-=#$:/\\()."), Ok(tokens));
//     }

//     #[test]
//     fn organic_atoms() {
//         assert_eq!(lex("CCO"), Ok(vec![atom(6), atom(6), atom(8)]));
//         assert_eq!(
//             lex("BNPSFI"),
//             Ok(vec![
//                 atom(5),
//                 atom(7),
//                 atom(15),
//                 atom(16),
//                 atom(9),
//                 atom(53)
//             ])
//         );
//     }

//     #[test]
//     fn two_letter_symbols() {
//         assert_eq!(lex("ClCBr"), Ok(vec![atom(17), atom(6), atom(35)]));
//         // `B` followed by something other than `r` is boron
//         assert_eq!(lex("BC"), Ok(vec![atom(5), atom(6)]));
//     }

//     #[test]
//     fn aromatic_atoms() {
//         assert_eq!(
//             lex("bcnops"),
//             Ok(vec![
//                 aromatic(5),
//                 aromatic(6),
//                 aromatic(7),
//                 aromatic(8),
//                 aromatic(15),
//                 aromatic(16)
//             ])
//         );
//     }

//     #[test]
//     fn atoms_with_bonds_and_branches() {
//         assert_eq!(
//             lex("CC(=O)O"),
//             Ok(vec![
//                 atom(6),
//                 atom(6),
//                 Token::BranchOpen,
//                 Token::Bond(BondKind::Double),
//                 atom(8),
//                 Token::BranchClose,
//                 atom(8),
//             ])
//         );
//     }

//     #[test]
//     fn unexpected_character() {
//         assert_eq!(
//             lex("(X"),
//             Err(LexError::UnexpectedCharacter {
//                 position: 1,
//                 character: 'X'
//             })
//         );
//         assert_eq!(
//             lex("X("),
//             Err(LexError::UnexpectedCharacter {
//                 position: 0,
//                 character: 'X'
//             })
//         );
//     }

//     #[test]
//     fn ring_closures() {
//         let tokens = vec![
//             atom(6),
//             Token::RingClosure(1),
//             atom(6),
//             atom(6),
//             Token::RingClosure(1),
//         ];
//         assert_eq!(lex("C1CC1"), Ok(tokens));
//         assert_eq!(
//             lex("C%12C%12"),
//             Ok(vec![
//                 atom(6),
//                 Token::RingClosure(12),
//                 atom(6),
//                 Token::RingClosure(12)
//             ])
//         );
//         // `%` takes exactly two digits, so the `3` is a ring closure of its own
//         assert_eq!(
//             lex("%123"),
//             Ok(vec![Token::RingClosure(12), Token::RingClosure(3)])
//         );
//         assert_eq!(lex("c1ccccc1").map(|tokens| tokens.len()), Ok(8));
//     }

//     #[test]
//     fn ring_closure_errors() {
//         assert_eq!(lex("C%"), Err(LexError::UnexpectedEnd { position: 2 }));
//         assert_eq!(lex("C%1"), Err(LexError::UnexpectedEnd { position: 3 }));
//         assert_eq!(
//             lex("C%1X"),
//             Err(LexError::UnexpectedCharacter {
//                 position: 3,
//                 character: 'X'
//             })
//         );
//     }

//     #[test]
//     fn bracket_atoms() {
//         let tokens = vec![atom(6), bracket(6, 1), atom(6), atom(8)];
//         assert_eq!(lex("C[CH]CO"), Ok(tokens));

//         assert_eq!(lex("[Fe]"), Ok(vec![bracket(26, 0)]));
//         assert_eq!(lex("[Cl]"), Ok(vec![bracket(17, 0)]));
//         assert_eq!(lex("[CH4]"), Ok(vec![bracket(6, 4)]));
//         assert_eq!(
//             lex("[nH]"),
//             Ok(vec![Token::BracketAtom(BracketAtom {
//                 aromatic: true,
//                 hydrogens: 1,
//                 ..bracket_atom(7)
//             })])
//         );
//     }

//     #[test]
//     fn bracket_isotope_and_class() {
//         assert_eq!(
//             lex("[13C]"),
//             Ok(vec![Token::BracketAtom(BracketAtom {
//                 isotope: Some(13),
//                 ..bracket_atom(6)
//             })])
//         );
//         // The symbol `H` is hydrogen itself, not a hydrogen count
//         assert_eq!(
//             lex("[2H]"),
//             Ok(vec![Token::BracketAtom(BracketAtom {
//                 isotope: Some(2),
//                 ..bracket_atom(1)
//             })])
//         );
//         assert_eq!(
//             lex("[CH3:7]"),
//             Ok(vec![Token::BracketAtom(BracketAtom {
//                 hydrogens: 3,
//                 class: Some(7),
//                 ..bracket_atom(6)
//             })])
//         );
//     }

//     #[test]
//     fn bracket_chirality() {
//         let chiral = |chirality| {
//             Token::BracketAtom(BracketAtom {
//                 chirality: Some(chirality),
//                 hydrogens: 1,
//                 ..bracket_atom(6)
//             })
//         };
//         assert_eq!(lex("[C@H]"), Ok(vec![chiral(Chirality::CounterClockwise)]));
//         assert_eq!(lex("[C@@H]"), Ok(vec![chiral(Chirality::Clockwise)]));

//         let tokens = lex("N[C@](C)(F)C(=O)O").expect("alanine-like SMILES lexes");
//         assert_eq!(
//             tokens[1],
//             Token::BracketAtom(BracketAtom {
//                 chirality: Some(Chirality::CounterClockwise),
//                 ..bracket_atom(6)
//             })
//         );
//     }

//     #[test]
//     fn bracket_charges() {
//         let charge = |smiles: &str| match lex(smiles) {
//             Ok(tokens) => match &tokens[..] {
//                 [Token::BracketAtom(atom)] => atom.charge,
//                 _ => panic!("expected one bracket atom"),
//             },
//             Err(error) => panic!("lex error {error:?}"),
//         };
//         assert_eq!(charge("[C]"), 0);
//         assert_eq!(charge("[NH4+]"), 1);
//         assert_eq!(charge("[O-]"), -1);
//         assert_eq!(charge("[Fe+2]"), 2);
//         assert_eq!(charge("[O-2]"), -2);
//         assert_eq!(charge("[Fe++]"), 2);
//         assert_eq!(charge("[O--]"), -2);
//         assert_eq!(lex("[Na+].[Cl-]").map(|tokens| tokens.len()), Ok(3));
//     }

//     #[test]
//     fn bracket_all_fields() {
//         assert_eq!(
//             lex("[13C@@H2+2:7]"),
//             Ok(vec![Token::BracketAtom(BracketAtom {
//                 isotope: Some(13),
//                 chirality: Some(Chirality::Clockwise),
//                 hydrogens: 2,
//                 charge: 2,
//                 class: Some(7),
//                 ..bracket_atom(6)
//             })])
//         );
//     }

//     #[test]
//     fn bracket_errors() {
//         assert_eq!(
//             lex("[Xx]"),
//             Err(LexError::UnknownElement {
//                 position: 1,
//                 symbol: "Xx".to_string()
//             })
//         );
//         // The symbol starts after the isotope
//         assert_eq!(
//             lex("[13Xx]"),
//             Err(LexError::UnknownElement {
//                 position: 3,
//                 symbol: "Xx".to_string()
//             })
//         );
//         assert_eq!(lex("[C"), Err(LexError::UnexpectedEnd { position: 2 }));
//         assert_eq!(
//             lex("[]"),
//             Err(LexError::UnexpectedCharacter {
//                 position: 1,
//                 character: ']'
//             })
//         );
//         // A class needs a number after the `:`
//         assert_eq!(
//             lex("[CH3:]"),
//             Err(LexError::UnexpectedCharacter {
//                 position: 5,
//                 character: ']'
//             })
//         );
//         assert_eq!(
//             lex("[99999C]"),
//             Err(LexError::NumberOutOfRange { position: 1 })
//         );
//         assert_eq!(
//             lex("[C+999]"),
//             Err(LexError::NumberOutOfRange { position: 3 })
//         );
//     }

//     #[test]
//     fn error_messages() {
//         assert_eq!(
//             lex("CX").unwrap_err().to_string(),
//             "unexpected character 'X' at position 1"
//         );
//         assert_eq!(
//             lex("[CH4").unwrap_err().to_string(),
//             "unexpected end of input at position 4"
//         );
//         assert_eq!(
//             lex("[Xx]").unwrap_err().to_string(),
//             "unknown element \"Xx\" at position 1"
//         );
//         assert_eq!(
//             lex("[99999C]").unwrap_err().to_string(),
//             "number out of range at position 1"
//         );
//         // `LexError` works as a standard error
//         let _: Box<dyn std::error::Error> = Box::new(LexError::UnexpectedEnd { position: 0 });
//     }
// }
