# Track: cixfile — implement Cixfile v1 (`cix build`)

Contract, in order of authority: `docs/design.md` "Part 4 — Cixfile v1" (+ D21, D22) and
`docs/cixfile.md` (the directive reference and semantics, including the worked nginx
example). Where ambiguous, choose boring and note it in LOG. Do NOT expand scope: assembly
subset only, no RUN, no ecosystem builds.

## Ground rules

- Work log: create/append `crates/cix-cixfile/LOG.md`. Territory: `crates/cix-cixfile/`
  (new crate — note the name, `cix-cixfile`, not `cix-file`), `crates/cix/src/main.rs` +
  `crates/cix/Cargo.toml` (wiring only), workspace `Cargo.toml`, `examples/*/Cixfile`,
  `examples/*/Cixfile.lock`.
- Sudo available for end-to-end verification; clean up all units.
- COMMIT AS YOU GO; done gate includes clean `git status --short`.

## Deliverables

1. **Parser** for the v1 directive set exactly as documented (PKG, COPY, FILE, SCRIPT, LINK,
   SERVICE, EXEC, SETUP, ENV, PORT, STATE/CACHE/LOGS/CONFIG/RUNDIR, JIT). Line-numbered
   errors with the offending line quoted. Interpolation rules enforced: `${…}` only in
   directive arguments (FILE/SCRIPT heredoc bodies are the exception — they interpolate);
   COPY is verbatim; unknown `${name}` (no matching PKG) is an error. `$VAR` passes through
   untouched. `$${…}` escapes to literal `${…}` in heredocs.
2. **Codegen**: Cixfile → a nix expression (runCommand-style assembly mirroring the
   hand-written examples: files, scripts with shebang+exec bit, links, generated
   `cix-spec.json` per cix-spec v2 with de-typed env). Deterministic output (stable ordering).
3. **Lock**: `Cixfile.lock` (JSON: nixpkgs url/rev/narHash). Created on first build from a
   sensible default channel; subsequent builds use the lock (fetch via narHash — pure);
   `cix build --update-lock` refreshes. Lock file is meant to be committed.
4. **CLI**: `cix build [dir] [-t ref]` — finds `dir/Cixfile`, builds, prints the store path;
   `-t` tags via the cix-index crate API (add the dependency). Wire into the existing
   flatten-enum pattern in `crates/cix/src/main.rs`.
5. **Examples as Cixfiles**: `examples/nginx/Cixfile` and `examples/postgres/Cixfile` next to
   the existing `default.nix` (which stays — the coexistence is the point). Use `/item` paths
   in file contents and LINK for cross-package references, per docs/cixfile.md. The nginx one
   should match the worked example in the doc (fix the doc via a note in LOG if reality
   forces a deviation — do not silently diverge).
6. **Verification gate**: for both examples, `cix build` the Cixfile and run the same
   end-to-end checks the demos perform (sudo system-mode run; nginx: curl the page; postgres:
   TCP `SELECT 1`), plus a semantic comparison of the generated `cix-spec.json` against the
   `default.nix` version's (same services/env/ports/dirs — allow value-form/lifecycle
   differences ONLY where the .nix versions haven't adopted D21/D22 yet; note diffs in LOG).
7. **Tests**: parser unit tests incl. every error case above; a golden test Cixfile →
   generated spec JSON; lock lifecycle tests (create/reuse/update; narHash mismatch fails
   loudly). Building in tests may use the real nix with the committed lock.

## Done gate

fmt/clippy/tests green; both Cixfile-built items pass their end-to-end checks under sudo; no
leftover units; committed; LOG final summary with deviations + any doc corrections needed.
