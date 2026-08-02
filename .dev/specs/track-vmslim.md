# track/vmslim — cut VM-scenario wall-clock without weakening a single assertion

Read AGENTS.md first — NOTE the amended gate convention: your gate is
fmt/clippy/workspace tests/tour + the FOCUSED scenarios you touch,
plus in this track's case a full timed scenario sweep because the
sweep IS the deliverable's measurement. Work in
`/home/mathijs/worktrees/composix/track-vmslim` (herdr worktree) on
branch `track/vmslim`. Keep `crates/cix-run/LOG.md`... no — this track
is nix-side: keep a LOG at `nix/LOG.md` (it exists) current.
FENCE: track/closedroot runs concurrently and ADDS
`nix/scenarios/closedroot-audit.nix` + touches crates/cix-run — do not
touch cix-run; your changes live in nix/scenarios/lib.nix and shared
VM plumbing, keep them composable with a new scenario arriving.

Goal (Mathijs KPIs: speed and correctness): reduce the wall-clock of
the VM scenario tier. Assertions may not change; scenario semantics
may not change; only the machinery gets cheaper.

1. **Measure first, synchronously**: time each scenario check
   (`/usr/bin/time nix build .#checks...scenario-X --no-link -L` after
   a warm cix build) and record a baseline table in the LOG.
2. **Slim the shared VM config** (nix/scenarios/lib.nix + whatever it
   imports): candidates — `documentation.enable = false` (man/db
   generation is a notorious VM-test cost), minimal module profile,
   disable unneeded default services, `system.switch.enable = false`
   if compatible, boot/kernel noise reduction, memory/cores right-
   sizing, virtio everywhere. Evaluate each knob against the baseline;
   keep what measurably helps, revert what doesn't, record numbers.
3. **Eval/build overhead**: check whether scenarios share their base
   VM system derivation or each rebuild one; factor the common machine
   config so nix builds the base system ONCE across scenarios where
   semantics allow.
4. **After table**: same measurement re-run; the LOG ends with
   before/after per scenario and a total. Target: meaningful total
   reduction (30%+ if the documentation/module knobs bite as they
   usually do); honesty over target — report what is real.
5. All scenarios must stay green, byte-level assertion diffs zero
   (`git diff` on scenario test scripts should show machinery-only
   changes).

Gate (amended convention): fmt / examples fmt / clippy / workspace
tests / tour regen + drift / your full timed scenario sweep (which
doubles as the after-measurement). Commit on this branch when green.
