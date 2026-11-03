//! A molecular graph with dense atom and bond indices.

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
/// let oxygen = graph.add_atom(8);
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

    /// Add an atom with the given atomic number and return its index.
    ///
    /// # Panics
    ///
    /// Panics if the new atom's index doesn't fit in a `u32`.
    pub fn add_atom(&mut self, atomic_number: u8) -> AtomIndex {
        let index = self.atomic_numbers.len();
        self.atomic_numbers.push(atomic_number);
        self.adjacency.push(Vec::new());
        AtomIndex::new(index)
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
}

/// The index of an atom in a [`MolGraph`].
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
        assert_eq!(graph.add_atom(6), AtomIndex::new(0));
        assert_eq!(graph.add_atom(8), AtomIndex::new(1));
        assert_eq!(graph.atom_count(), 2);
    }

    #[test]
    fn add_atom_adds_an_empty_adjacency_list() {
        let mut graph = MolGraph::new();
        graph.add_atom(6);
        graph.add_atom(8);
        assert_eq!(graph.adjacency, [Vec::new(), Vec::new()]);
    }

    #[test]
    fn atomic_number_lookup() {
        let mut graph = MolGraph::new();
        graph.add_atom(6);
        let oxygen = graph.add_atom(8);
        assert_eq!(graph.atomic_number(oxygen), Some(8));
        assert_eq!(graph.atomic_number(AtomIndex::new(2)), None);
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
}
