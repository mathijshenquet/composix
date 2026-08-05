# track/audit1 — code structure & complexity audit (sol; findings, no fixes)

Read first: AGENTS.md (esp. the shared-ownership justification rule and
module-map convention), docs/design.md "Building now", the crate module
maps, and `nix/` for the source-size check (it prints the current
limits and the one GRANDFATHERED file).

## Task — measure, judge, report; do NOT fix

An honest structural inventory of the workspace, in the house
discipline (measure first; decompose along strata; thin hotspots
proactively; alpha deletes speculative compat):

1. **Measure**: LOC per file/module across all crates (top 20 list);
   files near the 2000-LOC limit; the grandfathered
   `cix-build/src/build_chain.rs` (4369 LOC) — what strata still live
   in the conductor and what would each extraction cost; module maps
   vs reality (drifted? missing entries?); dependency direction
   between crates (any upward/cyclic leaks?).
2. **Judge**: strata boundaries that blur (build vs run vs cixfile vs
   compose vs index); hotspots that keep absorbing features (which
   files did the last ~10 tracks all touch?); every
   Arc/Rc/Mutex/RwLock/RefCell/static site — is the required
   justification comment present and does it hold?; speculative
   compat or dead surface that alpha should delete; test-code
   structure (fixture duplication, harness sprawl, the tour/corpus
   generators).
3. **Report**: a prioritized findings list — for each: evidence
   (numbers, paths), why it will hurt (concretely, not aesthetically),
   proposed decomposition, estimated effort (S/M/L). Actionable items
   land as `cips/draft/<name>.md` entries (CIP-light where they fit
   one screen) — drafts are the ONLY output besides the report; do
   not start any restructuring. Write the report itself as
   `.dev/audit-2026-08-05.md` (committed on this branch).

## Discipline

- Branch `track/audit1`, this worktree. Log: append to
  `.dev/audit-2026-08-05.md` as you go (it doubles as the log).
- Read-only toward src/: your diff should contain ONLY the report and
  drafts. Gates: fmt-check of your own md files' formatting is
  irrelevant — run `git diff --check` and nothing else; no VM/test
  runs needed for a read-only audit.
- Parallel tracks are in flight; base your reading on this worktree's
  checkout and note its commit hash in the report header.
- Commit the report + drafts; leave the branch clean. Do not merge.
