# track/cip94 — buildCixfile milestone 1 (eval-from-lock, byte-identity)

Read AGENTS.md first (gate convention; synchronous receipts), then
cips/accepted/0094-build-cixfile.md — its Decision section binds: pure
tier 1, in this repo under `nix/lib`, the byte-identity acceptance test
against `cix build --cold` is load-bearing (the CIP dies rather than
drift), FHS-consuming builders are excluded LOUDLY. Work in the herdr
worktree on branch `track/cip94`. Keep `crates/cix-build/LOG.md` current
(dated track heading; commit it).

## Scope: milestone 1, deliberately bounded

`cix-lib.buildCixfile` (a flake lib exposed via `?dir=nix/lib` or the
root flake's `lib` output — your call, recorded) that can build, WITHOUT
cix installed, from a checked-in `Cixfile` + `Cixfile.lock`:

1. A builder-less Cixfile (pure assembly: IMPORT/COPY/FILE artifacts).
2. A single-BUILDER Cixfile whose FETCH steps replay as fixed-output
   derivations from the lock's pins and whose RUN steps replay offline
   inside a nix derivation reproducing the cix skeleton (bare root,
   IMPORT union incl. lib-union rules as they exist post-CIP-95 for
   BUILDERS ONLY — artifacts' loader aliases are runtime, out of scope;
   /usr/bin/env alias; dev-env snapshot vars from the lock).

The **acceptance test** (this is the deliverable that matters): for at
least two fixtures — one pure-assembly, one FETCH+RUN (the
docs/migrate.md README sample shape is fine) — `buildCixfile` output
must be **byte-identical** (same store path or same NAR hash) to
`cix build --cold` of the same directory. Wire it as a flake check that
CI runs. If byte-identity cannot be reached, that is a STOP-and-report
outcome with the divergence named — do not ship an approximately-equal
build.

Boundary behavior: a Cixfile whose builder needs the FHS loader surface
(CIP-95 aliases) fails eval with a message naming the CIP-94 boundary;
multi-builder graphs and SERVICE-manifest generation beyond the store
tree may be declared out-of-milestone with a clear eval-time error
(record what you cut in the LOG and the CIP is amended at merge, not
silently).

Skeleton sharing (adversarial turn 3 of the CIP): derive both sides
from one definition where practical — at minimum, add a comment-anchored
cross-reference between the Rust skeleton constant and the nix
reimplementation, and let the byte-identity test be the drift tripwire.

Docs: a short section in docs/nix-build.md (consumer-facing: flake
usage, what is covered, the FHS/milestone boundary).

FENCE: regen staging dirs and wave-2 assembly run concurrently — do not
touch corpus/, docs/corpus*, docs/migrate.md. Your domain: nix/lib (new),
flake.nix wiring, fixtures/tests, docs/nix-build.md, your LOG.

## Gate

Standard agent tier + your new flake check + the focused scenarios your
flake.nix wiring touches. Bounded (`nice`, `--max-jobs 6 --cores 4`).
Synchronous receipts with exact repro commands.
