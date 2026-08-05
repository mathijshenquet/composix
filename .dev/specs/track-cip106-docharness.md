# track/cip106-docharness — doc harness thinning per CIP-106

Read first: `cips/accepted/0106-doc-harness-thinning.md` (decision) and
`.dev/audit-2026-08-05.md` §P1 doc harness. Execute exactly the CIP:
one doc-generation support library/tool (GeneratedFile, drift compare,
atomic write-to-sibling-then-rename); tour chapters as scenario
modules with command/cleanup and normalization harnesses separated;
ONE ordinary render supplies the drift receipt (explicit determinism
test may render twice — normal tests stop rendering the tour four
times); corpus.rs split (discovery/ledger parsing, highlighting,
templates); shared integration-test helpers; collapse byte-identical
system/user golden fixtures where mode makes no difference.

Byte-for-byte generated output is the acceptance: tour and browser
pages identical before/after (except where the CIP names atomicity).

Discipline: branch `track/cip106-docharness`, LOG `crates/cix/LOG.md`;
full agent gate tier, capture-as-epilogue receipts, bounded VM
parallelism (--max-jobs 2 --cores 2). Parallel tracks in flight —
merge semantically. Clean branch; do not merge.
