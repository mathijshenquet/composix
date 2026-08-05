# track/cip105-runtime — runtime thinning per CIP-105

Read first: `cips/accepted/0105-runtime-thinning.md` (decision incl.
the proof-not-confirm sharpening) and `.dev/audit-2026-08-05.md` §P1
runtime. Execute exactly the CIP: delete the superseded `ps` path WITH
proof (grep receipt over the workspace + compile of all dependents),
extract `target`/`app`/`manager` modules, move the `unit.rs` golden
tests to a submodule, update the run crate's module map exhaustively.
Acceptance is byte-identical: unit text, degradation order, app exit
propagation, listener lifecycle, CLI output. No file needs a
source-size exception afterward.

Discipline: branch `track/cip105-runtime`, this worktree; LOG
`crates/cix-run/LOG.md`; full agent gate tier with `.gate-exit`
capture-as-epilogue for long runs (never pre-touch the file; only a
non-empty numeric capture is a receipt); the shared user manager is
contended — coordinate: if another suite runs, wait, and say so in
your LOG. Parallel tracks in flight; resolve merges semantically.
Commit granularly; clean branch; do not merge.
