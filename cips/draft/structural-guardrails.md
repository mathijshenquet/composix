# Structural guardrails — maps and shared-state receipts that stay current

Status: **draft, CIP-light** (2026-08-05).

## Problem

The 2,000-line error points agents to crate-root module maps, but only compose's
map exhaustively matches its direct modules; Cixfile has none and build/run omit
major modules. The gate does not check maps. It also counts inline tests as
implementation, making `unit.rs` appear 1,929 lines although only 913 are live.
Of 12 shared-ownership/static declarations, five valid uses lack the mandated
site rationale (scratch handler `Once`, tour port atomic, one index-test `Arc`,
two compose-test `RefCell`s).

## Proposal

- Every crate with multiple direct modules carries an exhaustive root map:
  each `mod` name appears with one ownership sentence. A cheap check compares
  declarations with map names; intentional CLI omissions must be explicit.
- Source-size output reports live-before-test, inline-test, and total physical
  lines. Retain a total-file ceiling, but require production decomposition only
  when the live count crosses the policy threshold; large test modules get a
  test-module extraction diagnostic.
- Add the existing `Arc|Rc|Mutex|RwLock|RefCell|static` inventory command to the
  structural audit/checklist. Every exceptional declaration has a preceding
  comment stating why ownership cannot remain local; review still judges the
  rationale, since that part cannot be linted honestly.

## Effort

**S.** Documentation and shell-check work plus one site audit; no runtime
behavior change.
