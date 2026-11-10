//! A molecular graph with dense atom and bond indices.
use crate::element;
use std::collections::HashMap;
use std::fmt;

/// An undirected molecular graph without self-loops or multiple bonds.
///
/// Atoms carry an atomic number. Bonds carry nothing: bond orders, charges and
/// stereo belong in separate tables indexed by [`AtomIndex`] and [`BondIndex`].
///
/// # Examples
///
/// ```
/// use protochem::mol_graph::MolGraph;
///
/// let mut graph = MolGraph::new();
/// let oxygen = graph.add_atom(8).unwrap();
/// assert_eq!(graph.atomic_number(oxygen), Some(8));
/// assert_eq!(graph.atom_count(), 1);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MolGraph {
    atomic_numbers: Vec<u8>,
    bond_atoms: Vec<[AtomIndex; 2]>,
    adjacency: Vec<Vec<(AtomIndex, BondIndex)>>,
}

impl MolGraph {
    /// Create an empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a graph from atomic numbers and bonds given as pairs of atom positions.
    ///
    /// # Errors
    ///
    /// Returns the first [`GraphError`] that [`add_atom`](Self::add_atom) or
    /// [`add_bond`](Self::add_bond) hits.
    ///
    /// # Panics
    ///
    /// Panics if a position doesn't fit in a `u32`.
    pub fn from_bonds(atomic_numbers: &[u8], bonds: &[[usize; 2]]) -> Result<Self, GraphError> {
        let mut g = Self::new();
        for &atomic_number in atomic_numbers {
            g.add_atom(atomic_number)?;
        }
        for &[atom1, atom2] in bonds {
            g.add_bond(AtomIndex::new(atom1), AtomIndex::new(atom2))?;
        }
        Ok(g)
    }

    /// Add an atom with the given atomic number and return its index.
    ///
    /// The atomic number must be an element's or `0` for the wildcard (see
    /// [`element::symbol`]).
    ///
    /// # Errors
    ///
    /// Returns [`GraphError::UnknownElement`] if `atomic_number` isn't `0` and isn't in the
    /// element table.
    ///
    /// # Panics
    ///
    /// Panics if the new atom's index doesn't fit in a `u32`.
    pub fn add_atom(&mut self, atomic_number: u8) -> Result<AtomIndex, GraphError> {
        element::symbol(atomic_number).ok_or(GraphError::UnknownElement(atomic_number))?;
        let atom = AtomIndex::new(self.atomic_numbers.len());
        self.atomic_numbers.push(atomic_number);
        self.adjacency.push(Vec::new());
        Ok(atom)
    }

    /// Add a bond between `atom1` and `atom2` and return its index.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if either atom is out of range, if `atom1 == atom2`, or if the
    /// atoms are already bonded.
    ///
    /// # Panics
    ///
    /// Panics if the new bond's index doesn't fit in a `u32`.
    pub fn add_bond(
        &mut self,
        atom1: AtomIndex,
        atom2: AtomIndex,
    ) -> Result<BondIndex, GraphError> {
        // Check for errors
        for atom in [atom1, atom2] {
            self.atomic_number(atom)
                .ok_or(GraphError::AtomOutOfRange(atom))?;
        }
        if atom1 == atom2 {
            return Err(GraphError::SelfLoop(atom1));
        }
        if let Some(bond) = self.find_bond(atom1, atom2) {
            return Err(GraphError::DuplicateBond(bond));
        }
        // Add the bond
        let bond = BondIndex::new(self.bond_atoms.len());
        self.bond_atoms.push([atom1, atom2]);
        self.adjacency[atom1.index()].push((atom2, bond));
        self.adjacency[atom2.index()].push((atom1, bond));
        Ok(bond)
    }

    /// The number of atoms.
    #[must_use]
    pub fn atom_count(&self) -> usize {
        self.atomic_numbers.len()
    }

    /// The number of bonds.
    #[must_use]
    pub fn bond_count(&self) -> usize {
        self.bond_atoms.len()
    }

    /// The atomic number of `atom`, or `None` if it's out of range.
    #[must_use]
    pub fn atomic_number(&self, atom: AtomIndex) -> Option<u8> {
        self.atomic_numbers.get(atom.index()).copied()
    }

    /// The bond between `atom1` and `atom2`, if there is one.
    #[must_use]
    pub fn find_bond(&self, atom1: AtomIndex, atom2: AtomIndex) -> Option<BondIndex> {
        self.adjacency
            .get(atom1.index())?
            .iter()
            .find_map(|&(a, b)| (a == atom2).then_some(b))
    }

    /// The two atoms joined by `bond`, or `None` if it's out of range.
    #[must_use]
    pub fn bond_atoms(&self, bond: BondIndex) -> Option<[AtomIndex; 2]> {
        self.bond_atoms.get(bond.index()).copied()
    }

    /// Iterate over the atom indices.
    ///
    /// The iterator doesn't borrow the graph, so you can add bonds while iterating.
    pub fn atoms(&self) -> impl Iterator<Item = AtomIndex> + use<> {
        (0..self.atomic_numbers.len()).map(AtomIndex::new)
    }

    /// Iterate over the bond indices.
    ///
    /// The iterator doesn't borrow the graph, so you can add bonds while iterating.
    pub fn bonds(&self) -> impl Iterator<Item = BondIndex> + use<> {
        (0..self.bond_atoms.len()).map(BondIndex::new)
    }

    /// The number of neighbors of `atom`.
    ///
    /// # Panics
    ///
    /// Panics if `atom` is out of range.
    #[must_use]
    pub fn degree(&self, atom: AtomIndex) -> usize {
        self.adjacency[atom.index()].len()
    }

    /// Iterate over the neighbors of `atom`, in the order their bonds were added.
    ///
    /// # Panics
    ///
    /// Panics if `atom` is out of range.
    pub fn neighbors(&self, atom: AtomIndex) -> impl Iterator<Item = AtomIndex> + '_ {
        self.adjacency[atom.index()].iter().map(|&(a, _)| a)
    }

    /// The chemical symbol of each atom, in atom order, with `"*"` for a wildcard.
    ///
    /// # Examples
    ///
    /// ```
    /// use protochem::mol_graph::MolGraph;
    ///
    /// let graph = MolGraph::from_bonds(&[8, 1, 1, 0], &[[0, 1], [0, 2], [0, 3]]).unwrap();
    /// assert_eq!(graph.symbols(), ["O", "H", "H", "*"]);
    /// ```
    #[must_use]
    #[expect(
        clippy::missing_panics_doc,
        reason = "add_atom only accepts valid atomic numbers"
    )]
    pub fn symbols(&self) -> Vec<&'static str> {
        self.atomic_numbers
            .iter()
            .copied()
            .map(element::symbol)
            .map(|s| s.expect("add_atom should only accept valid atomic numbers"))
            .collect()
    }

    /// The molecular formula in Hill order.
    ///
    /// With carbon present, carbon comes first, then hydrogen, then the other symbols
    /// alphabetically. Without carbon, every symbol is alphabetical, hydrogen included. A
    /// count of 1 is left out, and wildcards are counted as `*`.
    ///
    /// # Examples
    ///
    /// ```
    /// use protochem::mol_graph::MolGraph;
    ///
    /// // Ethanol, CH3CH2OH
    /// let atomic_numbers = [6, 6, 8, 1, 1, 1, 1, 1, 1];
    /// let bonds = [[0, 1], [1, 2], [0, 3], [0, 4], [0, 5], [1, 6], [1, 7], [2, 8]];
    /// let graph = MolGraph::from_bonds(&atomic_numbers, &bonds).unwrap();
    /// assert_eq!(graph.formula(), "C2H6O");
    ///
    /// // Without carbon, hydrogen is sorted alphabetically
    /// let graph = MolGraph::from_bonds(&[5, 1, 1, 1], &[[0, 1], [0, 2], [0, 3]]).unwrap();
    /// assert_eq!(graph.formula(), "BH3");
    /// ```
    #[must_use]
    pub fn formula(&self) -> String {
        // count occurrences of each symbol
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for symbol in self.symbols() {
            *counts.entry(symbol).or_insert(0) += 1;
        }
        // sort the counts
        let has_carbon = counts.contains_key("C");
        let mut sorted_counts: Vec<(&str, usize)> = counts.into_iter().collect();
        sorted_counts.sort_unstable_by_key(|c| hill_key(c.0, has_carbon));
        // build formula
        let mut formula = String::new();
        for (symbol, count) in sorted_counts {
            formula.push_str(symbol);
            if count > 1 {
                formula.push_str(&count.to_string());
            }
        }
        formula
    }
}

fn hill_key(symbol: &str, has_carbon: bool) -> (u8, &str) {
    match symbol {
        "C" if has_carbon => (0, "C"),
        "H" if has_carbon => (1, "H"),
        symbol => (2, symbol),
    }
}

/// The index of an atom in a [`MolGraph`].
///
/// Atom and bond indices are different types, so they can't be mixed up:
///
/// ```compile_fail,E0308
/// use protochem::mol_graph::{BondIndex, MolGraph};
///
/// let graph = MolGraph::from_bonds(&[6, 8], &[[0, 1]]).unwrap();
/// let _ = graph.atomic_number(BondIndex::new(0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AtomIndex(u32);

impl AtomIndex {
    /// Create an atom index from a position.
    ///
    /// # Panics
    ///
    /// Panics if `index` doesn't fit in a `u32`.
    #[must_use]
    pub fn new(index: usize) -> Self {
        Self(to_u32(index))
    }

    /// The position, for indexing a `Vec`.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// The index of a bond in a [`MolGraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BondIndex(u32);

impl BondIndex {
    /// Create a bond index from a position.
    ///
    /// # Panics
    ///
    /// Panics if `index` doesn't fit in a `u32`.
    #[must_use]
    pub fn new(index: usize) -> Self {
        Self(to_u32(index))
    }

    /// The position, for indexing a `Vec`.
    #[must_use]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// An error from building a [`MolGraph`].
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GraphError {
    /// The atomic number is invalid.
    UnknownElement(u8),
    /// The atom isn't in the graph.
    AtomOutOfRange(AtomIndex),
    /// A bond would join the atom to itself.
    SelfLoop(AtomIndex),
    /// The atoms are already joined by this bond.
    DuplicateBond(BondIndex),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownElement(z) => write!(f, "{z} is not a valid atomic number"),
            Self::AtomOutOfRange(atom) => write!(f, "atom {} is out of range", atom.index()),
            Self::SelfLoop(atom) => write!(f, "atom {} can't be bonded to itself", atom.index()),
            Self::DuplicateBond(bond) => {
                write!(f, "bond {} already joins these atoms", bond.index())
            }
        }
    }
}

impl std::error::Error for GraphError {}

fn to_u32(index: usize) -> u32 {
    u32::try_from(index).expect("index should be convertible to u32")
}

#[cfg(test)]
mod tests {
    use super::*;

    // MolGraph

    #[test]
    fn new_graph_is_empty() {
        let graph = MolGraph::new();
        assert_eq!(graph.atom_count(), 0);
        assert_eq!(graph.bond_count(), 0);
    }

    #[test]
    fn add_atom_returns_consecutive_indices() {
        let mut graph = MolGraph::new();
        assert_eq!(graph.add_atom(6), Ok(AtomIndex::new(0)));
        assert_eq!(graph.add_atom(8), Ok(AtomIndex::new(1)));
        assert_eq!(graph.atom_count(), 2);
    }

    #[test]
    fn add_atom_adds_an_empty_adjacency_list() {
        let mut graph = MolGraph::new();
        graph.add_atom(6).unwrap();
        graph.add_atom(8).unwrap();
        assert_eq!(graph.adjacency, [Vec::new(), Vec::new()]);
    }

    #[test]
    fn add_atom_accepts_the_wildcard() {
        let mut graph = MolGraph::new();
        let wildcard = graph.add_atom(0).unwrap();
        assert_eq!(graph.atomic_number(wildcard), Some(0));
    }

    #[test]
    fn add_atom_rejects_unknown_elements() {
        let mut graph = MolGraph::new();
        assert_eq!(graph.add_atom(200), Err(GraphError::UnknownElement(200)));
        assert_eq!(graph.atom_count(), 0);
    }

    #[test]
    fn from_bonds_rejects_unknown_elements() {
        let result = MolGraph::from_bonds(&[6, 200], &[[0, 1]]);
        assert_eq!(result, Err(GraphError::UnknownElement(200)));
    }

    #[test]
    fn atomic_number_lookup() {
        let mut graph = MolGraph::new();
        graph.add_atom(6).unwrap();
        let oxygen = graph.add_atom(8).unwrap();
        assert_eq!(graph.atomic_number(oxygen), Some(8));
        assert_eq!(graph.atomic_number(AtomIndex::new(2)), None);
    }

    #[test]
    fn add_bonds() {
        let mut graph = MolGraph::new();
        let atom1 = graph.add_atom(6).unwrap();
        let atom2 = graph.add_atom(8).unwrap();
        let bond = BondIndex::new(0);
        // Normal use
        assert_eq!(graph.add_bond(atom1, atom2), Ok(bond));
        // Out of range
        let missing_atom = AtomIndex::new(5);
        assert_eq!(
            graph.add_bond(atom1, missing_atom),
            Err(GraphError::AtomOutOfRange(missing_atom))
        );
        assert_eq!(
            graph.add_bond(missing_atom, atom2),
            Err(GraphError::AtomOutOfRange(missing_atom))
        );
        // Self loop
        assert_eq!(
            graph.add_bond(atom1, atom1),
            Err(GraphError::SelfLoop(atom1))
        );
        // Duplicate bond
        assert_eq!(
            graph.add_bond(atom1, atom2),
            Err(GraphError::DuplicateBond(bond))
        );
        assert_eq!(
            graph.add_bond(atom2, atom1),
            Err(GraphError::DuplicateBond(bond))
        );
        // Other checks
        assert_eq!(graph.find_bond(atom1, atom2), Some(bond));
        assert_eq!(graph.find_bond(atom2, atom1), Some(bond));
        assert_eq!(graph.find_bond(atom1, missing_atom), None);
        assert_eq!(graph.find_bond(missing_atom, atom2), None);
        assert_eq!(graph.find_bond(atom1, atom1), None);
        assert_eq!(graph.bond_count(), 1);
        assert_eq!(graph.bond_atoms(bond), Some([atom1, atom2]));
        assert_eq!(graph.bond_atoms(BondIndex::new(1)), None);
    }

    #[test]
    fn from_bonds_stops_at_first_error() {
        let result = MolGraph::from_bonds(&[6, 6], &[[0, 1], [1, 1], [0, 7]]);
        assert_eq!(result, Err(GraphError::SelfLoop(AtomIndex::new(1))));
    }

    #[test]
    fn neighbors_and_degrees() {
        // Ethanol with explicit hydrogens
        let atomic_numbers = [6, 6, 8, 1, 1, 1, 1, 1, 1];
        let bonds = [
            [0, 1],
            [1, 2],
            [0, 3],
            [0, 4],
            [0, 5],
            [1, 6],
            [1, 7],
            [2, 8],
        ];
        let graph = MolGraph::from_bonds(&atomic_numbers, &bonds).unwrap();
        let neighbors: Vec<_> = graph.neighbors(AtomIndex::new(1)).collect();
        assert_eq!(neighbors, [0, 2, 6, 7].map(AtomIndex::new));
        let degrees: Vec<_> = graph.atoms().map(|atom| graph.degree(atom)).collect();
        assert_eq!(degrees, [4, 4, 2, 1, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn atoms_does_not_borrow_the_graph() {
        let mut graph = MolGraph::from_bonds(&[6, 6, 6], &[]).unwrap();
        for atom in graph.atoms().skip(1) {
            graph.add_bond(AtomIndex::new(0), atom).unwrap();
        }
        assert_eq!(graph.degree(AtomIndex::new(0)), 2);
    }

    #[test]
    fn formula() {
        // Bonds don't affect the formula, so the graphs have none
        for (atomic_numbers, expected) in [
            (vec![], ""),
            (vec![6, 1, 1, 1, 1], "CH4"),
            (vec![6, 6, 8, 1, 1, 1, 1, 1, 1], "C2H6O"), // ethanol
            (vec![17, 6, 1, 1, 1, 35], "CH3BrCl"),
            (vec![0, 6], "C*"),
            // Without carbon, everything is alphabetical
            (vec![8, 1, 1], "H2O"),
            (vec![11, 17], "ClNa"),
            (vec![5, 1, 1, 1], "BH3"),
        ] {
            let graph = MolGraph::from_bonds(&atomic_numbers, &[]).unwrap();
            assert_eq!(
                graph.formula(),
                expected,
                "atomic numbers {atomic_numbers:?}"
            );
        }
    }

    // AtomIndex, BondIndex

    #[test]
    fn index_round_trip() {
        assert_eq!(AtomIndex::new(3).index(), 3);
        assert_eq!(BondIndex::new(3).index(), 3);
    }

    #[test]
    fn index_order() {
        assert!(AtomIndex::new(1) < AtomIndex::new(2));
        assert!(BondIndex::new(1) < BondIndex::new(2));
    }

    #[test]
    #[should_panic(expected = "index should be convertible to u32")]
    #[cfg(target_pointer_width = "64")]
    fn index_too_large() {
        let _ = AtomIndex::new(u32::MAX as usize + 1);
    }

    // GraphError

    #[test]
    fn graph_error_is_a_good_error() {
        fn assert_error<E: std::error::Error + Send + Sync + 'static>() {}
        assert_error::<GraphError>();
    }
}
