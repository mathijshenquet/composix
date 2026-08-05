# CIP-104: crate strata — make D67/D73 executable in the dependency graph

Status: **accepted** (CIP-light) (2026-08-05).

## Problem

D67 makes manifest/runner, compose, Cixfile workshop, and index separate
strata; D73 says parser/codegen are language while chain/key/workspace/sandbox
are the build engine. The code disagrees: `cix-build` owns the 1,515-line
codegen and a second copy of ten manifest concepts; its direct codegen tests
are disabled because importing the parser would create a cycle. `cix-run` and
`cix-build` open the index internally, while `cix-cixfile::watch` activates
compose. One contract field recently fanned out across 11 Rust files.

## Proposal

- Give the alpha manifest schema and canonical serialization one neutral
  stratum-1a home consumed by runner, compose, and SERVICE/APP codegen. Nix and
  literal JSON emission must project from the same typed contract.
- Make `cix-cixfile` the parser/formatter/language adapter. Move build/watch CLI
  coordination to `cix` or a top-level application crate so the language crate
  no longer depends on compose.
- Keep chain/key/workspace/sandbox in `cix-build`; pass it language plans rather
  than having it own parser/codegen public surface.
- Inject artifact/name resolvers into run/build APIs. Concrete `cix-index`
  access stays in compose or the top-level command boundary.
- Split parser directive implementations/tests into `inputs`, `builder`,
  `assembly`, and `contract` modules, with one parser state and one validation
  layer.

Acceptance: acyclic graph; manifest/codegen cross-checks run normally; the
runner library can compile without index; parsing/formatting can compile
without compose; generated manifests and CLI behavior remain byte-identical.

## Effort

**L.** This changes crate/API ownership and Cargo manifests, though most source
moves are mechanical. Stage it after local build/run thinning, with contract
fixtures as the compatibility receipt.

## Decision

Adopted 2026-08-05 (orchestrator, delegated structural review). This
executes D67/D73 — no new strata are invented; the neutral manifest
home is stratum 1a as D67 already names it. Verified: codegen.rs is
1,515 lines inside cix-build; cix-cixfile depends on cix-compose in
Cargo.toml (the cycle claim holds). Staged after CIP-105 and CIP-103's
workspace/memo legs, per the draft's own ordering. Acceptance as
written: acyclic graph, byte-identical manifests and CLI behavior.

Changelog:
- 2026-08-05 — adopted as CIP-104.
