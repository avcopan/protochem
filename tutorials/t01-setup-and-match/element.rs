//! Chemical elements: symbols, atomic numbers, and the SMILES valence model.
//!
//! Tutorial 01 exercise. Replace every `todo!()` so that `cargo test` passes.
//! This module comes back in tutorials 02 and 05, where the SMILES lexer and parser
//! use it to recognize atoms and add implicit hydrogens.

/// Return the symbol for an atomic number, for the first 18 elements (H to Ar).
///
/// Use a single `match` expression. Return `None` for anything outside 1..=18.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(symbol(6), Some("C"));
/// assert_eq!(symbol(0), None);
/// ```
#[must_use]
pub fn symbol(atomic_number: u8) -> Option<&'static str> {
    todo!()
}

/// Return the atomic number for a symbol, for the first 18 elements.
///
/// Symbols are case-sensitive, so `"Cl"` is chlorine but `"CL"` and `"cl"` are not.
/// (Hint: you can `match` on a `&str` directly.)
#[must_use]
pub fn atomic_number(symbol: &str) -> Option<u8> {
    todo!()
}

/// Return the allowed valences of an element in the SMILES "organic subset".
///
/// The organic subset contains the elements that may be written without square
/// brackets in SMILES, each with its allowed valences (OpenSMILES spec, section 3.1.5):
///
/// | element        | valences  |
/// |----------------|-----------|
/// | B              | 3         |
/// | C              | 4         |
/// | N, P           | 3, 5      |
/// | O              | 2         |
/// | S              | 2, 4, 6   |
/// | F, Cl, Br, I   | 1         |
///
/// Return an empty slice for every other element.
///
/// Hint: match on `atomic_number`. A match arm can cover several patterns at once
/// (`7 | 15 => ...`). The return type is a slice with a `'static` lifetime, so you
/// can return array literals such as `&[3, 5]` directly.
#[must_use]
pub fn organic_valences(atomic_number: u8) -> &'static [u8] {
    todo!()
}

/// Return `true` if the element is in the SMILES organic subset.
///
/// Write this in one line using [`organic_valences`].
#[must_use]
pub fn is_organic(atomic_number: u8) -> bool {
    todo!()
}

/// Return the number of implicit hydrogens for an organic-subset atom without
/// brackets, given the sum of its explicit bond orders.
///
/// The SMILES rule is: take the *smallest* allowed valence that is greater than or
/// equal to the bond order sum. The implicit hydrogen count is that valence minus
/// the bond order sum. If no allowed valence is large enough (hypervalent), the
/// count is 0.
///
/// Return `None` if the element is not in the organic subset (bracket atoms never
/// get implicit hydrogens; they say how many they have, as in `[CH4]`).
///
/// Examples:
/// - carbon (C) with bond order sum 1, as in the CH3 groups of `CC`: valence 4 → 3 hydrogens
/// - nitrogen (N) with bond order sum 4: valences are 3 and 5, so 5 → 1 hydrogen
/// - sulfur (S) with bond order sum 3: valences 2, 4, 6 → 4 → 1 hydrogen
/// - fluorine (F) with bond order sum 2: hypervalent → 0 hydrogens
///
/// Hint: use `let ... else` to return early when the valence list is empty, then a
/// `for` loop over the valences. (Tutorial 06 shows how to do this with iterators.)
#[must_use]
pub fn implicit_hydrogens(atomic_number: u8, bond_order_sum: u8) -> Option<u8> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_known() {
        assert_eq!(symbol(1), Some("H"));
        assert_eq!(symbol(6), Some("C"));
        assert_eq!(symbol(17), Some("Cl"));
        assert_eq!(symbol(18), Some("Ar"));
    }

    #[test]
    fn symbol_unknown() {
        assert_eq!(symbol(0), None);
        assert_eq!(symbol(19), None);
        assert_eq!(symbol(255), None);
    }

    #[test]
    fn atomic_number_known() {
        assert_eq!(atomic_number("H"), Some(1));
        assert_eq!(atomic_number("N"), Some(7));
        assert_eq!(atomic_number("Cl"), Some(17));
    }

    #[test]
    fn atomic_number_is_case_sensitive() {
        assert_eq!(atomic_number("CL"), None);
        assert_eq!(atomic_number("cl"), None);
        assert_eq!(atomic_number(""), None);
    }

    #[test]
    fn symbol_and_atomic_number_round_trip() {
        for z in 1..=18 {
            let s = symbol(z).expect("symbol exists for 1..=18");
            assert_eq!(atomic_number(s), Some(z), "round trip failed for {s}");
        }
    }

    #[test]
    fn organic_subset() {
        let organic: Vec<u8> = (1..=18).filter(|&z| is_organic(z)).collect();
        // B, C, N, O, F, P, S, Cl (Br = 35 and I = 53 are beyond 18 but also organic)
        assert_eq!(organic, vec![5, 6, 7, 8, 9, 15, 16, 17]);
        assert!(is_organic(35));
        assert!(is_organic(53));
        assert!(!is_organic(1)); // hydrogen must be written as [H]
    }

    #[test]
    fn valences() {
        assert_eq!(organic_valences(6), &[4]);
        assert_eq!(organic_valences(7), &[3, 5]);
        assert_eq!(organic_valences(16), &[2, 4, 6]);
        assert!(organic_valences(26).is_empty()); // iron
    }

    #[test]
    fn implicit_hydrogens_carbon() {
        assert_eq!(implicit_hydrogens(6, 0), Some(4)); // methane, "C"
        assert_eq!(implicit_hydrogens(6, 1), Some(3)); // ethane, "CC"
        assert_eq!(implicit_hydrogens(6, 4), Some(0)); // neopentane center
        assert_eq!(implicit_hydrogens(6, 5), Some(0)); // hypervalent
    }

    #[test]
    fn implicit_hydrogens_multiple_valences() {
        assert_eq!(implicit_hydrogens(7, 3), Some(0)); // N with 3 bonds
        assert_eq!(implicit_hydrogens(7, 4), Some(1)); // N jumps to valence 5
        assert_eq!(implicit_hydrogens(16, 3), Some(1)); // S jumps to valence 4
        assert_eq!(implicit_hydrogens(16, 5), Some(1)); // S jumps to valence 6
    }

    #[test]
    fn implicit_hydrogens_not_organic() {
        assert_eq!(implicit_hydrogens(26, 0), None);
        assert_eq!(implicit_hydrogens(1, 0), None);
    }
}
