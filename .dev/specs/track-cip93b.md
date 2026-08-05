# track/cip93b — CIP-93 leg 2: change-keyed scenario selection (design + build)

Read first: `cips/accepted/0093-test-pyramid.md` — especially the
Decision's progressive-tests amendment ("a scenario re-runs when the
code/fixture/scenario slice it proves has changed"; the keying
mechanism is explicitly design work for THIS track) and the leg-1
changelog entry. Then `nix/progressive-vm-check.nix` (leg 1, in daily
use) and how scenarios consume the cix binary.

## The problem leg 1 left open (measured today)

Leg 1 keys on nix derivation identity — correct but coarse: any change
to the cix binary invalidates every scenario derivation, so code
tracks get "all 13/14 scenarios changed" and both the worker tier and
the orchestrator gate pay the full matrix (~15 QEMU boots) for a
one-subsystem change. Docs-only changes select 0 — that half works.

## Task

1. **Design the finer key** (this is the core; you have design
   freedom, record the design in the CIP as an amendment proposal —
   do not silently invent language/product surface). Candidate
   directions to evaluate, not prescriptions: per-scenario read-set
   over the cix source tree (which modules does the code path a
   scenario proves actually touch — CIP-87 read-set philosophy applied
   to tests); scenario-declared contract surfaces (each scenario names
   the subsystems it proves, checked against the diff); crate/module-
   granular derivation splitting so nix identity itself becomes finer.
   Weigh honesty hard: a false-negative selection (scenario skipped
   that would have failed) is the failure mode that must be
   structurally excluded or loudly bounded — say precisely what the
   selection can and cannot miss.
2. **Build it** behind the existing `progressive-vm-check` entry point
   (same UX: prints selected AND skipped with reasons, `--full`
   escape). The full matrix stays available and stays the
   orchestrator/CI release-grade gate — this track speeds the inner
   loop and the per-track tier, it does not weaken the merge layer.
3. **Measure and report**: for three representative historical diffs
   (docs-only, one-subsystem code change, cross-cutting change) show
   old-selector vs new-selector counts and wall-clock. The number is
   the deliverable Mathijs sees.

## Discipline

- Branch `track/cip93b`, this worktree. Log: `crates/cix/LOG.md`.
- Gates: full agent tier (fmt / examples fmt / warning-denied clippy /
  full workspace tests / tour drift) + progressive-vm-check both OLD
  and NEW selectors on your own diff; `.gate-exit` capture pattern for
  long runs; synchronous receipts in the LOG.
- The CIP amendment (design + measurements) goes in
  `cips/accepted/0093-test-pyramid.md`'s changelog with a dated entry.
- Parallel tracks may be in flight — resolve merges semantically.
- Commit granularly; leave the branch clean. Do not merge to main.
