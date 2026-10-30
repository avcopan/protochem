//! A molecular graph with dense atom and bond indices.
//!
//! [`MolGraph`] stores atoms and bonds in `Vec`s. Atoms are numbered `0..atom_count()` by
//! [`AtomIndex`] and bonds `0..bond_count()` by [`BondIndex`], in the order they were added. Each
//! atom's adjacency list holds `(neighbor, bond)` pairs, so walking the graph also gives you the
//! bond to look up per-bond data.
use crate::Element;
use std::collections::{HashMap, VecDeque};
use std::fmt;

/// An undirected molecular graph without self-loops or multiple bonds.
///
/// Atoms carry an atomic number. Bonds carry nothing: bond orders, charges and
/// stereo belong in separate tables indexed by [`AtomIndex`] and [`BondIndex`].
///
/// # Examples
///
/// ```
/// use protochem::mol_graph::{AtomIndex, MolGraph};
///
/// // Ethanol's heavy atoms: C-C-O
/// let graph = MolGraph::from_bonds(&[6, 6, 8], &[[0, 1], [1, 2]]).unwrap();
/// let neighbors: Vec<_> = graph.neighbors(AtomIndex::new(1)).collect();
/// assert_eq!(neighbors, [AtomIndex::new(0), AtomIndex::new(2)]);
/// assert_eq!(graph.formula(), "C2O");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MolGraph {
    atomic_numbers: Vec<u8>,
    bonds: Vec<[AtomIndex; 2]>,
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
    /// Returns the first [`GraphError`] that [`add_bond`](Self::add_bond) hits.
    ///
    /// # Panics
    ///
    /// Panics if a position doesn't fit in a `u32`.
    pub fn from_bonds(atomic_numbers: &[u8], bonds: &[[usize; 2]]) -> Result<Self, GraphError> {
        let mut graph = Self::new();
        for &atomic_number in atomic_numbers {
            graph.add_atom(atomic_number);
        }
        for &[a, b] in bonds {
            graph.add_bond(AtomIndex::new(a), AtomIndex::new(b))?;
        }
        Ok(graph)
    }

    /// Add an atom and return its index.
    ///
    /// # Panics
    ///
    /// Panics if the graph already has `u32::MAX` atoms.
    pub fn add_atom(&mut self, atomic_number: u8) -> AtomIndex {
        let atom = AtomIndex::new(self.atomic_numbers.len());
        self.atomic_numbers.push(atomic_number);
        self.adjacency.push(Vec::new());
        atom
    }

    /// Add a bond between `a` and `b` and return its index.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if either atom is out of range, if `a == b`, or if the atoms
    /// are already bonded.
    ///
    /// # Panics
    ///
    /// Panics if the graph already has `u32::MAX` bonds.
    pub fn add_bond(&mut self, a: AtomIndex, b: AtomIndex) -> Result<BondIndex, GraphError> {
        for atom in [a, b] {
            self.atomic_number(atom)
                .ok_or(GraphError::AtomOutOfRange(atom))?;
        }
        if a == b {
            return Err(GraphError::SelfLoop(a));
        }
        if let Some(bond) = self.find_bond(a, b) {
            return Err(GraphError::DuplicateBond(bond));
        }

        let bond = BondIndex::new(self.bonds.len());
        self.bonds.push([a, b]);
        self.adjacency[a.index()].push((b, bond));
        self.adjacency[b.index()].push((a, bond));
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
        self.bonds.len()
    }

    /// The atomic number of `atom`, or `None` if it's out of range.
    #[must_use]
    pub fn atomic_number(&self, atom: AtomIndex) -> Option<u8> {
        self.atomic_numbers.get(atom.index()).copied()
    }

    /// The two atoms of `bond`, in the order they were added, or `None` if it's out of range.
    #[must_use]
    pub fn bond_atoms(&self, bond: BondIndex) -> Option<[AtomIndex; 2]> {
        self.bonds.get(bond.index()).copied()
    }

    /// The bond between `a` and `b`, if there is one.
    #[must_use]
    pub fn find_bond(&self, a: AtomIndex, b: AtomIndex) -> Option<BondIndex> {
        self.adjacency
            .get(a.index())?
            .iter()
            .find_map(|&(neighbor, bond)| (neighbor == b).then_some(bond))
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

    /// Iterate over the atom indices.
    ///
    /// The iterator doesn't borrow the graph, so you can add bonds while iterating.
    pub fn atoms(&self) -> impl Iterator<Item = AtomIndex> + use<> {
        (0..self.atom_count()).map(AtomIndex::new)
    }

    /// Iterate over the bond indices.
    ///
    /// The iterator doesn't borrow the graph, so you can add bonds while iterating.
    pub fn bonds(&self) -> impl Iterator<Item = BondIndex> + use<> {
        (0..self.bond_count()).map(BondIndex::new)
    }

    /// Iterate over the neighbors of `atom`, in the order their bonds were added.
    ///
    /// # Panics
    ///
    /// Panics if `atom` is out of range.
    pub fn neighbors(&self, atom: AtomIndex) -> impl Iterator<Item = AtomIndex> + '_ {
        self.adjacency[atom.index()]
            .iter()
            .map(|&(neighbor, _)| neighbor)
    }

    /// The molecular formula in Hill order.
    ///
    /// Carbon comes first, then hydrogen, then the other elements alphabetically. Without
    /// carbon, all elements are alphabetical. Atomic numbers that aren't elements (the SMILES
    /// wildcard `0`) are written as `*`.
    #[must_use]
    pub fn formula(&self) -> String {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for &atomic_number in &self.atomic_numbers {
            *counts.entry(symbol(atomic_number)).or_default() += 1;
        }

        let has_carbon = counts.contains_key("C");
        let mut counts: Vec<_> = counts.into_iter().collect();
        counts.sort_unstable_by_key(|&(symbol, _)| hill_key(symbol, has_carbon));
        counts
            .into_iter()
            .map(|(symbol, count)| match count {
                1 => symbol.to_owned(),
                _ => format!("{symbol}{count}"),
            })
            .collect()
    }

    /// The atoms grouped by their distance from `start`.
    ///
    /// Layer 0 is `[start]`, layer 1 its neighbors, and so on, up to the last atom reachable
    /// from `start`. Within a layer, atoms are in the order the search found them.
    ///
    /// # Panics
    ///
    /// Panics if `start` is out of range.
    #[must_use]
    pub fn bfs_layers(&self, start: AtomIndex) -> Vec<Vec<AtomIndex>> {
        let mut visited = Visited::new(self.atom_count());
        visited.insert(start);
        std::iter::successors(Some(vec![start]), |layer| {
            let next: Vec<_> = layer
                .iter()
                .flat_map(|&atom| self.neighbors(atom))
                .filter(|&neighbor| visited.insert(neighbor))
                .collect();
            (!next.is_empty()).then_some(next)
        })
        .collect()
    }

    /// The connected components.
    ///
    /// Components are ordered by their lowest atom index. Each one starts with that atom, then
    /// lists the others in breadth-first order.
    #[must_use]
    pub fn components(&self) -> Vec<Vec<AtomIndex>> {
        let mut visited = Visited::new(self.atom_count());
        let mut components = Vec::new();
        for root in self.atoms() {
            if !visited.insert(root) {
                continue;
            }
            let mut component = vec![root];
            let mut queue = VecDeque::from([root]);
            while let Some(atom) = queue.pop_front() {
                for neighbor in self.neighbors(atom) {
                    if visited.insert(neighbor) {
                        component.push(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
            components.push(component);
        }
        components
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
        Self(u32::try_from(index).expect("atom index should fit in a u32"))
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
        Self(u32::try_from(index).expect("bond index should fit in a u32"))
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
            Self::AtomOutOfRange(atom) => write!(f, "atom {} is out of range", atom.index()),
            Self::SelfLoop(atom) => write!(f, "atom {} can't bond to itself", atom.index()),
            Self::DuplicateBond(bond) => {
                write!(f, "bond {} already joins these atoms", bond.index())
            }
        }
    }
}

impl std::error::Error for GraphError {}

fn symbol(atomic_number: u8) -> &'static str {
    Element::from_atomic_number(atomic_number).map_or("*", |element| element.symbol)
}

fn hill_key(symbol: &str, has_carbon: bool) -> (u8, &str) {
    match symbol {
        "C" if has_carbon => (0, symbol),
        "H" if has_carbon => (1, symbol),
        _ => (2, symbol),
    }
}

/// A set of visited atoms, as a `Vec<bool>` indexed by atom.
#[derive(Debug)]
struct Visited(Vec<bool>);

impl Visited {
    fn new(atom_count: usize) -> Self {
        Self(vec![false; atom_count])
    }

    /// Mark `atom` as visited. Return `true` if it wasn't before, like `HashSet::insert`.
    fn insert(&mut self, atom: AtomIndex) -> bool {
        !std::mem::replace(&mut self.0[atom.index()], true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(index: usize) -> AtomIndex {
        AtomIndex::new(index)
    }

    fn atoms(indices: &[usize]) -> Vec<AtomIndex> {
        indices.iter().copied().map(AtomIndex::new).collect()
    }

    /// Ethanol with explicit hydrogens: C0-C1-O2, H3-H5 on C0, H6-H7 on C1, H8 on O2.
    fn ethanol() -> MolGraph {
        let atomic_numbers = [6, 6, 8, 1, 1, 1, 1, 1, 1];
        let bonds = [[0, 1], [1, 2], [0, 3], [0, 4], [0, 5], [1, 6], [1, 7], [2, 8]];
        MolGraph::from_bonds(&atomic_numbers, &bonds).expect("valid graph")
    }

    fn cyclohexane() -> MolGraph {
        let bonds = [[0, 1], [1, 2], [2, 3], [3, 4], [4, 5], [5, 0]];
        MolGraph::from_bonds(&[6; 6], &bonds).expect("valid graph")
    }

    // MolGraph

    #[test]
    fn add_atoms_and_bonds() {
        let mut graph = MolGraph::new();
        let c = graph.add_atom(6);
        let o = graph.add_atom(8);
        assert_eq!((c, o), (atom(0), atom(1)));
        assert_eq!(graph.add_bond(c, o), Ok(BondIndex::new(0)));
        assert_eq!((graph.atom_count(), graph.bond_count()), (2, 1));
    }

    #[test]
    fn add_bond_errors() {
        let mut graph = MolGraph::from_bonds(&[6, 8], &[[0, 1]]).expect("valid graph");
        for ([a, b], error) in [
            ([0, 5], GraphError::AtomOutOfRange(atom(5))),
            ([5, 0], GraphError::AtomOutOfRange(atom(5))),
            ([1, 1], GraphError::SelfLoop(atom(1))),
            ([0, 1], GraphError::DuplicateBond(BondIndex::new(0))),
            ([1, 0], GraphError::DuplicateBond(BondIndex::new(0))),
        ] {
            assert_eq!(graph.add_bond(atom(a), atom(b)), Err(error), "{a}-{b}");
        }
        assert_eq!(graph.bond_count(), 1, "failed bonds aren't added");
    }

    #[test]
    fn from_bonds_stops_at_first_error() {
        let result = MolGraph::from_bonds(&[6, 6], &[[0, 1], [1, 1], [0, 7]]);
        assert_eq!(result, Err(GraphError::SelfLoop(atom(1))));
    }

    #[test]
    fn lookups() {
        let graph = ethanol();
        assert_eq!(graph.atomic_number(atom(2)), Some(8));
        assert_eq!(graph.atomic_number(atom(9)), None);
        assert_eq!(graph.bond_atoms(BondIndex::new(1)), Some([atom(1), atom(2)]));
        assert_eq!(graph.bond_atoms(BondIndex::new(8)), None);
        assert_eq!(graph.find_bond(atom(2), atom(1)), Some(BondIndex::new(1)));
        assert_eq!(graph.find_bond(atom(0), atom(2)), None);
        assert_eq!(graph.find_bond(atom(9), atom(0)), None);
        let degrees: Vec<_> = graph.atoms().map(|atom| graph.degree(atom)).collect();
        assert_eq!(degrees, [4, 4, 2, 1, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn iterators() {
        let graph = ethanol();
        assert_eq!(graph.atoms().count(), 9);
        assert_eq!(graph.bonds().count(), 8);
        let neighbors: Vec<_> = graph.neighbors(atom(1)).collect();
        assert_eq!(neighbors, atoms(&[0, 2, 6, 7]));
    }

    #[test]
    fn atoms_iterator_does_not_borrow() {
        let mut graph = MolGraph::from_bonds(&[6, 6, 6], &[]).expect("valid graph");
        for atom in graph.atoms().skip(1) {
            graph.add_bond(AtomIndex::new(0), atom).expect("new bond");
        }
        assert_eq!(graph.degree(AtomIndex::new(0)), 2);
    }

    #[test]
    fn adjacency_matches_bonds() {
        for graph in [ethanol(), cyclohexane()] {
            // Each bond once, as [lower, higher], from the adjacency lists
            let mut from_adjacency: Vec<_> = graph
                .atoms()
                .flat_map(|atom| {
                    graph
                        .neighbors(atom)
                        .filter(move |&neighbor| atom < neighbor)
                        .map(move |neighbor| [atom, neighbor])
                })
                .collect();
            // ...and from the bond list
            let mut from_bonds: Vec<_> = graph
                .bonds()
                .filter_map(|bond| graph.bond_atoms(bond))
                .map(|mut pair| {
                    pair.sort_unstable();
                    pair
                })
                .collect();
            from_adjacency.sort_unstable();
            from_bonds.sort_unstable();
            assert_eq!(from_adjacency, from_bonds);
        }
    }

    #[test]
    fn formula() {
        for (atomic_numbers, expected) in [
            (vec![], ""),
            (vec![6, 1, 1, 1, 1], "CH4"),
            (vec![6, 6, 8, 1, 1, 1, 1, 1, 1], "C2H6O"),
            (vec![8, 1, 1], "H2O"),
            (vec![11, 17], "ClNa"),
            (vec![17, 6, 1, 1, 1, 35], "CH3BrCl"),
            (vec![0, 6], "C*"),
        ] {
            let graph = MolGraph::from_bonds(&atomic_numbers, &[]).expect("valid graph");
            assert_eq!(graph.formula(), expected, "{atomic_numbers:?}");
        }
    }

    #[test]
    fn bfs_layers() {
        let layers = |graph: &MolGraph, start| -> Vec<Vec<usize>> {
            graph
                .bfs_layers(atom(start))
                .into_iter()
                .map(|layer| layer.into_iter().map(AtomIndex::index).collect())
                .collect()
        };
        let ring = cyclohexane();
        assert_eq!(layers(&ring, 0), [vec![0], vec![1, 5], vec![2, 4], vec![3]]);
        let graph = ethanol();
        assert_eq!(layers(&graph, 8), [vec![8], vec![2], vec![1], vec![0, 6, 7], vec![3, 4, 5]]);
        // Unreachable atoms don't show up
        let two = MolGraph::from_bonds(&[6, 6], &[]).expect("valid graph");
        assert_eq!(layers(&two, 1), [vec![1]]);
    }

    #[test]
    fn components() {
        for (atomic_numbers, bonds, expected) in [
            (vec![], vec![], vec![]),
            (vec![6, 6, 8], vec![[0, 1]], vec![atoms(&[0, 1]), atoms(&[2])]),
            (vec![6, 8, 6], vec![[0, 2]], vec![atoms(&[0, 2]), atoms(&[1])]),
            (vec![11, 17], vec![], vec![atoms(&[0]), atoms(&[1])]),
            (vec![6; 4], vec![[3, 1], [1, 2], [2, 0]], vec![atoms(&[0, 2, 1, 3])]),
        ] {
            let graph = MolGraph::from_bonds(&atomic_numbers, &bonds).expect("valid graph");
            assert_eq!(graph.components(), expected, "{bonds:?}");
        }
    }

    // AtomIndex, BondIndex

    #[test]
    fn index_round_trip() {
        assert_eq!(AtomIndex::new(3).index(), 3);
        assert_eq!(BondIndex::new(3).index(), 3);
        assert!(AtomIndex::new(1) < AtomIndex::new(2));
    }

    // GraphError

    #[test]
    fn error_messages() {
        for (error, message) in [
            (GraphError::AtomOutOfRange(atom(5)), "atom 5 is out of range"),
            (GraphError::SelfLoop(atom(1)), "atom 1 can't bond to itself"),
            (
                GraphError::DuplicateBond(BondIndex::new(0)),
                "bond 0 already joins these atoms",
            ),
        ] {
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn graph_error_is_a_good_error() {
        fn assert_error<E: std::error::Error + Send + Sync + 'static>() {}
        assert_error::<GraphError>();
    }
}
