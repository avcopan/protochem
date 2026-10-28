use super::{Atom, LexError, Scanner, atom_from_symbol};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chirality {
    CounterClockwise,
    Clockwise,
    Tetrahedral(u8),
    Allene(u8),
    SquarePlanar(u8),
    TrigonalBipyramidal(u8),
    Octahedral(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BracketAtom {
    pub isotope: Option<u16>,
    pub atomic_number: u8,
    pub aromatic: bool,
    pub chirality: Option<Chirality>,
    pub hydrogens: u8,
    pub charge: i8,
    pub class: Option<u16>,
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

#[cfg(test)]
mod tests {
    use super::super::{Token, lex};
    use super::*;

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
        ] {
            assert_eq!(bracket(smiles), expected, "{smiles}");
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
        ] {
            assert_eq!(bracket(smiles).isotope, expected, "{smiles}");
        }
        assert_eq!(bracket("[2H]").atomic_number, 1);
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
        for (smiles, expected) in [("[C]", 0), ("[CH]", 1), ("[CH4]", 4), ("[C@@H3]", 3)] {
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
        ] {
            assert_eq!(bracket(smiles).charge, expected, "{smiles}");
        }
    }

    #[test]
    fn class() {
        for (smiles, expected) in [("[C]", None), ("[CH3:7]", Some(7)), ("[O:0]", Some(0))] {
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
        for smiles in [
            // Structure
            "[C",
            "[]",
            "[CH4",
            // Symbol
            "[Xx]",
            "[13]",
            // Number out of range
            "[99999C]",
            "[C+999]",
            // A hydrogen count is one digit
            "[CH12]",
            // A repeated sign takes no count
            "[C++2]",
            // A class needs a number
            "[CH3:]",
            // A chirality class needs a number in range
            "[C@TH]",
            "[C@TH0]",
            "[C@TH3]",
            "[C@AL3]",
            "[Pt@SP4]",
            "[As@TB21]",
            "[Co@OH31]",
        ] {
            assert!(lex(smiles).is_err(), "{smiles}");
        }
    }
}
