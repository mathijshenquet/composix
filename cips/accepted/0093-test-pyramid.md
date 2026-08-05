# test-pyramid — a systemd shim for orchestration, reality kept anchored

Status: **CIP-93, adopted 2026-08-04** (drafted 2026-08-02 from
Mathijs's spitball: e2e tier is slow, can we mock more — "wel aligned
met reality houden, thecodelesscode.com/case/103").

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

## 5. Decision

Adopted 2026-08-04 (Mathijs): "een erg goed idee — aannemen totdat er
een fundamentele gap blijkt." Amendments at adoption:

- **Full e2e stays minimal**: the complete VM matrix concentrates at
  the orchestrator/CI gate layer; nothing new joins the inner loop at
  VM cost.
- **Progressive tests are the prized win** (Mathijs): the alignment
  (VM) tier should not re-run wholesale when its contract surface did
  not change. Design goal: change-keyed selection — a scenario re-runs
  when the code/fixture/scenario slice it proves has changed, with the
  full matrix remaining available (and still standing at release-grade
  gates). This is read-set-keying philosophy (CIP-87) applied to the
  test pyramid; the keying mechanism is design work for the
  implementing track, not prescribed here.
- Timing guard from §3.5 stands: the shim lands after the systemd
  property surface stabilizes; the e2e tier is currently underpriced,
  not overpriced.

## Changelog

- 2026-08-02: drafted.
- 2026-08-04: adopted with the minimal-e2e and progressive-tests
  amendments.
- 2026-08-04 (evening): progressive leg 1 landed — a derivation-diff VM
  selector (nix/progressive-vm-check.nix): selection derived from nix's
  own derivation identity, never hand-picked; measured docs-only 0/13,
  corpus-only 0/13, one-crate change 13/13, loudly reported, full
  matrix one flag away. Recorded follow-up: stratify scenario inputs so
  a code change stops invalidating all 13 closures (the cix package is
  in every one); the shim itself still waits per §3.5.
- 2026-08-05 (leg 2 amendment proposal): key the progressive tier by
  **declared scenario contract surfaces**, not the linked binary's store
  identity. `nix/scenario-contracts.json` is an ordered total
  classification of product inputs; each scenario declares the surfaces it
  proves. A changed scenario file keys itself, the shared scenario harness,
  package metadata, and other cross-cutting inputs key all scenarios, and an
  unclassified or newly added product path conservatively keys all scenarios.
  Explicitly non-product paths and product surfaces with no VM contract (for
  example the Cixfile build compiler, which is proved in the Cargo/tour tiers)
  select none and say so. The selector validates the scenario inventory and
  classification of every tracked Rust/scenario input on every run, prints
  each changed path plus every selected and skipped scenario with reasons,
  and retains `--full`. `--selector old` preserves leg 1 for comparison;
  `--target` and `--rebuild` make historical measurements reproducible.

  This is deliberately not presented as a static semantic dependency proof.
  The conservative fallback structurally excludes silent skips for unknown
  files, but a reviewer can still misclassify a known file or omit a real
  scenario-to-surface dependency. That bounded human risk is why contract-map
  edits key all scenarios and why the complete derivation matrix remains the
  orchestrator/CI release gate. Dynamic read-sets were rejected because a VM
  observes runtime filesystem/process reads, not which Rust semantics the
  linked binary exercised. Crate/module derivation splitting was rejected for
  this leg because every scenario still consumes the same final `cix` binary;
  splitting its build graph does not split that runtime identity. Explicit
  contracts are the only candidate that expresses what each scenario proves
  while keeping those limits loud.

  Historical-diff measurement on the same host (14-scenario inventory):

  | Diff | Old selector | Old wall | Contract selector | Contract wall |
  | --- | ---: | ---: | ---: | ---: |
  | docs-only `99b45fb..e436bef` | 0/14 | 24.402s | 0/14 | 13.608s |
  | build-subsystem `aa40ffd..d6023f0` | 14/14 | 634.809s | 0/14 | 11.388s |
  | cross-cutting runtime `aa40ffd..a87caa4` | 14/14 | 631.354s | 14/14 | 622.024s |

  The VM timings are synchronous forced rebuilds after pre-warming the
  historical closures, with at most two guests and no competing matrix on the
  host. The docs-only and zero-selection contract timings are selector-only
  runs. The substantive saving is the intended one: a Cixfile build-compiler
  subsystem change drops from all 14 VMs and 634.809s to no VMs and 11.388s
  (98.2% wall-clock reduction), while a cross-cutting runtime change retains
  all 14. An initial rebuild before the historical outputs existed and one run
  overlapped by another worktree's full matrix were both non-zero and excluded,
  rather than being interpreted as measurements.
