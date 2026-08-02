# track/fetchself — CIP-87 amendment: FETCH self-observation rule

Read AGENTS.md first (focused agent gate; synchronous receipts;
shared-state rule). Authoritative: the newly adopted CIP-87 changelog
entry "FETCH self-observation rule" — its two halves and ALL FOUR
load-bearing conditions are the contract; also read the original
FINDING in crates/cix-cixfile/LOG.md (tracefast entries) for the
receipts and mechanism. Work in
`/home/mathijs/worktrees/composix/track-fetchself` (herdr worktree) on
branch `track/fetchself`. Keep `crates/cix-cixfile/LOG.md` current.
Your domain: crates/cix-build (build_chain/trace/memo). FENCE:
track/thin2 runs concurrently in cix-compose — do not touch it.

1. Half (1): a FETCH about to execute reverts the paths its superseded
   memo recorded as its own writes, before the pre-state snapshot.
2. Half (2): validation's self-read exception, implementing all four
   conditions verbatim (per-path content hash; full-write-set
   constructive apply on hit; cold-replay precedence — cold never
   consults the exception; same-memo scope only).
3. Remove the two-prime workaround from
   examples/compare/gitsitter/measure-warm.sh — the rule makes it
   unnecessary; the cold control in that harness must go green.
4. Tests: warm and cold recorded traces byte-identical by construction
   on the mini-fixture; the cold control green; partial-self-state
   (delete one written file) → revert + re-execute; and the a/b
   adversarial case from review: FETCH a writes ./foo, RUN b reads it —
   prove b NEVER hits via the exception (b's read validates only
   against b's own fingerprint), across both a-hit and a-reexecuted
   states, including the pin-mismatch path when a's output drifts
   without --update-lock.
5. Re-run the warm benchmark once — the number must hold (~8.3s) or
   any regression be attributed.

Gate (agent side): fmt / examples fmt / clippy / workspace tests /
tour regen + drift / focused: the cix-build suite + measure-warm
receipts. Full matrix at the orchestrator gate. Commit when green.
