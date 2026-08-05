# CIP-105: runtime thinning — preserve a real run conductor

Status: **accepted** (CIP-light) (2026-08-05).

## Problem

`cix-run/src/runtime.rs` is 1,969/2,000 lines (1,841 before tests) and owns
directory overrides, finite/scheduled apps, degradation policy, persistent
listener/unit/GC-root lifecycle, transient journal control, index/Nix target
resolution, and an obsolete human `ps`. The real binary intercepts `Ps` and
uses `cix-compose::ps(json)`, so 114 lines are already superseded. `unit.rs` is
1,929 lines only because 1,016 lines are inline golden tests.

## Proposal

- Confirm no supported library caller needs `cix_run::runtime::ps`, then delete
  it and keep one observability implementation.
- Extract `target` (path/ref/Nix resolution), `app` (finite/scheduled units),
  and `manager` (persistent units, listeners, GC roots, systemctl/journal).
  `runtime` retains option validation, service selection, degradation ordering,
  and calls into typed request/result APIs.
- Move the `unit` golden/test module to a submodule without changing fixture
  ownership or property ordering.
- Update the run crate's module map exhaustively.

Acceptance: unit text, degradation order, app exit propagation, listener
lifecycle, and CLI output remain identical; no source file needs an exception.

## Effort

**M.** The obsolete path/test move is S; the three pure module moves and API
shaping are M.

## Decision

Adopted 2026-08-05 (orchestrator, delegated structural review) as
written, one sharpening: the `ps` deletion must carry PROOF of no
supported caller (grep receipt + compile of all dependents), not the
draft's "confirm". Byte-identical acceptance stands. The superseded-ps
deletion satisfies CIP-107 item 1's overlapping bullet; CIP-107's
track skips it when already landed.

Changelog:
- 2026-08-05 — adopted as CIP-105.
