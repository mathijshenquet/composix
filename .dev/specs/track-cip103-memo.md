# track/cip103-memo — CIP-103 leg 3: the MemoEngine owner

Read first: `cips/accepted/0103-build-chain-seams.md` (decision; this
is the L leg — "its acceptance condition is a genuinely narrow owned
interface rather than fewer lines") and `crates/cix-build/src/
workspace.rs` (leg 2's owner, merged today — MemoEngine may operate
THROUGH Workspace, never around it).

Extract a `MemoEngine` owner from build_chain: key construction,
memo validation, read/write-set reduction, cold comparison,
constructive replay (the step_key/step_memo_key/validate_step_memo/
memo_write_set_matches/compare_cold_paths/memo_entry/
builder_step_results cluster). The acceptance is the INTERFACE: no
bag-of-fields context crosses the seam; build_chain hands MemoEngine
typed requests and receives typed verdicts; document the interface in
the module map. Byte-identical lock/output receipts on a
representative corpus case (before/after, recorded). If a truly
narrow interface forces a design choice the CIP does not answer,
STOP-and-flag that part rather than widening the seam.

Discipline: branch `track/cip103-memo`, LOG `crates/cix-build/LOG.md`;
full agent tier (contract-keyed selector), value-checked captures,
bounded VM parallelism, coordinate shared-manager/VM axes with the
other running tracks (wait and log if occupied). Merge semantically —
cip109-probeurl and lockagg are in flight. Clean branch; do not merge.
