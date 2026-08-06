# track/ittools-relock — it-tools: CIP-99 re-lock + runtime proof

Case: corpus/migrate/docker/it-tools (GAPS.md says
`Status: stale — regenerate with CIP-99 workspace-root aggregation`).
Prior state: builds green, but the lock predates CIP-99 lock-subtree
aggregation, and runtime was honestly recorded as unproved past a
600s bound.

Do two things, each with synchronous captured receipts:
1. Re-lock with current cix (CIP-99 aggregation live): rebuild from
   scratch, commit the new Cixfile.lock, record the lock line-count
   delta in receipt.md (aggregation should shrink it substantially —
   parse-server went 197,888 -> 54,915).
2. Runtime proof: run the built app via check.sh (or extend it) and
   capture an actual HTTP 200 (or equivalent probe) from the running
   service — the previous receipt stopped at the 600s build bound.
   If runtime genuinely cannot be proven (document WHY: missing
   asset, port behavior, timeout), record that honestly in GAPS.md
   and receipt.md instead of claiming green.

Update GAPS.md status accordingly (drop the stale marker; state what
is now verified vs still open) and re-grade the docs/corpus.md
it-tools row in the same track — desk grades vs verified receipts
stay honestly distinguished.

Discipline: branch `track/ittools-relock` from current main, LOG in
the case directory or crates/cix/LOG.md (append). Gates: fmt +
corpus-affected checks; the full VM matrix is NOT needed for a
corpus-only track — run `cargo test --workspace` once if you touch
any Rust (you should not need to). FRICTION section: record anything
about the Cixfile language or cix CLI that was not immediately
intuitive. Value-checked synchronous captures only. Clean committed
branch; do not merge.
