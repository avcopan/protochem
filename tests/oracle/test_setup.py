"""Check that the oracle environment and the test data are in place."""

import csv
from pathlib import Path

import pytest

DATA = Path(__file__).parent.parent / "data"


@pytest.mark.parametrize("module", ["rdkit", "networkx", "stereomolgraph"])
def test__oracle_imports(module: str) -> None:
    """Test that each reference implementation imports (skipped outside `oracle`)."""
    pytest.importorskip(module)


@pytest.mark.parametrize(
    ("name", "count"),
    [("kegg_smiles.txt", 5676), ("mmff94_smiles.txt", 691), ("failed_smiles.txt", 126)],
)
def test__smiles_files(name: str, count: int) -> None:
    """Test that each SMILES file has one SMILES per line."""
    lines = (DATA / name).read_text().splitlines()
    smiles = [line.split()[0] for line in lines if line.strip()]
    assert len(smiles) == count


@pytest.mark.parametrize(
    ("name", "header"),
    [
        ("kinbot.csv", ["inchi", "kinbot_value", "expected_value"]),
        ("sigma.csv", ["inchi", "sigma_expected", "sigma_calculated"]),
    ],
)
def test__symmetry_number_files(name: str, header: list[str]) -> None:
    """Test that each symmetry-number file has 346 InChIs with integer values."""
    with (DATA / name).open(newline="") as file:
        rows = list(csv.reader(file, escapechar="\\"))
    assert rows[0] == header
    assert len(rows) == 1 + 346
    for inchi, *values in rows[1:]:
        assert inchi.startswith("InChI=")
        assert all(value.isdigit() for value in values)
