# track/lockagg — why CIP-99 aggregation missed it-tools/homer (investigate, then fix or re-lock)

Context: expand1 landed `corpus/migrate/docker/it-tools/Cixfile.lock`
at 1.54M lines and homer's at 146k — the CIP-99 subtree aggregation
(merged today, exhibits: parse-server 197,888->54,915) did not bite.
Hypothesis: aggregation requires complete traced subtrees and these
builds walled/were partial (homer's pnpm wall; it-tools' 600s-bounded
runtime). Read `cips/accepted/0099-lock-scale.md`, the aggregation
implementation (crates/cix-build trace/lock), and both locks.

Task: (1) diagnose precisely why each lock stayed unaggregated —
criteria gap vs needs-re-lock vs legitimately incompressible; (2) if
re-lock suffices, regenerate the two locks with receipts
(before/after line counts; byte-identical store outputs where builds
complete); (3) if it is a criteria gap (e.g. partial-build traces
never aggregate), implement the minimal fix per CIP-99's decision and
append a dated changelog line; (4) record the verdict in the LOG and
docs/corpus.md rows if evidence changes. Do NOT force aggregation
that would hide volatility.

Discipline: branch `track/lockagg`, LOG `crates/cix-cixfile/LOG.md`;
full agent tier, value-checked captures, bounded VM parallelism,
df-guard before big builds. Merge semantically. Clean branch; do not
merge.
