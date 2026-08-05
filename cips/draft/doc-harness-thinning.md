# Doc harness thinning — executable docs as maintained tooling

Status: **draft, CIP-light** (2026-08-05).

## Problem

`tour.rs` is 2,739 lines and mixes host cleanup, execution, normalization,
seven chapters, rendering, destructive generation, and drift tests. Normal
tests render the entire tour four times under a process-global mutex. It added
1,144 and removed 383 lines in four of the last ten merges. `corpus.rs` is
1,206 lines, including corpus/ledger loading, about 390 lines of custom syntax
highlighting, templates, generation, and tests; it renders the browser three
times. Both generators delete the tracked output directory before replacement.

## Proposal

- Extract one doc-generation support library/tool with `GeneratedFile`, drift
  comparison, and atomic write-to-sibling-then-rename behavior.
- Put each tour chapter in its own scenario module; keep command/cleanup and
  normalization harnesses separate. Test isolation/normalization through
  injected inputs rather than complete rerenders. One ordinary render supplies
  the drift receipt; an explicit determinism test may render twice.
- Split corpus discovery/ledger parsing, highlighting, and page templates.
- Add shared integration-test helpers for store addition, program discovery,
  command receipts, and waits. Collapse byte-identical system/user golden
  fixtures to one expectation where mode produces no difference.

Generated pages and executable receipts remain the contract; this proposal
changes ownership and failure atomicity, not the literate-doc design.

## Effort

**M.** Mostly test/tool moves, but the tour's host-resource lifecycle requires
careful serial receipts and byte-for-byte generated output checks.
