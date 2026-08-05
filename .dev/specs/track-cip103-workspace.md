# track/cip103-workspace — CIP-103 leg 2: the Workspace owner

Read first: `cips/accepted/0103-build-chain-seams.md` (decision incl.
the two amendments) and `.dev/audit-2026-08-05.md` §P0 build-chain.
This track is the WORKSPACE leg only.

Extract a `Workspace` owner from `crates/cix-build/src/build_chain.rs`
per the CIP: persisted state, staging, snapshots, tree reconciliation,
node hashing/fingerprints, and store materialization (the audit mapped
the cluster around lines ~2438–2880 pre-leg1: workspace_paths,
load/save_workspace_state, replace_workspace_tree, restore_snapshot,
stage_input, sync_directories/sync_node, nodes_equal, copy_node,
make_writable, ensure_store_path, workspace_identity, collect_files,
file_fingerprint). Owned interface, not a bag-of-fields context —
build_chain calls Workspace methods; no shared mutable context struct
crosses the seam. Pure move + interface shaping: byte-identical
lock/output receipts are the acceptance (same store paths, same lock
bytes on a representative corpus case, recorded in the LOG).

NOT in scope: MemoEngine, context/sandbox boundaries, FETCH-state
(later legs). Update the crate module map (CIP-108 check enforces it).

Discipline: branch `track/cip103-workspace`, LOG
`crates/cix-build/LOG.md`; full agent gate tier with the NEW
contract-keyed progressive selector (it will select runtime/build
scenarios for this diff — that is correct), capture-as-epilogue
value-checked receipts; bounded VM parallelism. Parallel tracks in
flight (cip99 lock representation — coordinate on cix-build/lock.rs if
you both touch it; merge semantically). Clean branch; do not merge.
