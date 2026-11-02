//! A molecular graph with dense atom and bond indices.

/// The index of an atom in a molecular graph.
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

/// The index of a bond in a molecular graph.
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
