//! A molecular graph with dense atom and bond indices.
//!
//! A [`MolGraph`] stores an atomic number for each atom and the pair of atoms for each
//! bond. Nothing else is stored: bond orders, charges and stereo go in separate tables,
//! indexed by [`AtomIndex`] and [`BondIndex`].
//!
//! Atoms and bonds are numbered from 0 in the order they're added. The two index types are
//! different, so an atom index can't be passed where a bond index is expected. Lookups
//! that take an index return `None` if it's out of range. The neighbor and degree queries
//! and the algorithms panic instead.
//!
//! [`MolGraph::bfs_layers`] and [`MolGraph::components`] are the graph algorithms.
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
/// use protochem::{AtomIndex, MolGraph};
///
/// // Ethanol's heavy atoms: C0-C1-O2
/// let graph = MolGraph::from_bonds(&[6, 6, 8], &[[0, 1], [1, 2]]).unwrap();
/// let neighbors: Vec<_> = graph.neighbors(AtomIndex::new(1)).collect();
/// assert_eq!(neighbors, [AtomIndex::new(0), AtomIndex::new(2)]);
///
/// // The formula counts only the atoms in the graph, so the hydrogens are missing
/// assert_eq!(graph.formula(), "C2O");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MolGraph {
    atomic_numbers: Vec<u8>,
    bond_atoms: Vec<[AtomIndex; 2]>,
    adjacency: Vec<Vec<(AtomIndex, BondIndex)>>,
}

impl MolGraph {
    // Constructors

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

    // Mutators

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

    // Accessors and iterators

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

    // Derived properties

    /// The chemical symbol of each atom, in atom order, with `"*"` for a wildcard.
    ///
    /// # Examples
    ///
    /// ```
    /// use protochem::MolGraph;
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
    /// use protochem::MolGraph;
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

    // Algorithms

    /// The atoms grouped by their distance from `start`.
    ///
    /// Layer 0 is `[start]`, and each later layer holds the neighbors of the layer before it
    /// that haven't been seen yet. Within a layer, atoms are in the order the search found
    /// them. Atoms that can't be reached from `start` are left out.
    ///
    /// # Panics
    ///
    /// Panics if `start` is out of range.
    ///
    /// # Examples
    ///
    /// ```
    /// use protochem::{AtomIndex, MolGraph};
    ///
    /// // Cyclohexane's carbon ring
    /// let bonds = [[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5, 0]];
    /// let graph = MolGraph::from_bonds(&[6; 6], &bonds).unwrap();
    /// let layers = graph.bfs_layers(AtomIndex::new(0));
    /// assert_eq!(layers.len(), 4);
    /// assert_eq!(layers[1], [1, 5].map(AtomIndex::new));
    /// assert_eq!(layers[3], [AtomIndex::new(3)]);
    /// ```
    #[must_use]
    pub fn bfs_layers(&self, start: AtomIndex) -> Vec<Vec<AtomIndex>> {
        let mut visited = Visited::new(self.atom_count());
        self.layers_from(start, &mut visited).collect()
    }

    /// The connected components: groups of atoms joined to each other by a path of bonds.
    ///
    /// Components are ordered by their lowest atom, and each one lists its atoms in BFS
    /// order from that atom, as in [`bfs_layers`](Self::bfs_layers) flattened. An atom with
    /// no bonds is a component by itself, and an empty graph has no components.
    ///
    /// # Examples
    ///
    /// ```
    /// use protochem::{AtomIndex, MolGraph};
    ///
    /// // A lone carbon atom, plus a water molecule
    /// let graph = MolGraph::from_bonds(&[6, 8, 1, 1], &[[1, 2], [1, 3]]).unwrap();
    /// let components = graph.components();
    /// assert_eq!(components.len(), 2);
    /// assert_eq!(components[0], [AtomIndex::new(0)]);
    /// assert_eq!(components[1], [1, 2, 3].map(AtomIndex::new));
    /// ```
    #[must_use]
    #[expect(
        clippy::filter_map_bool_then,
        reason = "`filter` then `map` would borrow `visited` in both closures, mutably in `map`"
    )]
    pub fn components(&self) -> Vec<Vec<AtomIndex>> {
        let mut visited = Visited::new(self.atom_count());
        self.atoms()
            // Map `a` to its flattened BFS, recording the atoms as visited
            .filter_map(|a| {
                // Skip `a` if it was already visited
                (!visited.contains(a))
                    .then(|| self.layers_from(a, &mut visited).flatten().collect())
            })
            .collect()
    }

    /// The BFS layers from `start`, as in [`bfs_layers`](Self::bfs_layers), skipping atoms
    /// already in `visited`.
    ///
    /// Marks `start` and every atom it reaches in `visited`, so a later call with the same
    /// `visited` won't revisit them.
    fn layers_from<'a>(
        &'a self,
        start: AtomIndex,
        visited: &'a mut Visited,
    ) -> impl Iterator<Item = Vec<AtomIndex>> + 'a {
        visited.insert(start);
        std::iter::successors(Some(vec![start]), |layer| {
            let next: Vec<_> = layer
                .iter()
                .flat_map(|&a| self.neighbors(a))
                .filter(|&a| visited.insert(a))
                .collect();
            (!next.is_empty()).then_some(next)
        })
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

fn hill_key(symbol: &str, has_carbon: bool) -> (u8, &str) {
    match symbol {
        "C" if has_carbon => (0, "C"),
        "H" if has_carbon => (1, "H"),
        symbol => (2, symbol),
    }
}

/// The set of atoms a search has visited, as a `Vec<bool>` indexed by atom.
struct Visited(Vec<bool>);

impl Visited {
    fn new(atom_count: usize) -> Self {
        Self(vec![false; atom_count])
    }

    /// Mark `atom` as visited and return `true` if it wasn't before.
    fn insert(&mut self, atom: AtomIndex) -> bool {
        !std::mem::replace(&mut self.0[atom.index()], true)
    }

    /// Return `true` if `atom` was visited.
    fn contains(&self, atom: AtomIndex) -> bool {
        self.0[atom.index()]
    }
}

fn to_u32(index: usize) -> u32 {
    u32::try_from(index).expect("index should be convertible to u32")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ethanol with explicit hydrogens: C0-C1-O2, H3-H5 on C0, H6-H7 on C1, H8 on O2.
    fn ethanol() -> MolGraph {
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
        MolGraph::from_bonds(&atomic_numbers, &bonds).unwrap()
    }

    // Cyclohexane's carbon ring, without hydrogens.
    fn cyclohexane() -> MolGraph {
        let bonds = [[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5, 0]];
        MolGraph::from_bonds(&[6; 6], &bonds).unwrap()
    }

    // Atom groups as plain positions, for short expected values.
    fn positions(groups: Vec<Vec<AtomIndex>>) -> Vec<Vec<usize>> {
        groups
            .into_iter()
            .map(|group| group.into_iter().map(AtomIndex::index).collect())
            .collect()
    }

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
    fn add_bond() {
        let mut graph = MolGraph::from_bonds(&[6, 8], &[]).unwrap();
        let [atom1, atom2] = [0, 1].map(AtomIndex::new);
        let bond = BondIndex::new(0);
        assert_eq!(graph.add_bond(atom1, atom2), Ok(bond));
        assert_eq!(graph.bond_count(), 1);
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
        assert_eq!(graph.bond_count(), 1);
    }

    #[test]
    fn find_bond_and_bond_atoms() {
        let graph = MolGraph::from_bonds(&[6, 8], &[[0, 1]]).unwrap();
        let [atom1, atom2] = [0, 1].map(AtomIndex::new);
        let missing_atom = AtomIndex::new(5);
        let bond = BondIndex::new(0);
        assert_eq!(graph.find_bond(atom1, atom2), Some(bond));
        assert_eq!(graph.find_bond(atom2, atom1), Some(bond));
        assert_eq!(graph.find_bond(atom1, missing_atom), None);
        assert_eq!(graph.find_bond(missing_atom, atom2), None);
        assert_eq!(graph.find_bond(atom1, atom1), None);
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
        let graph = ethanol();
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

    #[test]
    fn bfs_layers() {
        let layers = |graph: &MolGraph, start| positions(graph.bfs_layers(AtomIndex::new(start)));
        assert_eq!(
            layers(&cyclohexane(), 0),
            [vec![0], vec![1, 5], vec![2, 4], vec![3]]
        );
        assert_eq!(
            layers(&ethanol(), 8),
            [vec![8], vec![2], vec![1], vec![0, 6, 7], vec![3, 4, 5]]
        );
        // Unreachable atoms are left out
        let unbonded = MolGraph::from_bonds(&[6, 6], &[]).unwrap();
        assert_eq!(layers(&unbonded, 1), [vec![1]]);
    }

    #[test]
    #[should_panic(expected = "index out of bounds")]
    fn bfs_layers_panics_out_of_range() {
        let _ = cyclohexane().bfs_layers(AtomIndex::new(6));
    }

    #[test]
    fn components() {
        let components = |atomic_numbers: &[u8], bonds: &[[usize; 2]]| {
            positions(
                MolGraph::from_bonds(atomic_numbers, bonds)
                    .unwrap()
                    .components(),
            )
        };
        // Ordered by lowest atom, each in BFS order from it
        assert_eq!(components(&[6, 6, 8], &[[0, 1]]), [vec![0, 1], vec![2]]);
        assert_eq!(components(&[6, 8, 6], &[[0, 2]]), [vec![0, 2], vec![1]]);
        // NaCl, no bonds
        assert_eq!(components(&[11, 17], &[]), [vec![0], vec![1]]);
        // A chain whose bonds were added out of order comes out in BFS order
        assert_eq!(
            components(&[6; 4], &[[3, 1], [1, 2], [2, 0]]),
            [vec![0, 2, 1, 3]]
        );
        assert_eq!(components(&[], &[]), Vec::<Vec<usize>>::new());
    }

    // AtomIndex, BondIndex

    #[test]
    fn index_round_trip() {
        assert_eq!(AtomIndex::new(3).index(), 3);
        assert_eq!(BondIndex::new(3).index(), 3);
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
