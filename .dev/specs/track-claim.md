# track/claim — CIP-78: the GRANT→CLAIM rename sweep

Read AGENTS.md first. Authoritative: docs/cips/0078-devices.md §5
(Decision). Scope: the RENAME ONLY — `CLAIM gpu`/`CLAIM device`/`SHM`
implementation is a later track. Work in `.worktrees/claim` on branch
`track/claim`. Keep `crates/cix-cixfile/LOG.md` current.

1. Cixfile directive: `GRANT egress` / `GRANT jit` become
   `CLAIM egress` / `CLAIM jit`. The old spelling `GRANT …` gets a
   migration error with a did-you-mean-style suggestion pointing at
   `CLAIM` (the crunchy forgiveness boundary: suggest, never silently
   accept). Torture/suggestion fixture added for `GRANT egress` and for
   the fuzzy near-miss `CLAM egress`.
2. Manifest field: `grants` renames to `claims` (v0 schema moves freely,
   D72). The name `grants` is RESERVED for the future compose-side
   loosening field (CIP-78) — leave a comment at the manifest field
   stating that reservation.
3. Sweep every consumer: parser directives + validation, spec/manifest
   structs, unit generation, examples (`examples/**` Cixfiles use
   `GRANT egress`), docs (docs/cixfile.md, docs/docker.md rows,
   docs/migrate.md translate table, docs/tour — regen), fixture
   corpora. `rg -i 'grant'` at the end must show only: the CIP texts,
   design.md history, and the reserved-name comment — list any other
   survivors in the LOG with justification.
4. Tests: existing suites green after rename; the new suggestion
   fixtures; tour regen + drift.

Gate: fmt / `cix fmt --check examples` / warning-denied clippy /
workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Exact repros in the LOG. Commit on
this branch when green.
