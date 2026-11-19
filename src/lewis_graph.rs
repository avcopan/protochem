//! One Lewis structure of a molecule: a graph with implicit hydrogens, bond
//! orders, and formal charges.
//!
//! A `LewisGraph` is built on a `MolGraph`, which defines its
//! [`skeleton`](LewisGraph::skeleton): the explicit atoms and the bonds between
//! them. It can be thought of as a deserialized SMILES string. It shares the
//! skeleton's [`AtomIndex`] and [`BondIndex`] and associates them with
//! additional [`AtomData`] and [`BondOrder`]s. The skeleton's methods are used
//! for graph traversal.
use crate::element::{self, hill_formula};
use crate::{AtomIndex, BondIndex, GraphError, MolGraph};

/// A molecular graph with bond orders, formal charges, and implicit hydrogens.
///
/// It holds a [`MolGraph`] skeleton and the Lewis structure data for each of
/// its atoms and bonds. Use the [`skeleton`](Self::skeleton) for graph queries,
/// such as neighbors and atom counts (minus implicit hydrogens).
///
/// # Examples
///
/// ```
/// use protochem::{AtomData, BondOrder, LewisGraph};
///
/// // Formaldehyde (H2C=O)
/// let mut graph = LewisGraph::new();
/// let carbon = graph.add_atom(AtomData { hydrogens: 2, ..AtomData::new(6) }).unwrap();
/// let oxygen = graph.add_atom(8).unwrap();
/// let bond = graph.add_bond(carbon, oxygen, BondOrder::Double).unwrap();
/// assert_eq!(graph.bond_order(bond), Some(BondOrder::Double));
/// assert_eq!(graph.skeleton().atom_count(), 2);
/// assert_eq!(graph.formula(), "CH2O");
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LewisGraph {
    /// The explicit atoms and their bonds.
    skeleton: MolGraph,
    /// The data of each atom, by [`AtomIndex`].
    atom_data: Vec<AtomData>,
    /// The order of each bond, by [`BondIndex`].
    bond_orders: Vec<BondOrder>,
}

impl LewisGraph {
    // Constructors

    /// Create an empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // Mutators

    /// Add an atom and return its index.
    ///
    /// Take an [`AtomData`] or anything that converts into one, such as a bare
    /// atomic number, which becomes a bare neutral atom.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if the skeleton rejects the atom (see
    /// [`MolGraph::add_atom`]).
    ///
    /// # Panics
    ///
    /// Panics if the new atom's index doesn't fit in a `u32`.
    ///
    /// # Examples
    ///
    /// ```
    /// use protochem::{AtomData, LewisGraph};
    ///
    /// let mut graph = LewisGraph::new();
    /// let carbon = graph.add_atom(6).unwrap();
    /// let ammonium = graph.add_atom(AtomData { charge: 1, hydrogens: 4, ..AtomData::new(7) }).unwrap();
    /// assert_eq!(graph.atom_data(carbon), Some(AtomData::new(6)));
    /// assert_eq!(graph.atom_data(ammonium).map(|data| data.atomic_number), Some(7));
    /// assert_eq!(graph.atom_data(ammonium).map(|data| data.charge), Some(1));
    /// assert_eq!(graph.atom_data(ammonium).map(|data| data.hydrogens), Some(4));
    /// ```
    pub fn add_atom(&mut self, data: impl Into<AtomData>) -> Result<AtomIndex, GraphError> {
        let data = data.into();
        let atom = self.skeleton.add_atom(data.atomic_number)?;
        self.atom_data.push(data);
        Ok(atom)
    }

    /// Add a bond of the given order between `atom1` and `atom2` and return its
    /// index.
    ///
    /// # Errors
    ///
    /// Returns a [`GraphError`] if the skeleton rejects the bond (see
    /// [`MolGraph::add_bond`]).
    ///
    /// # Panics
    ///
    /// Panics if the new bond's index doesn't fit in a `u32`.
    pub fn add_bond(
        &mut self,
        atom1: AtomIndex,
        atom2: AtomIndex,
        order: BondOrder,
    ) -> Result<BondIndex, GraphError> {
        let bond = self.skeleton.add_bond(atom1, atom2)?;
        self.bond_orders.push(order);
        Ok(bond)
    }

    // Accessors

    /// The skeleton: the explicit atoms and their bonds.
    #[must_use]
    pub fn skeleton(&self) -> &MolGraph {
        &self.skeleton
    }

    /// The data of `atom`, or `None` if `atom.index()` is out of range.
    #[must_use]
    pub fn atom_data(&self, atom: AtomIndex) -> Option<AtomData> {
        self.atom_data.get(atom.index()).copied()
    }

    /// The order of `bond`, or `None` if `bond.index()` is out of range.
    #[must_use]
    pub fn bond_order(&self, bond: BondIndex) -> Option<BondOrder> {
        self.bond_orders.get(bond.index()).copied()
    }

    // Conversions

    /// The [`MolGraph`] of this structure, with implicit hydrogens appended as
    /// explicit atoms.
    ///
    /// **Index rule:** The atom and bond indices match the original skeleton
    /// [`MolGraph`], with hydrogens and their bonds appended to the end of the
    /// atom and bond lists in the order of the atoms they're bonded to.
    ///
    /// The net charge is the sum of the atoms' formal charges. The net spin is
    /// the sum of the atoms' unpaired electrons. For each atom, take the
    /// electrons it uses (its bond-order sum plus its hydrogens) and the
    /// smallest default valence that is at least that many. The difference
    /// is its unpaired electrons, minus one for an aromatic atom. A charged
    /// atom has the valences of the neutral atom with as many electrons
    /// (atomic number minus charge), so N+ has those of C. Hydrogen has
    /// valence 1. Elements outside the SMILES organic subset have no unpaired
    /// electrons.
    ///
    /// # Examples
    ///
    /// ```
    /// use protochem::{AtomData, AtomIndex, BondOrder, LewisGraph};
    ///
    /// // The methyl radical, [CH3]
    /// let mut methyl = LewisGraph::new();
    /// methyl.add_atom(AtomData { hydrogens: 3, ..AtomData::new(6) }).unwrap();
    /// let graph = methyl.to_mol_graph();
    /// assert_eq!(graph.atom_count(), 4);
    /// assert_eq!(graph.spin(), 1);
    ///
    /// // Hydroxide, [OH-], keeps the oxygen at index 0
    /// let mut hydroxide = LewisGraph::new();
    /// hydroxide.add_atom(AtomData { charge: -1, hydrogens: 1, ..AtomData::new(8) }).unwrap();
    /// let graph = hydroxide.to_mol_graph();
    /// assert_eq!(graph.atomic_number(AtomIndex::new(0)), Some(8));
    /// assert_eq!((graph.charge(), graph.spin()), (-1, 0));
    /// ```
    #[must_use]
    #[expect(
        clippy::missing_panics_doc,
        reason = "adding a hydrogen atom and its one bond can't fail"
    )]
    pub fn to_mol_graph(&self) -> MolGraph {
        let bond_order_sums = self.bond_order_sums();
        let mut graph = self.skeleton.clone();
        let mut charge: i8 = 0;
        let mut spin: u8 = 0;
        for (atom, data) in self.skeleton.atoms().zip(&self.atom_data) {
            for _ in 0..data.hydrogens {
                let hydrogen = graph.add_atom(1).expect("hydrogen is an element");
                graph
                    .add_bond(atom, hydrogen)
                    .expect("just-added atom has no bonds");
            }
            // Add charge increment
            charge = charge.saturating_add(data.charge);
            // Calculate and add spin increment
            let unpaired = data.unpaired_electrons(bond_order_sums[atom.index()]);
            spin = spin.saturating_add(unpaired);
        }
        graph.set_charge(charge);
        graph.set_spin(spin);
        graph
    }

    // Derived properties

    /// The molecular formula in Hill order, counting the implicit hydrogens.
    ///
    /// See [`MolGraph::formula`] for the order.
    ///
    /// # Examples
    ///
    /// ```
    /// use protochem::{AtomData, LewisGraph};
    ///
    /// // Borane, BH3: no carbon, so the symbols are in alphabetical order
    /// let mut graph = LewisGraph::new();
    /// graph.add_atom(AtomData { hydrogens: 3, ..AtomData::new(5) }).unwrap();
    /// assert_eq!(graph.formula(), "BH3");
    /// ```
    #[must_use]
    pub fn formula(&self) -> String {
        let mut counts = self.skeleton.element_counts();
        let hydrogens: usize = self
            .atom_data
            .iter()
            .map(|data| usize::from(data.hydrogens))
            .sum();
        if hydrogens > 0 {
            *counts.entry(1).or_insert(0) += hydrogens;
        }
        hill_formula(&counts)
    }

    /// The sums of the bond-order valences (see [`BondOrder::valence`]), by
    /// [`AtomIndex`].
    fn bond_order_sums(&self) -> Vec<u8> {
        let mut sums = vec![0u8; self.skeleton.atom_count()];
        for (bond, order) in self.skeleton.bonds().zip(&self.bond_orders) {
            let atoms = self
                .skeleton
                .bond_atoms(bond)
                .expect("the skeleton's bonds should be in range");
            for atom in atoms {
                sums[atom.index()] = sums[atom.index()].saturating_add(order.valence());
            }
        }
        sums
    }
}

/// The data of one atom in a [`LewisGraph`].
///
/// Build one with [`AtomData::new`] and struct update syntax, or convert an
/// atomic number with `From`/`Into`. Both give a neutral, non-aromatic atom
/// with no hydrogens.
///
/// # Examples
///
/// ```
/// use protochem::AtomData;
///
/// // The oxygen of hydroxide, [OH-]
/// let oxygen = AtomData { charge: -1, hydrogens: 1, ..AtomData::new(8) };
/// assert_eq!(oxygen.atomic_number, 8);
/// assert_eq!(AtomData::from(8), AtomData::new(8));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AtomData {
    /// The atomic number, or `0` for the wildcard `*`.
    pub atomic_number: u8,
    /// The formal charge.
    pub charge: i8,
    /// The number of hydrogens attached to this atom that aren't atoms of the
    /// graph.
    pub hydrogens: u8,
    /// Whether the atom is aromatic: written lowercase in SMILES.
    pub aromatic: bool,
}

impl AtomData {
    /// A neutral, non-aromatic atom with no hydrogens.
    #[must_use]
    pub fn new(atomic_number: u8) -> Self {
        Self {
            atomic_number,
            charge: 0,
            hydrogens: 0,
            aromatic: false,
        }
    }

    pub(crate) fn unpaired_electrons(self, bond_order_sum: u8) -> u8 {
        let used = bond_order_sum.saturating_add(self.hydrogens);
        let isoelectronic = i16::from(self.atomic_number) - i16::from(self.charge);
        let Ok(atomic_number) = u8::try_from(isoelectronic) else {
            return 0;
        };
        let valences = match atomic_number {
            1 => &[1],
            z => element::organic_valences(z),
        };
        valences
            .iter()
            .find(|&&valence| valence >= used)
            .map_or(0, |valence| valence - used)
            .saturating_sub(u8::from(self.aromatic))
    }
}

impl From<u8> for AtomData {
    /// The atom [`AtomData::new`] gives for this atomic number.
    fn from(atomic_number: u8) -> Self {
        Self::new(atomic_number)
    }
}

/// The order of a bond in a [`LewisGraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BondOrder {
    /// Order 1.
    Single,
    /// Order 2.
    Double,
    /// Order 3.
    Triple,
    /// Order 4.
    Quadruple,
    /// A bond in an aromatic ring, whose order depends on the Kekulé structure.
    Aromatic,
}

impl BondOrder {
    /// The bond order as a number, or `None` for an aromatic bond.
    ///
    /// # Examples
    ///
    /// ```
    /// use protochem::BondOrder;
    ///
    /// assert_eq!(BondOrder::Double.order(), Some(2));
    /// assert_eq!(BondOrder::Aromatic.order(), None);
    /// ```
    #[must_use]
    pub fn order(self) -> Option<u8> {
        match self {
            Self::Single => Some(1),
            Self::Double => Some(2),
            Self::Triple => Some(3),
            Self::Quadruple => Some(4),
            Self::Aromatic => None,
        }
    }

    /// What each bond adds to each atom's valence, for counting hydrogens and
    /// unpaired electrons.
    ///
    /// An aromatic bond contributes a valence of 1, like a sigma bond, with the
    /// extra π valence of the aromatic atom determined from the
    /// [`AtomData::aromatic`] property. The distinction between
    /// `BondOrder::Aromatic` and `BondOrder::Single` is still important for
    /// distinguishing between bonds participating in the aromatic ring and
    /// bonds connecting separate aromatic systems (e.g. c1ccccc1-c2ccccc2).
    pub(crate) fn valence(self) -> u8 {
        self.order().unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_atom_stores_the_data() {
        let mut graph = LewisGraph::new();
        let ammonium = AtomData {
            charge: 1,
            hydrogens: 4,
            ..AtomData::new(7)
        };
        let atom = graph.add_atom(ammonium).unwrap();
        assert_eq!(graph.atom_data(atom), Some(ammonium));
        assert_eq!(graph.skeleton().atomic_number(atom), Some(7));
        assert_eq!(graph.atom_data(AtomIndex::new(1)), None);
    }

    #[test]
    fn add_atom_rejects_unknown_elements() {
        let mut graph = LewisGraph::new();
        let unknown = AtomData {
            charge: 1,
            ..AtomData::new(200)
        };
        assert_eq!(
            graph.add_atom(unknown),
            Err(GraphError::UnknownElement(200))
        );
        // A rejected atom changes nothing
        assert_eq!(graph, LewisGraph::new());
    }

    #[test]
    fn add_bond_stores_the_order() {
        let mut graph = LewisGraph::new();
        let carbon = graph.add_atom(6).unwrap();
        let oxygen = graph.add_atom(8).unwrap();
        let bond = graph.add_bond(carbon, oxygen, BondOrder::Double).unwrap();
        assert_eq!(graph.bond_order(bond), Some(BondOrder::Double));
        assert_eq!(graph.bond_order(BondIndex::new(1)), None);
    }

    #[test]
    fn add_bond_rejects_duplicates() {
        let mut graph = LewisGraph::new();
        let carbon = graph.add_atom(6).unwrap();
        let oxygen = graph.add_atom(8).unwrap();
        let bond = graph.add_bond(carbon, oxygen, BondOrder::Double).unwrap();
        let before = graph.clone();
        for (atom1, atom2) in [(carbon, oxygen), (oxygen, carbon)] {
            assert_eq!(
                graph.add_bond(atom1, atom2, BondOrder::Single),
                Err(GraphError::DuplicateBond(bond))
            );
        }
        // A rejected bond gets no order
        assert_eq!(graph, before);
    }

    #[test]
    fn atom_data_from_atomic_number() {
        let nitrogen: AtomData = 7.into();
        assert_eq!(nitrogen, AtomData::new(7));
        assert_eq!(nitrogen.charge, 0);
        assert_eq!(nitrogen.hydrogens, 0);
        assert!(!nitrogen.aromatic);
    }

    #[test]
    fn graphs_with_different_data_differ() {
        let mut methane = LewisGraph::new();
        methane
            .add_atom(AtomData {
                hydrogens: 4,
                ..AtomData::new(6)
            })
            .unwrap();
        let mut carbon = LewisGraph::new();
        carbon.add_atom(6).unwrap();
        // Same skeleton, different Lewis data
        assert_eq!(methane.skeleton(), carbon.skeleton());
        assert_ne!(methane, carbon);
    }

    #[test]
    fn formula_counts_implicit_and_explicit_hydrogens() {
        // Methanol, with three implicit methyl hydrogens and an explicit H atom
        let mut graph = LewisGraph::new();
        let carbon = graph
            .add_atom(AtomData {
                hydrogens: 3,
                ..AtomData::new(6)
            })
            .unwrap();
        let oxygen = graph.add_atom(8).unwrap();
        let hydrogen = graph.add_atom(1).unwrap();
        graph.add_bond(carbon, oxygen, BondOrder::Single).unwrap();
        graph.add_bond(oxygen, hydrogen, BondOrder::Single).unwrap();
        assert_eq!(graph.formula(), "CH4O");
        assert_eq!(graph.skeleton().formula(), "CHO");
    }

    #[test]
    fn formula_of_an_empty_graph_is_empty() {
        assert_eq!(LewisGraph::new().formula(), "");
    }

    #[test]
    fn bond_order_order_and_valence() {
        let orders = [
            BondOrder::Single,
            BondOrder::Double,
            BondOrder::Triple,
            BondOrder::Quadruple,
            BondOrder::Aromatic,
        ];
        let numbers: Vec<_> = orders.iter().map(|order| order.order()).collect();
        assert_eq!(numbers, [Some(1), Some(2), Some(3), Some(4), None]);
        let valences: Vec<_> = orders.iter().map(|order| order.valence()).collect();
        assert_eq!(valences, [1, 2, 3, 4, 1]);
    }

    /// A graph of these atoms, with a single bond between each pair of
    /// positions in `bond_atoms`.
    fn lewis_graph(atom_data: &[AtomData], bond_atoms: &[[usize; 2]]) -> LewisGraph {
        let mut graph = LewisGraph::new();
        for &data in atom_data {
            graph.add_atom(data).unwrap();
        }
        for &[atom1, atom2] in bond_atoms {
            graph
                .add_bond(
                    AtomIndex::new(atom1),
                    AtomIndex::new(atom2),
                    BondOrder::Single,
                )
                .unwrap();
        }
        graph
    }

    /// An atom with this charge and hydrogen count.
    fn atom(atomic_number: u8, charge: i8, hydrogens: u8) -> AtomData {
        AtomData {
            charge,
            hydrogens,
            ..AtomData::new(atomic_number)
        }
    }

    /// The charge and spin of a one-atom graph.
    fn charge_and_spin(data: AtomData) -> (i8, u8) {
        let graph = lewis_graph(&[data], &[]).to_mol_graph();
        (graph.charge(), graph.spin())
    }

    #[test]
    fn to_mol_graph_appends_hydrogens_by_the_index_rule() {
        // Ethanol, C0-C1-O2, as in MolGraph's `ethanol()` fixture
        let graph = lewis_graph(
            &[atom(6, 0, 3), atom(6, 0, 2), atom(8, 0, 1)],
            &[[0, 1], [1, 2]],
        );
        let atomic_numbers = [6, 6, 8, 1, 1, 1, 1, 1, 1];
        let bond_atoms = [
            [0, 1],
            [1, 2],
            [0, 3],
            [0, 4],
            [0, 5],
            [1, 6],
            [1, 7],
            [2, 8],
        ];
        let expected = MolGraph::from_bonds(&atomic_numbers, &bond_atoms).unwrap();
        assert_eq!(graph.to_mol_graph(), expected);
    }

    #[test]
    fn to_mol_graph_counts_hydrogen_counts_and_hydrogen_atoms() {
        // Methanol: three methyl hydrogens as a count, the hydroxyl one as an atom
        let methanol = lewis_graph(
            &[atom(6, 0, 3), atom(8, 0, 0), atom(1, 0, 0)],
            &[[0, 1], [1, 2]],
        );
        assert_eq!(methanol.to_mol_graph().formula(), "CH4O");
        let borane = lewis_graph(&[atom(5, 0, 3)], &[]);
        assert_eq!(borane.to_mol_graph().formula(), "BH3");
    }

    #[test]
    fn to_mol_graph_closed_shells_have_no_spin() {
        let cases = [
            ("CH4", atom(6, 0, 4)),
            ("BH3", atom(5, 0, 3)),
            ("NH4+", atom(7, 1, 4)),
            ("BH4-", atom(5, -1, 4)),
            ("H3O+", atom(8, 1, 3)),
            ("CH3+", atom(6, 1, 3)),
            ("CH3-", atom(6, -1, 3)),
            ("H-", atom(1, -1, 0)),
            ("H+", atom(1, 1, 0)),
            ("Ne", atom(10, 0, 0)),
            ("Na+", atom(11, 1, 0)),
            ("Fe", atom(26, 0, 0)),
        ];
        for (name, data) in cases {
            assert_eq!(charge_and_spin(data), (data.charge, 0), "{name}");
        }
    }

    #[test]
    fn to_mol_graph_counts_unpaired_electrons() {
        let cases = [
            ("CH3", atom(6, 0, 3), 1),
            ("CH2", atom(6, 0, 2), 2),
            ("OH", atom(8, 0, 1), 1),
            ("H", atom(1, 0, 0), 1),
            ("O", atom(8, 0, 0), 2),
        ];
        for (name, data, spin) in cases {
            assert_eq!(charge_and_spin(data), (0, spin), "{name}");
        }
        // Triplet O2, with a single bond between the oxygens
        let oxygen = lewis_graph(&[atom(8, 0, 0), atom(8, 0, 0)], &[[0, 1]]);
        assert_eq!(oxygen.to_mol_graph().spin(), 2);
    }

    #[test]
    fn to_mol_graph_sums_the_charges() {
        let salt = lewis_graph(&[atom(11, 1, 0), atom(17, -1, 0)], &[]);
        let graph = salt.to_mol_graph();
        assert_eq!((graph.charge(), graph.spin()), (0, 0));
    }

    /// A six-membered aromatic carbon ring, with these hydrogen counts.
    fn aromatic_ring(hydrogens: [u8; 6]) -> LewisGraph {
        let mut graph = LewisGraph::new();
        let atoms: Vec<_> = hydrogens
            .iter()
            .map(|&hydrogens| {
                let data = AtomData {
                    hydrogens,
                    aromatic: true,
                    ..AtomData::new(6)
                };
                graph.add_atom(data).unwrap()
            })
            .collect();
        for (i, &atom) in atoms.iter().enumerate() {
            let next = atoms[(i + 1) % atoms.len()];
            graph.add_bond(atom, next, BondOrder::Aromatic).unwrap();
        }
        graph
    }

    #[test]
    fn to_mol_graph_aromatic_atoms() {
        let benzene = aromatic_ring([1; 6]).to_mol_graph();
        assert_eq!(benzene.formula(), "C6H6");
        assert_eq!(benzene.spin(), 0);
        // Phenyl radical: one carbon without its hydrogen
        let phenyl = aromatic_ring([0, 1, 1, 1, 1, 1]).to_mol_graph();
        assert_eq!(phenyl.spin(), 1);
    }
}
