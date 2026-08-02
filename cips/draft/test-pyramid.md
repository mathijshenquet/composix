# test-pyramid — a systemd shim for orchestration, reality kept anchored

Status: **draft** (2026-08-02, from Mathijs's spitball: e2e tier is
slow, can we mock more — "wel aligned met reality houden,
thecodelesscode.com/case/103").

## 1. The problem

The VM scenario tier (~15 QEMU boots) conflates two test subjects:

1. **Our orchestration state machine** — `cix up/down/rollback`,
   restart-changed sequencing, teardown ordering, lock flows. These
   pay a VM boot to test OUR code's call sequences.
2. **Our model of systemd/kernel reality** — DevicePolicy, netns,
   watchdog, credentials, mount stacking. Irreplaceable by
   construction: mocking systemd here means testing our beliefs about
   systemd against our beliefs about systemd (case 103, verbatim).

vmslim (-53%) and the gate re-layering already cut cost and
frequency; what remains is the structural split.

## 2. Prior work

The unit-text fixtures are the house's existing "mock done right":
assert the generated artifact, never the runtime — fast, and they
carried a large share of today's catches. Docker's ecosystem went the
other way (dockerized test everything) and pays in minutes-long
suites; k8s upstream splits envtest (API-server-only double) from
real-cluster e2e with exactly the contract discipline proposed here.
Case 103 is the canonical failure: a fully-mocked suite proving only
that the mocks agree with themselves.

## 3. Recommendation

1. **A systemd shim for category 1 only**: a systemctl/D-Bus-shaped
   test double that records unit operations and plays back configured
   states, so orchestration regressions test in milliseconds in the
   cargo tier.
2. **The case-103 guard, as a hard rule**: every behavior the shim
   asserts must have a VM-scenario counterpart proving the same
   contract against real systemd. The shim never claims behavior no
   VM scenario has demonstrated. Where practical, shim and scenario
   share assertion definitions so drift is structural, not social.
   A shim assertion without a named VM counterpart fails review.
3. **Reality stays the merge gate**: the VM tier keeps running in the
   orchestrator's full matrix unchanged. The shim ADDS a fast inner
   loop; it removes nothing from the outer one.
4. **Scenario consolidation** (independent, mock-free win): group
   compatible scenario scripts into shared VM boots to amortize boot
   cost, trading some failure isolation — measured before/after like
   vmslim, adopted only where the numbers justify.
5. **Timing**: adopt the shim AFTER the systemd property surface
   stabilizes (today it churns weekly and the e2e tier caught seven
   real defects in one day — it is currently underpriced, not
   overpriced). This draft exists so the split is designed before the
   pressure arrives.

## 4. Open questions

1. Shim seam: fake `systemctl` binary on PATH vs a D-Bus double vs a
   trait boundary in cix-run's systemd calls (the trait is the most
   Rust-native and least stringly; it also serves CIP-90's
   injection philosophy).
2. Does the shim live in-tree as a test-support crate?
3. Consolidation grouping: by fixture weight, by subsystem, or by
   historical flake-independence?
