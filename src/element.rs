//! Chemical elements and their properties.
//!
//! The element table itself is generated (see `scripts/elements_data.py`). An
//! [`Element`] can only be obtained by looking it up in that table, so every
//! `&Element` refers to a real element.

mod data;
use data::ELEMENTS;

/// A chemical element.
///
/// Look one up with [`Element::from_symbol`] or [`Element::from_atomic_number`].
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub struct Element {
    /// Atomic number.
    pub atomic_number: u8,
    /// Chemical symbol, e.g. `"Cl"`.
    pub symbol: &'static str,
    /// Mass number of the most abundant (or longest-lived) isotope.
    pub mass_number: u16,
    /// Mass of that isotope, in daltons.
    pub mass: f64,
    /// Group number, or `None` for the lanthanides and actinides.
    pub group: Option<u8>,
    /// Period number.
    pub period: u8,
    /// Pyykkö single-bond covalent radius, in angstroms.
    pub covalent_radius: f64,
    /// Number of valence electrons.
    pub valence_electrons: u8,
    /// Pauling electronegativity, if defined.
    pub electronegativity: Option<f64>,
}

impl Element {
    /// Look up an element by symbol.
    ///
    /// Symbols are case-sensitive, as in SMILES: `"Cl"` is chlorine, but `"CL"` and
    /// `"cl"` are not elements. Returns `None` for anything that isn't a symbol.
    ///
    /// # Examples
    ///
    /// ```
    /// use protochem::Element;
    ///
    /// let carbon = Element::from_symbol("C").expect("carbon is in the table");
    /// assert_eq!(carbon.atomic_number, 6);
    /// assert!(Element::from_symbol("CL").is_none());
    /// ```
    #[must_use]
    pub fn from_symbol(symbol: &str) -> Option<&'static Self> {
        ELEMENTS.iter().find(|e| e.symbol == symbol)
    }

    /// Look up an element by atomic number.
    ///
    /// Returns `None` for 0 and for atomic numbers past the end of the table.
    ///
    /// # Examples
    ///
    /// ```
    /// use protochem::Element;
    ///
    /// let carbon = Element::from_atomic_number(6).expect("carbon is in the table");
    /// assert_eq!(carbon.symbol, "C");
    /// ```
    #[must_use]
    pub fn from_atomic_number(atomic_number: u8) -> Option<&'static Self> {
        let index = usize::from(atomic_number).checked_sub(1)?;
        ELEMENTS.get(index)
    }
}

/// Return the allowed valences of an element in the SMILES "organic subset".
///
/// The organic subset contains the elements that may be written without square
/// brackets in SMILES, each with its allowed valences ([OpenSMILES spec, section
/// 3.1.5](http://opensmiles.org/opensmiles.html#orgsbst)):
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
/// Returns an empty slice for every other element, including invalid atomic numbers.
///
/// # Examples
///
/// ```
/// use protochem::element;
///
/// assert_eq!(element::organic_valences(7), &[3, 5]); // nitrogen
/// assert!(element::organic_valences(26).is_empty()); // iron must be written [Fe]
/// ```
#[must_use]
pub fn organic_valences(atomic_number: u8) -> &'static [u8] {
    match atomic_number {
        5 => &[3],                // B
        6 => &[4],                // C
        7 | 15 => &[3, 5],        // N P
        8 => &[2],                // O
        16 => &[2, 4, 6],         // S
        9 | 17 | 35 | 53 => &[1], // F Cl Br I
        _ => &[],
    }
}

/// Return whether an element is in the SMILES "organic subset" (see [`organic_valences`]).
///
/// # Examples
///
/// ```
/// use protochem::element;
///
/// assert!(element::is_organic(6));
/// assert!(!element::is_organic(26));
/// ```
#[must_use]
pub fn is_organic(atomic_number: u8) -> bool {
    !organic_valences(atomic_number).is_empty()
}

/// Return the number of implicit hydrogens on an organic-subset atom written without
/// brackets.
///
/// `bond_order_sum` is the sum of the orders of the atom's explicit bonds (its explicit
/// valence). The SMILES rule takes the smallest allowed valence (see
/// [`organic_valences`]) that is at least `bond_order_sum` and fills the difference
/// with hydrogens. If no valence is large enough, the atom gets no hydrogens.
///
/// Returns `None` if the element is not in the organic subset, since bracket atoms
/// never get implicit hydrogens: they say how many they have, as in `[CH4]`.
///
/// # Examples
///
/// ```
/// use protochem::element;
///
/// assert_eq!(element::implicit_hydrogens(6, 1), Some(3)); // C in CC: valence 4
/// assert_eq!(element::implicit_hydrogens(7, 4), Some(1)); // N: valence 5
/// assert_eq!(element::implicit_hydrogens(9, 2), Some(0)); // F: hypervalent
/// assert_eq!(element::implicit_hydrogens(26, 0), None); // Fe must be written [Fe]
/// ```
#[must_use]
pub fn implicit_hydrogens(atomic_number: u8, bond_order_sum: u8) -> Option<u8> {
    let valences = organic_valences(atomic_number);
    let [_, ..] = valences else { return None };
    let valence = valences.iter().copied().find(|&v| v >= bond_order_sum);
    Some(valence.map_or(0, |v| v - bond_order_sum))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_ordered_by_atomic_number() {
        for (index, element) in ELEMENTS.iter().enumerate() {
            assert_eq!(usize::from(element.atomic_number), index + 1);
        }
    }

    #[test]
    fn lookups_round_trip() {
        for atomic_number in 1..=118 {
            let element = Element::from_atomic_number(atomic_number).expect("z is in 1..=118");
            assert_eq!(Element::from_symbol(element.symbol), Some(element));
        }
    }

    #[test]
    fn from_atomic_number_out_of_range() {
        assert_eq!(Element::from_atomic_number(0), None);
        assert_eq!(Element::from_atomic_number(119), None);
        assert_eq!(Element::from_atomic_number(255), None);
    }

    #[test]
    fn from_symbol_is_case_sensitive() {
        assert_eq!(
            Element::from_symbol("Cl").map(|e| e.atomic_number),
            Some(17)
        );
        assert_eq!(Element::from_symbol("CL"), None);
        assert_eq!(Element::from_symbol("cl"), None);
        assert_eq!(Element::from_symbol(""), None);
    }

    #[test]
    fn organic_valences_table() {
        const NOT_ORGANIC: &[u8] = &[];
        assert_eq!(organic_valences(5), &[3]); // B
        assert_eq!(organic_valences(6), &[4]); // C
        assert_eq!(organic_valences(7), &[3, 5]); // N
        assert_eq!(organic_valences(16), &[2, 4, 6]); // S
        assert_eq!(organic_valences(35), &[1]); // Br
        assert_eq!(organic_valences(53), &[1]); // I
        assert_eq!(organic_valences(1), NOT_ORGANIC); // H must be written [H]
        assert_eq!(organic_valences(26), NOT_ORGANIC); // Fe
        assert_eq!(organic_valences(0), NOT_ORGANIC);
    }

    #[test]
    fn organic_subset() {
        let organic: Vec<u8> = (1..=18).filter(|&z| is_organic(z)).collect();
        // B, C, N, O, F, P, S, Cl
        assert_eq!(organic, vec![5, 6, 7, 8, 9, 15, 16, 17]);
        assert!(is_organic(35));
        assert!(is_organic(53));
        assert!(!is_organic(1));
        assert!(!is_organic(26));
    }

    #[test]
    fn implicit_hydrogens_rule() {
        assert_eq!(implicit_hydrogens(6, 1), Some(3)); // C
        assert_eq!(implicit_hydrogens(7, 4), Some(1)); // N: 3 < 4 <= 5
        assert_eq!(implicit_hydrogens(16, 3), Some(1)); // S: 2 < 3 <= 4
        assert_eq!(implicit_hydrogens(16, 6), Some(0)); // S: exactly the top valence
        assert_eq!(implicit_hydrogens(9, 2), Some(0)); // F: hypervalent
        assert_eq!(implicit_hydrogens(26, 0), None); // Fe
        assert_eq!(implicit_hydrogens(0, 0), None);
    }

    #[test]
    fn implicit_hydrogens_unbonded_fill_lowest_valence() {
        for atomic_number in 1..=118 {
            let expected = organic_valences(atomic_number).first().copied();
            assert_eq!(implicit_hydrogens(atomic_number, 0), expected);
        }
    }
}
