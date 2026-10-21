# Tutorials

Stage 1 tutorials (see `~/code/sm/PLAN.md` for the full sequence and status). Each folder
has a `README.md` (background, reading, tasks) and a stub module that you copy into
the sandbox's `src/`, then fill in until its tests pass.

| # | Folder | Status |
|---|---|---|
| T01 | `t01-setup-and-match/` | ready |
| T02 | `t02-enums-smiles-lexer/` | ready |
| T03 | ownership, borrowing, lifetimes: zero-copy lexer | written after T02 review |
| T04 | collections and errors: `MolGraph` v0 | – |
| T05 | traits and generics: SMILES parser + `Follower` | – |
| T06 | iterators and closures: BFS layers | – |
| T07 | hashing and determinism: 1-WL color refinement | – |
| T08 | backtracking: isomorphism search + petgraph traits | – |
| T09 | PyO3 and the Python oracle | – |
| T10 | bridge: tetrahedral local stereodescriptors | – |

The tests in each stub were checked against a reference solution, so if a test
looks wrong, it's worth a second look at the spec before changing the test (but do
tell me if you think it's wrong).
