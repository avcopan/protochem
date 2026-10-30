use super::{Atom, LexError, LexErrorKind, Scanner, atom_from_symbol};

/// The contents of a bracket atom, e.g. `[13CH3+:2]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BracketAtom {
    /// The mass number, e.g. `13` in `[13C]`.
    pub isotope: Option<u16>,
    /// The atomic number, or `0` for the wildcard `*`.
    pub atomic_number: u8,
    /// Whether the symbol was lowercase, e.g. `[nH]`.
    pub aromatic: bool,
    /// The chirality marker, e.g. `@@` in `[C@@H]`.
    pub chirality: Option<Chirality>,
    /// The hydrogen count, e.g. `3` in `[CH3]`.
    pub hydrogens: u8,
    /// The formal charge, e.g. `-1` in `[O-]`.
    pub charge: i8,
    /// The atom class, e.g. `7` in `[CH3:7]`.
    pub class: Option<u16>,
}

/// A chirality marker inside brackets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chirality {
    /// `@`: `@TH1` or `@AL1`, depending on the center
    CounterClockwise,
    /// `@@`: `@TH2` or `@AL2`, depending on the center
    Clockwise,
    /// `@TH1`–`@TH2`
    Tetrahedral(u8),
    /// `@AL1`–`@AL2`
    Allene(u8),
    /// `@SP1`–`@SP3`
    SquarePlanar(u8),
    /// `@TB1`–`@TB20`
    TrigonalBipyramidal(u8),
    /// `@OH1`–`@OH30`
    Octahedral(u8),
}

pub(super) fn read_bracket_atom(scanner: &mut Scanner<'_>) -> Result<BracketAtom, LexError> {
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
        Some(read_number(scanner)?.ok_or_else(|| scanner.unexpected())?)
    } else {
        None
    };
    // Read the closing bracket
    if !scanner.take_eq(b']') {
        return Err(scanner.unexpected());
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

fn read_number<T: std::str::FromStr>(scanner: &mut Scanner<'_>) -> Result<Option<T>, LexError> {
    let start = scanner.cursor;
    let digits = scanner.take_while(|c| c.is_ascii_digit());
    if digits.is_empty() {
        return Ok(None);
    }
    digits
        .parse()
        .map(Some)
        .map_err(|_| LexError::new(start, LexErrorKind::NumberOutOfRange))
}

fn read_symbol(scanner: &mut Scanner<'_>) -> Result<Atom, LexError> {
    let start = scanner.cursor;
    scanner.take_if(|c| c.is_ascii_alphabetic() || c == b'*');
    scanner.take_if(|c| c.is_ascii_lowercase());
    let symbol = scanner.slice(start);
    if symbol.is_empty() {
        return Err(scanner.unexpected());
    }

    let Some(atom) = atom_from_symbol(symbol) else {
        return Err(LexError::new(
            start,
            LexErrorKind::UnknownElement(symbol.to_owned()),
        ));
    };
    Ok(atom)
}

// Prefix, constructor, largest number
type ChiralClass = (&'static str, fn(u8) -> Chirality, u8);

const CLASSES: [ChiralClass; 5] = [
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
    let start = scanner.cursor;
    let n = read_number(scanner)?.ok_or_else(|| scanner.unexpected())?;
    if !(1..=max).contains(&n) {
        return Err(LexError::new(start, LexErrorKind::NumberOutOfRange));
    }
    Ok(Some(make(n)))
}

fn read_charge(scanner: &mut Scanner<'_>) -> Result<i8, LexError> {
    let Some(first) = scanner.take_if(|c| c == b'+' || c == b'-') else {
        return Ok(0);
    };
    let sign = if first == b'+' { 1 } else { -1 };
    let magnitude = if scanner.take_eq(first) {
        2
    } else {
        let start = scanner.cursor;
        match read_number::<i8>(scanner)? {
            None => 1,
            Some(n) if n <= 15 => n,
            Some(_) => return Err(LexError::new(start, LexErrorKind::NumberOutOfRange)),
        }
    };
    Ok(sign * magnitude)
}

#[cfg(test)]
mod tests {
    use super::super::{Token, lex};
    use super::*;
    use LexErrorKind::*;

    /// Lexes `smiles`, which must be exactly one bracket atom.
    fn bracket(smiles: &str) -> BracketAtom {
        match lex(smiles).as_deref() {
            Ok([Token::BracketAtom(atom)]) => *atom,
            other => panic!("{smiles:?}: expected one bracket atom, got {other:?}"),
        }
    }

    /// A bracket atom with every optional field empty.
    fn plain(atomic_number: u8) -> BracketAtom {
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

    #[test]
    fn isotope() {
        for (smiles, expected) in [
            ("[13C]", Some(13)),
            // `H` here is the element, not a hydrogen count
            ("[2H]", Some(2)),
            ("[238U]", Some(238)),
            ("[C]", None),
            ("[2*]", Some(2)),
        ] {
            assert_eq!(bracket(smiles).isotope, expected, "{smiles}");
        }
        assert_eq!(bracket("[2H]").atomic_number, 1);
    }

    #[test]
    fn symbol() {
        let aromatic = |atomic_number| BracketAtom {
            aromatic: true,
            ..plain(atomic_number)
        };
        for (smiles, expected) in [
            ("[Fe]", plain(26)),
            ("[Cl]", plain(17)),
            // Scandium, not S + aromatic c as outside brackets
            ("[Sc]", plain(21)),
            ("[H]", plain(1)),
            ("[c]", aromatic(6)),
            ("[se]", aromatic(34)),
            ("[as]", aromatic(33)),
            (
                "[nH]",
                BracketAtom {
                    hydrogens: 1,
                    ..aromatic(7)
                },
            ),
            ("[*]", plain(0)),
        ] {
            assert_eq!(bracket(smiles), expected, "{smiles}");
        }
    }

    #[test]
    fn chirality() {
        use Chirality::*;
        for (smiles, expected) in [
            ("[C]", None),
            ("[C@]", Some(CounterClockwise)),
            ("[C@@]", Some(Clockwise)),
            // `H` after `@` is a hydrogen count, not a class
            ("[C@H]", Some(CounterClockwise)),
            ("[C@@H]", Some(Clockwise)),
            ("[C@TH1]", Some(Tetrahedral(1))),
            ("[C@TH2]", Some(Tetrahedral(2))),
            ("[C@AL1]", Some(Allene(1))),
            ("[C@AL2]", Some(Allene(2))),
            ("[Pt@SP1]", Some(SquarePlanar(1))),
            ("[Pt@SP3]", Some(SquarePlanar(3))),
            ("[As@TB1]", Some(TrigonalBipyramidal(1))),
            ("[As@TB20]", Some(TrigonalBipyramidal(20))),
            ("[Co@OH1]", Some(Octahedral(1))),
            ("[Co@OH30]", Some(Octahedral(30))),
        ] {
            assert_eq!(bracket(smiles).chirality, expected, "{smiles}");
        }
    }

    #[test]
    fn hydrogens() {
        for (smiles, expected) in [
            ("[C]", 0),
            ("[CH]", 1),
            ("[CH4]", 4),
            ("[C@@H3]", 3),
            ("[*H]", 1),
        ] {
            assert_eq!(bracket(smiles).hydrogens, expected, "{smiles}");
        }
    }

    #[test]
    fn charge() {
        for (smiles, expected) in [
            ("[C]", 0),
            ("[NH4+]", 1),
            ("[O-]", -1),
            ("[Fe+2]", 2),
            ("[O-2]", -2),
            ("[Fe++]", 2),
            ("[O--]", -2),
            ("[Ti+12]", 12),
            ("[C+15]", 15),
            ("[C-15]", -15),
        ] {
            assert_eq!(bracket(smiles).charge, expected, "{smiles}");
        }
    }

    #[test]
    fn class() {
        for (smiles, expected) in [
            ("[C]", None),
            ("[CH3:7]", Some(7)),
            ("[O:0]", Some(0)),
            ("[*:123]", Some(123)),
        ] {
            assert_eq!(bracket(smiles).class, expected, "{smiles}");
        }
    }

    #[test]
    fn all_fields() {
        assert_eq!(
            bracket("[13C@@H2+2:7]"),
            BracketAtom {
                isotope: Some(13),
                chirality: Some(Chirality::Clockwise),
                hydrogens: 2,
                charge: 2,
                class: Some(7),
                ..plain(6)
            }
        );
    }

    #[test]
    fn errors() {
        for (smiles, position, kind) in [
            // Structure
            ("[C", 2, UnexpectedEnd),
            ("[]", 1, UnexpectedCharacter(']')),
            ("[CH4", 4, UnexpectedEnd),
            // Symbol
            ("[Xx]", 1, UnknownElement("Xx".to_owned())),
            ("[13]", 3, UnexpectedCharacter(']')),
            ("[*c]", 1, UnknownElement("*c".to_owned())),
            // A hydrogen count is one digit
            ("[CH12]", 4, UnexpectedCharacter('2')),
            // A class needs a number
            ("[CH3:]", 5, UnexpectedCharacter(']')),
            // A chirality class needs a number
            ("[C@TH]", 5, UnexpectedCharacter(']')),
            ("[C@SP]", 5, UnexpectedCharacter(']')),
            // Number out of range
            ("[99999C]", 1, NumberOutOfRange),
            ("[C+999]", 3, NumberOutOfRange),
            // A chirality class needs a number in range
            ("[C@TH0]", 5, NumberOutOfRange),
            ("[C@TH3]", 5, NumberOutOfRange),
            ("[C@AL3]", 5, NumberOutOfRange),
            ("[Pt@SP4]", 6, NumberOutOfRange),
            ("[As@TB21]", 6, NumberOutOfRange),
            ("[Co@OH31]", 6, NumberOutOfRange),
            // A repeated sign takes no count
            ("[C++2]", 4, UnexpectedCharacter('2')),
            // No more than two signs
            ("[C+++]", 4, UnexpectedCharacter('+')),
            // Max charge of 15
            ("[C+16]", 3, NumberOutOfRange),
            ("[C-16]", 3, NumberOutOfRange),
        ] {
            assert_eq!(lex(smiles), Err(LexError { position, kind }), "{smiles}");
        }
    }
}
