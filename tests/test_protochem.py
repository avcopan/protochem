"""protochem tests."""

import pytest

import protochem
from protochem import Element


def test_stub() -> None:
    """Stub test to ensure the test suite runs."""
    print(protochem.__version__)  # noqa: T201


def test__element_from_symbol() -> None:
    """Test looking up an element by symbol."""
    carbon = Element.from_symbol("C")
    assert carbon is not None
    assert carbon.atomic_number == 6
    assert carbon.symbol == "C"
    assert repr(carbon) == "Element('C')"
    assert Element.from_symbol("CL") is None  # case-sensitive


def test__element_from_atomic_number() -> None:
    """Test looking up an element by atomic number."""
    chlorine = Element.from_atomic_number(17)
    assert chlorine is not None
    assert chlorine.symbol == "Cl"
    assert Element.from_atomic_number(0) is None
    # The Rust argument is a u8, so PyO3 rejects values outside 0..=255
    with pytest.raises(OverflowError):
        Element.from_atomic_number(256)


def test__organic_valences() -> None:
    """Test the SMILES organic-subset valences."""
    assert protochem.organic_valences(7) == [3, 5]  # a list, not bytes
    assert protochem.organic_valences(26) == []
