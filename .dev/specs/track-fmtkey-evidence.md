# track/fmtkey-evidence — pin the keying-leak mechanism behind the three exhibits

Charter: `cips/draft/fmt-key-neutrality.md`. Three recorded exhibits of
sourceHash churn on GREEN verifies (identical content, dirty lock):
- ntfy: `.dev/LOG.md` 2026-08-06 night entry ("same content hash +
  storePath, but sourceHash changed with dev/inode fingerprints").
- nginx: `crates/cix/LOG.md` "coldreplay sweep" section — exact old/new
  sourceHash pair recorded; cold exit 0.
- tomcat: same section — sourceHash AND storePath churned; cold exit 0.

This track is EVIDENCE ONLY — no product changes. Deliverables:

1. **Causal chain, proven not guessed**: for each exhibit class, show
   exactly which fingerprint fields flow into which key. Known code
   anchors to start from: `build_fingerprint` + `hash_source_tree`
   (crates/cix-cixfile/src/build.rs — serializes lock inputs/
   artifacts/fetches/dev_envs into sourceHash), `read_hash`
   (crates/cix-build/src/trace.rs, folds full `st_mode` bytes in),
   `file_fingerprint` (same file: dev/inode/mtime_ns/size/mode),
   FetchPin/lock serialization (crates/cix-build/src/lock.rs).
2. **Hermetic repro tests**: minimal tests demonstrating each leak
   (e.g. chmod a non-exec bit → key changes with identical content;
   copy a file to new inode/mtime → which keys move). Mark clearly as
   characterization tests of CURRENT behavior.
3. **Inventory table written into the draft** (evidence chapter, not
   adoption): every fingerprint/key site vs NAR semantics (file type +
   content + exec bit + symlink target; nothing else) — which fields
   each site uses today, which exhibit it explains, what would change
   under NAR-invariant keying.

Gates: fmt, warning-denied clippy, `cargo test --workspace` (with the
new tests). No VM (no behavior change).

Discipline: branch `track/fmtkey-evidence`, LOG `crates/cix/LOG.md`
(append, timestamped, FRICTION section). Synchronous value-checked
receipts only. Clean committed branch; do not merge.
