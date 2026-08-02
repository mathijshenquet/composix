# Test hygiene — the two failure classes today's incidents proved

Status: **CIP-90, adopted 2026-08-02** (Mathijs). Decision in §5.

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

1. **Config at the absolute boundary** (Mathijs's design, clap-shaped;
   supersedes the earlier inject-or-mutex option — global state is
   avoided at all costs, so mutex-guarding the symptom is out):
   every `CIX_*` environment variable is resolved exactly ONCE, at
   the CLI boundary, into a config struct with clap's precedence
   (explicit arg > env var > default — clap's `env` feature gives
   this for free on existing flags). Library and runtime code take
   values from the struct and NEVER read the environment;
   `std::env::var("CIX_…")` outside the boundary module becomes a
   denied pattern (CI grep or custom lint). Tests construct the
   config directly — `set_var` disappears from the test suite
   entirely rather than being guarded. Sweep the existing sites
   (CIX_STATE_DIR, CIX_BUILD_WORKSPACE_DIR, CIX_PRIVATE_DEVICES_PROBE,
   the index test vars, watch.rs) into fields.
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

## 5. Decision

Adopted as recommended, with Mathijs's sharpening recorded: use clap
LITERALLY — the boundary config is clap derive structs carrying
`env = "CIX_…"` attributes (arg > env > default for free); good
libraries are preferred over hand-rolling, explicitly also as a DX
stance. `cix ps --json` is the public machine surface the tour
consumes (docker `--format json` precedent). Env reads outside the
clap boundary become a denied pattern in the gate.

## Changelog

- 2026-08-02: drafted from the step-2 sweep; reshaped same day to
  boundary-config on Mathijs's review; adopted same day.
- 2026-08-02 (same day, shared-state amendment): inventory ran clean —
  zero Arc/Rc/Mutex/RwLock in production src; four atomics (two
  signal-handler INTERRUPTED flags, duplicated across watch.rs and
  runtime.rs — unify in cix-common; the --stats subprocess counter;
  a borderline NONCE_COUNTER — make it an injected generator); three
  test RefCells (idiomatic mocks, fine); two test Mutexes
  (TOUR_RENDER_LOCK, WORKSPACE_DIRECTORY) which this CIP's
  boundary-config work must make REDUNDANT and delete, not keep.
  Standing review rule recorded in AGENTS.md: shared ownership /
  interior mutability requires a stated justification at the site.
- 2026-08-02: leg A implemented clap-derived boundary configuration, env-free tests, the unified interrupt flag, and the boundary lint.
