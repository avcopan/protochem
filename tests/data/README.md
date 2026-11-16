# Test data

Reference data sets for the oracle tests (`pixi run oracle`), copied verbatim from their
sources. Each file keeps its source's name and license.

| file | entries | what it is | source | license |
|---|---|---|---|---|
| `kegg_smiles.txt` | 5676 SMILES | from the KEGG Drug Database, one SMILES per line | [indigo-bondorder](https://github.com/allison-group/indigo-bondorder) `data/`, commit `77b1574` | MIT, © 2017 allison-group |
| `mmff94_smiles.txt` | 691 SMILES | from the MMFF94 validation suite, one SMILES per line | same | same |
| `failed_smiles.txt` | 126 SMILES | recorded failures (the source doesn't describe them), one SMILES per line | same | same |
| `kinbot.csv` | 346 rows | symmetry numbers: `inchi`, `kinbot_value`, `expected_value` | [Experiments-SymmetryNumbers](https://github.com/avcopan/Experiments-SymmetryNumbers) `data/`, commit `d323b66` | MIT, © 2026 Maxim Papusha and Andreas V. Copan |
| `sigma.csv` | 346 rows | symmetry numbers: `inchi`, `sigma_expected`, `sigma_calculated` | same | same |

Notes:

- The indigo-bondorder files are the test sets of Welsh & Allison, *J. Cheminform.* 11, 18
  (2019), [doi:10.1186/s13321-019-0340-0](https://doi.org/10.1186/s13321-019-0340-0), which
  uses the MMFF94 validation suite and the KEGG Drug Database as reference sets. Some
  lines end in a space, as in the source.
- `kinbot.csv` escapes the commas inside InChIs with a backslash instead of quoting them, so
  read it with `csv.reader(..., escapechar="\\")`.
