# CIP-103: build-chain seams — complete CIP-89 without a shared-context split

Status: **accepted** (CIP-light) (2026-08-05; implementation amendment to
CIP-89).

## Problem

`cix-build/src/build_chain.rs` is 4,369 physical lines. Excluding a
254-line commented copy of the extracted FETCH-consent implementation and
773 lines of inline tests still leaves about 3,342 live lines. CIP-89 already
decided to split memo/replay and workspace ownership, but its first leg stopped
correctly when both strata depended on conductor-local trace, sandbox, and
filesystem helpers. Since then four of the last ten track merges have edited
the exception.

## Proposal

Adopt interfaces, not a file shuffle:

1. Delete extraction residue and move the conductor tests beside their owner.
2. Introduce `Workspace`, owning persisted state, staging, snapshots, tree
   reconciliation, node hashing/fingerprints, and store materialization.
3. Introduce `MemoEngine`, owning key construction, validation, read/write-set
   reduction, cold comparison, and constructive replay. It may operate through
   `Workspace`; it must not receive a bag-of-fields shared context.
4. Introduce context/sandbox request-result boundaries around Nix evaluation,
   offer realization, import union, FHS/network fallback, and tracing.
5. Move FETCH snapshot volatility and pin refresh behind a FETCH-state owner.
   Keep `build_chain` responsible only for ordered FETCH/BUILDER dispatch and
   collecting typed step results.

Each leg is a pure move with byte-identical lock/output receipts, and only runs
while the crate is quiet, preserving CIP-89's existing conditions.

## Effort

**L.** Residue/tests are S; workspace, context/sandbox, and FETCH state are M
each; memo/replay is L because its acceptance condition is a genuinely narrow
owned interface rather than fewer lines.

## Decision

Adopted 2026-08-05 (orchestrator, under Mathijs's delegated structural
review; KPIs: complexity down, seams clean, SLOC under control). Claims
verified against the code (comment block = lines 162–411; the four
proposed owners exist as contiguous function clusters). Two amendments:

1. **Residual-conductor target**: after all legs, `build_chain`'s live
   (pre-test) count is ≤1,500 lines INCLUDING a decomposed
   `execute_builder` (~600 lines today) emitting typed step results —
   measured in the closing receipt, not asserted.
2. **Scheduling**: leg 1 (residue deletion + test moves, S) runs
   immediately as its own track; each M/L leg is a dedicated track
   slotted between feature waves. Byte-identical lock/output receipts
   per leg, per CIP-89's standing conditions.

Changelog:
- 2026-08-05 — adopted as CIP-103 with the two amendments.
