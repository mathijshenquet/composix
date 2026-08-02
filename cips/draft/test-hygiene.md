# Test hygiene — the two failure classes today's incidents proved

Status: **draft** (2026-08-02, drive-progress step-2 sweep). Both
items are generalizations of failures that actually bit today —
evidence-backed, not speculative.

## 1. The problem

**Class A — process-global env in tests.** The dirs2/ergo
"workspace-key instability" cost two fix rounds before terra found it
was two tests racing on the process-global `CIX_BUILD_WORKSPACE_DIR`.
The same class exists in at least six more files
(`rg -l "set_var|remove_var" crates`): cix-index/tests/{hammer,pull},
cix-cixfile/tests/{proj1,fmt,lock_nix}, cix-cixfile/src/watch.rs.
Every one is a latent scheduling-dependent flake that will eventually
burn a gate round — and scheduling-dependent greens are exactly how
false receipts happen honestly.

**Class B — the tour's screen-scraping filters.** Chapter 3's
`cix ps` filter broke TWICE today (devfix: stale five-column parse hid
the active row; ergo fix: the RESULT column shift broke it again).
The tour harness re-parses human-facing table text; every column
change is a silent tour breakage that surfaces as CI drift an hour
later.

## 2. Prior work (house)

The vmslim fix for the timer-gc race (`wait_until_succeeds`) and
today's orchestrator fix for the store-prefix test (inject the search
list instead of mutating PATH) are the pattern: tests take injected
inputs; they never mutate process state another test can observe.

## 3. Recommendation

One hygiene track:

1. **Env-global sweep**: for each set_var/remove_var site, either
   inject the value (parameterize like `find_path_program_in`) or —
   where the code under test genuinely reads the env — serialize via
   a shared mutex AND restore under a drop-guard. Add a clippy-style
   deny (a small custom lint or a CI grep) refusing new
   `std::env::set_var` in test code without the guard.
2. **Tour structural reads**: the tour keeps rendering human tables
   (that IS the documentation), but the harness's assertions/filters
   consume `cix ps --json`-shaped output (the CIP-83 machinery
   already projects this data) and render the human table from the
   same source — one truth, no column-parsing. The two regressions
   this dissolves are the receipts.

## 4. Open questions

1. Does `cix ps` grow a `--json` flag (useful to operators too, and
   trivially — the data is already structured internally), or does
   the tour harness get a private hook? Draft leans public `--json`:
   docker precedent (`--format json`), zero extra surface beyond
   serialization.
