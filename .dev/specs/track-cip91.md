# track/cip91 — artifact-import implementation (universal IMPORT, store-aware COPY)

Read AGENTS.md first (gate convention; synchronous receipts), then
cips/accepted/0091-artifact-import.md — it is the contract; its Decision
section binds. Work in the herdr worktree on branch `track/cip91`. Keep
`crates/cix-cixfile/LOG.md` current (dated track heading).

## Phase 1 — the spike (report before implementing phase 2)

Prototype store-aware COPY per CIP-91 §3(b): link-by-rule for immutable
store sources, materialize when (i) a role-dir/`DIR` mount point lies
beneath the destination or (ii) later assembly writes beneath it (choose
materialize-vs-refuse for (ii) and defend the choice). Validate three
shapes:

1. **tomcat**: whole-package COPY with a later write under the tree.
2. **directus**: `COPY ${build}/dist /app` plus role dirs beneath the
   destination — the ancestor chain must materialize automatically.
3. **realpath probe**: a node (or equivalent realpath-walking) script
   resolved through a linked tree — record the observable difference vs a
   materialized tree so the risk CIP-91 accepts is documented evidence,
   not folklore.

Write the spike verdict in your LOG. If it sours (rules stop being
statically decidable, or the realpath hazard is worse than accepted),
STOP after the spike, implement only the fallback scope (universal IMPORT
+ canon docs; LINK stays first-class), and say so plainly.

## Phase 2 — implementation (spike clean)

- **Universal IMPORT**: SERVICE/APP/ITEM accept `IMPORT` with builder
  union semantics (`bin`/`etc`/`share`, earlier wins); bare START/
  START_PRE argv checking extends over the imported set. ITEM stays
  manifest-free.
- **Store-aware COPY** per the spike; deterministic, mode derivable from
  the Cixfile text.
- **LINK becomes a deprecated alias**: parses, behaves as the equivalent
  COPY, emits a diagnostic hinting COPY. Do not delete it — the corpus
  still uses it until the regeneration sweep; the alias dies in a
  fast-follow after that sweep (alpha rule).
- **Docs**: rewrite docs/migrate.md's runtime-toolset teaching around
  IMPORT + bare names (no interpolated argv in examples; role dirs at
  app-native paths per §3(d)). Re-grade affected docs/docker.md rows.
  Update the tour if it demonstrates LINK.
- **Ledger currency** (AGENTS.md extension): grep
  `corpus/migrate/*/GAPS.md` for artifact-import/CIP-91 and flip
  exhibiting cases to `Status: stale — regenerate with CIP-91`. Do NOT
  edit corpus Cixfiles — regeneration is cold, later, per the loops.

Out of scope: the interpolated-argv lint (lands with the regeneration
sweep), corpus regeneration, CIP-92 (queued behind this track — do not
touch PORT parsing).

FENCE: track/netnsrace (netns wiring + scenarios) and track/adapterlive
(health wiring + scenarios) run concurrently — do not touch their
modules. Your domain: crates/cix-cixfile (parser/model), crates/cix-build
(assembly/codegen/lock as needed), docs/migrate.md, docs/docker.md rows,
GAPS status lines, tour, your LOG.

## Gate

Standard agent tier (fmt, examples fmt, warning-denied clippy, full
workspace tests, tour regen+drift) plus the FOCUSED VM scenarios your
assembly changes touch (at minimum one closed-root scenario consuming a
linked artifact). Bound everything (`nice`, `--max-jobs 6 --cores 4`) —
two other tracks run VM work concurrently. Receipts are synchronous exit
statuses with exact repro commands.
