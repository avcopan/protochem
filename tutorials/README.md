# Tutorials

Stage 1 tutorials (see `PLAN.md` in the private `sm` workspace repo, `~/proj/sm/PLAN.md`,
for the full sequence). Each folder
has a `README.md` (background, reading, step-by-step tasks) and Claude's spec
`.rs` file(s), used when you say "fill in".

| # | Folder | Status |
|---|---|---|
| T01 | `t01-setup-and-match/` | done |
| T02 | `t02-enums-smiles-lexer/` | done |
| T03 | `t03-ownership-zero-copy-lexer/` | done |
| T04 | `t04-collections-errors-iterators/` | ready |
| T05 | traits and generics: SMILES parser + `Follower` | – |
| T06 | hashing and determinism: color refinement | – |
| T07 | backtracking: VF2++ isomorphism | – |
| T08 | oracles: PyO3 bindings and property tests | – |
| T09 | matching I: augmenting paths | – |
| T10 | matching II: Edmonds blossom | – |
| T11 | kekulization and the rigid-planar shortcut | – |
| T12 | tree decompositions: elimination orders, GreedyFillIn | – |
| T13 | nice tree decompositions | – |
| T14 | dynamic programming over nice tree decompositions | – |
| T15 | permutations and local stereodescriptors | – |
| T16 | `StereoMolGraph` v0 | – |
| T17 | stereo SMILES | – |
| T18 | stereoisomer enumeration | – |
| T19 | permutation groups II: Schreier–Sims | – |
| T20 | canonical labeling: individualization–refinement | – |
| T21 | automorphism pruning and symmetry numbers | – |

The tests in each stub were checked against a reference solution, so if a test
looks wrong, it's worth a second look at the spec before changing the test (but do
tell me if you think it's wrong).
