# Tutorials

Stage 1 tutorials (see `~/code/sm/PLAN.md` for the full sequence and status). Each folder
has a `README.md` (background, reading, step-by-step tasks) and Claude's spec
`.rs` file(s), used when you say "fill in".

| # | Folder | Status |
|---|---|---|
| T01 | `t01-setup-and-match/` | done |
| T02 | `t02-enums-smiles-lexer/` | done |
| T03 | `t03-ownership-zero-copy-lexer/` | done |
| T04 | `t04-collections-errors-iterators/` | ready |
| T05 | traits and generics: SMILES parser + `Follower` | – |
| T06 | ~~iterators and closures~~: folded into T02 (syntax) and T04 (`neighbors`, `bfs_layers`) | – |
| T07 | hashing and determinism: 1-WL color refinement | – |
| T08 | backtracking: isomorphism search + petgraph traits | – |
| T09 | PyO3 and the Python oracle | – |
| T10 | bridge: tetrahedral local stereodescriptors | – |

The tests in each stub were checked against a reference solution, so if a test
looks wrong, it's worth a second look at the spec before changing the test (but do
tell me if you think it's wrong).
