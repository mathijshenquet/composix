# track/start — CIP-80: EXEC→START, SETUP→START_PRE

Read AGENTS.md first. Authoritative: docs/cips/0080-exec-naming.md §5
(Decision). Same recipe as track/claim (which merged as `d2d5033` — use
it as the template for sweep hygiene). SEQUENCED AFTER track/timers and
track/watch merge — do not start before their merges land on main (all
three touch examples/docs; this sweep goes last). Work in
`.worktrees/start` on branch `track/start`. Keep
`crates/cix-cixfile/LOG.md` current.

1. Cixfile directives: `EXEC` → `START`, `SETUP` → `START_PRE`. Old
   spellings get the standard migration suggestion (crunchy forgiveness
   boundary: suggest, never silently accept). Torture fixtures for
   `EXEC`, `SETUP`, and a fuzzy near-miss (`STRAT`).
2. Manifest fields: `exec` → `start`, `setup` → `start_pre` (v0 schema,
   D72). Unit generation unchanged semantically (`ExecStart=`,
   `ExecStartPre=`). Bump the codegen fingerprint (d78-v1 → d80-v1).
3. Sweep every consumer: parser, spec/manifest structs, unit.rs,
   compose generation, examples, corpus/migrate Cixfiles + receipts,
   docs (cixfile.md, docker.md translate rows — ENTRYPOINT+CMD → one
   `START` line —, migrate.md, tour regen), vm fixtures, fmt module
   keyword tables if any. End with `rg -iw 'exec|setup'` triage in the
   LOG: remaining hits must be justified (ExecStart= property names,
   design-history, CIP texts).
4. Gate: fmt / `cix fmt --check examples` / warning-denied clippy /
   workspace tests / tour regen + drift / full
   `devenv shell -- nix flake check -L`. Exact repros in the LOG.
   Commit on this branch when green.
