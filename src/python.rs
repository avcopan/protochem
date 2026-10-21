use pyo3::prelude::*;

/// The compiled Python module, imported as `<package_name>._core`.
///
/// Its type stubs (`_core.pyi`) are generated from this module by `pixi run stubs`, which
/// includes these doc comments as docstrings, so they use Python's (Google) docstring style.
#[pymodule]
mod _core {
    use pyo3::prelude::*;

    /// The package version.
    #[pymodule_export]
    #[allow(non_upper_case_globals)] // Python's naming convention
    const __version__: &str = env!("CARGO_PKG_VERSION");

    /// Greet a person by their name.
    ///
    /// Args:
    ///     name: The name of the person to greet.
    ///
    /// Returns:
    ///     A greeting message.
    #[pyfunction]
    fn greet(name: &str) -> String {
        crate::greet(name)
    }
}
