# track/cip93 — progressive test selection (CIP-93's prized amendment)

Read AGENTS.md first, then cips/accepted/0093-test-pyramid.md — the
Decision section's progressive-tests amendment is the mandate: the
alignment (VM) tier should not re-run wholesale when its contract
surface did not change; a scenario re-runs when the slice it proves
changed. Work in the herdr worktree on branch `track/cip93`. Keep the
owning LOG current (likely a new `nix/LOG.md` or the crate whose test
plumbing you touch; commit it).

## Phase 1 — measure the selection signal (report before building)

Nix already content-addresses every scenario derivation: an unchanged
scenario on an unchanged closure is a cache hit. Measure WHY that is
not already progressive in practice: for two consecutive
orchestrator-gate runs on trees differing by (a) a docs-only commit,
(b) a corpus-only commit, (c) a one-crate code commit, record which
scenario derivations rebuilt and why (the cix package in every scenario
closure is the obvious suspect — any Rust edit rebuilds cix, which
invalidates every VM). Report the dependency map honestly.

## Phase 2 — design + implement per the evidence

Candidate shapes to weigh (pick with the phase-1 numbers, record the
choice): (a) scenario stratification — scenarios that only consume
generated units/fixtures depend on those fixtures instead of the full
cix binary where truthful; (b) a two-tier gate driver — `nix flake
check` scoped to the scenarios whose input slice changed (computed from
nix's own dry-run/eval diff, never a hand-maintained list — agents must
not hand-pick what counts as green, so the selection must be DERIVED,
loud, and overridable by the full matrix); (c) honest conclusion that
nix's caching is already the mechanism and the win is restructuring
scenario inputs. The full matrix remains available and remains the
release-grade gate; progressive selection is the inner loop.

Whatever lands: the orchestrator gate command in AGENTS.md gets its
amendment in the same track, the selection is loud about what it
skipped and why (no silent green), and a `--full` escape stays one flag
away.

FENCE: coordinate with in-flight corpus/tour tracks by domain — your
surface is nix/scenarios plumbing, flake.nix check wiring, gate docs.
No corpus content, no tour prose, no cix runtime code.

## Gate

Standard agent tier + demonstrate the selection on the three phase-1
tree shapes with synchronous receipts (what ran, what was skipped,
why). df-guard; bounded.
