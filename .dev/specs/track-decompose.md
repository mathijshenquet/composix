# track/decompose — D73 + D72: the thinning round

Read AGENTS.md first. Authoritative: design.md **D73** (with addendum) and
**D72**. This is a MECHANICAL round: move and delete, do not redesign.
Behavior-preserving except where D72 prescribes deletion. The crunchy torture
snapshot suite is the diagnostics contract: it must pass UNCHANGED (byte-equal
snapshots) after every step.

1. **Crate split (D73a)**: new crate `cix-build` receives the workshop engine
   from cix-cixfile: build_chain.rs (chain, keying, workspaces, sandbox,
   FETCH pins/probe) and the build-side halves of lock handling. cix-cixfile
   keeps parser + codegen + the language surface; it depends on cix-build.
   Public API between them: smallest possible, documented in the module head.
2. **Parser diet (D73d, per the analysis report)**: split parser.rs into
   parser/{machine,directives,migrations,validate}.rs (+ existing
   diagnostics.rs) with a thin mod; move the 571 in-file test lines to
   tests/; replace hand-written migration arms with a declarative table
   (directive → internal D-number field → message text; messages keep their
   exact current wording — snapshot-guarded); consolidate near-duplicate
   validators. No message text changes.
3. **Index modules (D73b)**: cix-index/lib.rs splits into refs/tags/roots/
   serve/pull modules; lib.rs = thin re-exports.
4. **spec v0 (D72)**: cix-run/spec.rs collapses to the single version-0
   schema: `cixManifest: 0` is the only accepted value; all v1–v5 validation
   paths die; any other value = the friendly "rebuild with the current cix"
   error. Codegen emits 0. Sweep EVERY in-repo manifest: examples locks/
   fixtures regenerate via build, hand-written manifests (nix/vm-dogfood.nix
   and any scenario fixtures) edit to version 0 with field shapes matching
   the current schema. Tour regenerates.
5. Re-measure with tokei (crates only) and record the before/after per-file
   table in the LOG — the D73 baseline expects a visible drop.
6. Gate: fmt / warning-denied clippy / `cargo test --workspace` (torture
   snapshots byte-identical) / tour regen + drift + determinism twice / the
   full `devenv shell -- nix flake check -L` (repo convention). Exact repros
   in crates/cix-cixfile/LOG.md. Commit on this branch when green.
