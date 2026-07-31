# track/underlay — D71: builder underlay semantics

Read AGENTS.md first. Authoritative: design.md **D71** (and D69(e) for the
--cold contract). SEQUENCED AFTER track/pinkeys merges (same
build_chain/keying area — build on its landed state, including consumed-set
keying, offline --cold, and the codegen fingerprint).

1. Re-running a changed step no longer resets to the prefix snapshot: the
   builder's persistent workspace carries its last end-state as the underlay;
   the replay executes the changed step (and successors) directly on it.
   Same-builder-only (same project workspace + builder name); no
   cross-builder/cross-project reuse. Consumed-output recording continues to
   pin exactly what the dock consumes.
2. `--cold` runs WITHOUT the underlay (empty workspace, offline per D69e) —
   verify the cold_audit test still measures exactly this and stays green
   across examples.
3. Tests: (a) a warm re-run after a source edit reuses step-local state
   (prove via a fixture whose RUN appends to a workspace file — warm sees the
   previous value, cold does not); (b) the ghost-file hazard is documented in
   docs/cixfile.md's workshop section in D71's words (path-dependence, rm-rf
   safe, --cold is the clean truth); (c) determinism of the DOCK is untouched
   (tour determinism-twice stays green).
4. Living receipts (D71e): update docs/migrate.md's `RUN --mount=type=cache`
   row to the now-fully-true claim; re-run the nix-build.md warm-edit
   measurement for the cix route with the same harness
   (examples/compare/gitsitter/measure-warm.sh) and update ONLY that row
   with a new dated receipt (leave the other routes' numbers with their
   original dates).
5. Gate: fmt / warning-denied clippy / workspace tests / cold_audit sweep /
   tour regen + drift + determinism twice / vm-dogfood / the re-measurement.
   Exact repros in crates/cix-cixfile/LOG.md. Commit on this branch when
   green.
