# track/cip107-prune — alpha surface prune per CIP-107

Read first: `cips/accepted/0107-alpha-surface-prune.md` (decision incl.
the resolved formatter-legacy choice) and `.dev/audit-2026-08-05.md`
§P1 alpha prune. NOTE: parts already landed elsewhere — the commented
consent copy and disabled codegen tests died in cip103-leg1, the
superseded runtime `ps` died in cip105. Your scope is the REMAINDER:

1. Zero-exhibit legacy lock fields/read branches (three legacy
   `MemoEntry` fields, `FetchPin::store_path`) and the `--no-cache`
   alias — delete with the evidence check re-run against current locks
   (44 at audit time; recount).
2. Formatter-only leading `FETCH EXPECT` acceptance — drop (decided).
3. LINK: mechanically migrate the active LINK-using Cixfiles to COPY
   (recount; seven at audit time), then remove LINK acceptance in the
   same track. docs/cixfile.md's deprecation note updates to a removed
   note with the teaching rejection diagnostic kept.
4. Whole-tree FetchPin support: REGENERATE the exhibiting locks first
   (18 pins / nine locks at audit time — recount), verify byte-level
   what changed, and only then delete the support. If regeneration
   hits a wall (volatile fetches), record it and leave that leg
   undone rather than deleting evidence-bearing support.
5. Keep all rejection/teaching diagnostics.

Ledger currency: corpus locks change → affected GAPS/receipts note the
regeneration; browser regen; docs/corpus.md rows only if evidence
class changes.

Discipline: branch `track/cip107-prune`, LOG `crates/cix-cixfile/LOG.md`;
full agent gate tier, capture-as-epilogue, bounded VM parallelism
(--max-jobs 2 --cores 2). Parallel tracks in flight — merge
semantically. Clean branch; do not merge.
