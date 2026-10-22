// The docstrings below are Python's (Google style), whose `Args:` entries name
// parameters without backticks
#![allow(clippy::doc_markdown)]

// The PyO3 prelude brings the `pymodule` attribute macro into scope.
use pyo3::prelude::*;

/// The compiled Python module, imported as `<package_name>._core`.
///
/// Its type stubs (`_core.pyi`) are generated from this module by `pixi run stubs`, which
/// includes these doc comments as docstrings, so they use Python's (Google) docstring style.
#[pymodule]
mod _core {
    // Import the prelude again, since `use` does not carry into child modules.
    use pyo3::prelude::*;

    use crate::element;

    /// The package version.
    #[pymodule_export]
    #[allow(non_upper_case_globals)] // Python's naming convention
    const __version__: &str = env!("CARGO_PKG_VERSION");

    /// A chemical element.
    ///
    /// Look one up with `Element.from_symbol` or `Element.from_atomic_number`.
    #[pyclass(frozen, name = "Element")]
    struct PyElement(&'static element::Element);

    #[pymethods]
    impl PyElement {
        /// Look up an element by symbol.
        ///
        /// Symbols are case-sensitive, as in SMILES: "Cl" is chlorine, but "CL" is not.
        ///
        /// Args:
        ///     symbol: The chemical symbol.
        ///
        /// Returns:
        ///     The element, or None if there is no element with this symbol.
        #[staticmethod]
        fn from_symbol(symbol: &str) -> Option<Self> {
            element::Element::from_symbol(symbol).map(Self)
        }

        /// Look up an element by atomic number.
        ///
        /// Args:
        ///     atomic_number: The atomic number.
        ///
        /// Returns:
        ///     The element, or None if the atomic number is out of range.
        #[staticmethod]
        fn from_atomic_number(atomic_number: u8) -> Option<Self> {
            element::Element::from_atomic_number(atomic_number).map(Self)
        }

        /// The atomic number.
        #[getter]
        fn atomic_number(&self) -> u8 {
            self.0.atomic_number
        }

        /// The chemical symbol.
        #[getter]
        fn symbol(&self) -> &'static str {
            self.0.symbol
        }

        fn __repr__(&self) -> String {
            format!("Element('{}')", self.0.symbol)
        }
    }

    /// Return the allowed valences of an element in the SMILES organic subset.
    ///
    /// Args:
    ///     atomic_number: The atomic number.
    ///
    /// Returns:
    ///     The allowed valences, in increasing order, or an empty list if the element
    ///     is not in the organic subset.
    #[pyfunction]
    fn organic_valences(atomic_number: u8) -> Vec<u32> {
        // Convert from u8, since PyO3 turns a `Vec<u8>` into `bytes`, not `list[int]`
        element::organic_valences(atomic_number)
            .iter()
            .map(|&valence| u32::from(valence))
            .collect()
    }
}
