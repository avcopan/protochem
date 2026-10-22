"""
The compiled Python module, imported as `<package_name>._core`.

Its type stubs (`_core.pyi`) are generated from this module by `pixi run stubs`, which
includes these doc comments as docstrings, so they use Python's (Google) docstring style.
"""

from typing import Final, final

__version__: Final[str]
"""
The package version.
"""

@final
class Element:
    """
    A chemical element.

    Look one up with `Element.from_symbol` or `Element.from_atomic_number`.
    """
    def __repr__(self, /) -> str: ...
    @property
    def atomic_number(self, /) -> int:
        """
        The atomic number.
        """
    @staticmethod
    def from_atomic_number(atomic_number: int) -> Element | None:
        """
        Look up an element by atomic number.

        Args:
            atomic_number: The atomic number.

        Returns:
            The element, or None if the atomic number is out of range.
        """
    @staticmethod
    def from_symbol(symbol: str) -> Element | None:
        """
        Look up an element by symbol.

        Symbols are case-sensitive, as in SMILES: "Cl" is chlorine, but "CL" is not.

        Args:
            symbol: The chemical symbol.

        Returns:
            The element, or None if there is no element with this symbol.
        """
    @property
    def symbol(self, /) -> str:
        """
        The chemical symbol.
        """

def organic_valences(atomic_number: int) -> list[int]:
    """
    Return the allowed valences of an element in the SMILES organic subset.

    Args:
        atomic_number: The atomic number.

    Returns:
        The allowed valences, in increasing order, or an empty list if the element
        is not in the organic subset.
    """
