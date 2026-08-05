# track/cip108-guardrails — structural guardrails per CIP-108

Read first: `cips/accepted/0108-structural-guardrails.md` (decision) and
`.dev/audit-2026-08-05.md` §P2 guardrails. Execute exactly the CIP:
exhaustive crate-root module maps (every `mod` + one ownership
sentence; cheap declaration-vs-map check in the source-size gate;
explicit intentional omissions), source-size output reporting
live/inline-test/total with the total ceiling retained and a
test-module-extraction diagnostic, shared-state inventory added to the
audit checklist, and rationale comments added at the five sites the
audit named (scratch `Once`, tour port atomic, index-test `Arc`, two
compose-test `RefCell`s). NOTE: the codebase moved since the audit
(cip97/cip103-leg1/cip105 merged — module maps may already be partially
updated; reconcile against current reality, not the audit snapshot).

Discipline: branch `track/cip108-guardrails`, LOG
`crates/cix-build/LOG.md`; full agent gate tier, capture-as-epilogue
for long runs (never pre-touch; non-empty numeric captures only);
VM matrix with bounded parallelism (--max-jobs 2 --cores 2) to avoid
the recorded parallel-TCG mastodon flake. Parallel tracks in flight —
merge semantically. Clean branch; do not merge.
