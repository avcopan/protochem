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
