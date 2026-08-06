# track/tourdet-teardown — fix the producer-unit stop race in tour determinism

CI on main (`f3115542`, workflow job `test`, 2026-08-06T08:33Z) failed
`generated_tour_is_deterministic` with:

    assertion `left == right` failed: `systemctl --user stop "$unit"` produced:
    Failed to stop cix-run-producer-v1-18c92a….service: Unit … not loaded.
    left: false / right: true

The identical tree plus one LOG line (`dab9fb4f`) passed the same suite
minutes later: this is a teardown RACE, not a content regression. The
transient producer unit finishes and self-unloads between the scenario's
completion and the explicit `systemctl --user stop`, so the stop's
success assertion is timing-dependent.

Do:
1. Locate the teardown in the tour harness (crates/cix/tests/tour*,
   producer/consumer scenario) and characterize the race precisely —
   confirm the unit is transient and self-unloading (RemainAfterExit
   semantics) rather than papering over an unknown.
2. Fix so teardown is idempotent: "unit not loaded" at stop time is a
   SUCCESS outcome (already stopped), not an assertion failure. Keep a
   genuine stop failure (unit loaded but stop errors) a hard failure —
   do not blanket-ignore systemctl errors.
3. Check the same pattern elsewhere in the harness (other scenarios
   stopping transient units) and fix symmetrically where the same race
   exists.
4. If a cheap deterministic regression test is feasible (e.g. stopping
   an already-exited unit in a harness unit test), add it; if not,
   record why in the LOG and rely on the assertion-shape change.

Gates (this touches test infra, Rust): fmt, warning-denied clippy,
`cargo test --workspace` (tour tier included), tour regen + drift. No
product/runtime semantic changes — harness only; if the fix seems to
need product changes, STOP and record that instead.

Discipline: branch `track/tourdet-teardown`, LOG `crates/cix/LOG.md`
(append, timestamped, FRICTION section). Synchronous value-checked
receipts; exact repro commands in the LOG. Clean committed branch; do
not merge.
