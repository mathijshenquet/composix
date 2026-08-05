# track/cip103-leg1 — build-chain residue deletion + test moves (S leg)

Read first: `cips/accepted/0103-build-chain-seams.md` (decision; this
track is leg 1 only). Scope, strictly:

1. Delete the commented FETCH-consent copy in
   `crates/cix-build/src/build_chain.rs` (the `/* … */` block, lines
   ~162–411 at commit 8a081e6) — the live implementation is fetch.rs.
2. Delete the permanently disabled tests the audit found (148 lines) —
   coordinate with CIP-107's later track: only zero-exhibit DISABLED
   code, nothing live.
3. Move `build_chain`'s inline `mod tests` (~772 lines) to a sibling
   test module/file per repo convention, without changing any
   assertion or fixture.
4. Update cix-build's crate-root module map.

NOT in scope: any extraction of Workspace/MemoEngine/context/FETCH
owners (those are the M/L legs, separate tracks). The diff should be
deletions + moves only; behavior byte-identical by construction, but
run the full agent gate tier anyway (capture-as-epilogue pattern,
never pre-touch, non-empty numeric captures only). Shared manager is
contended — wait your turn and log it. Branch `track/cip103-leg1`,
LOG `crates/cix-build/LOG.md`. Commit granularly; clean branch; do
not merge.
