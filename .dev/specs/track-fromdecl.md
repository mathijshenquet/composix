# Track: fromdecl — required FROM binding package universes (D32 amendment)

Read `docs/design.md` D32 incl. its amendment — the contract. Builds directly on the just-
landed nopkg work. Environment: gates via `devenv shell -- …` or PATH cargo; never wait.
Territory: `crates/cix-cixfile/`, `examples/` Cixfiles, `docs/cixfile.md`, the FROM row in
`docs/docker.md`. NOT `crates/cix/tests/`/`docs/tour/`. COMMIT AS YOU GO. Log:
`crates/cix-cixfile/LOG.md`.

1. Parser: `FROM <flakeref> AS <name>` — `AS` is REQUIRED (no default namespace; Mathijs:
   nothing implicit, the binding is always written). Must precede any `${…}` use; missing
   FROM → error ("every Cixfile begins with FROM; try: FROM nixpkgs AS pkgs"); FROM without
   AS → error showing the AS form; duplicate namespace → error. Bare `nixpkgs` = the registry ref; otherwise
   accept the flakeref forms `github:owner/repo[/ref]` and https tarball URLs (boring subset;
   error clearly on the rest).
2. Lock: `Cixfile.lock` keyed per input name {url, rev, narHash}; existing single-input locks
   migrate on read (write the new shape; note it). `cix build --update-lock [name]` updates
   one or all. Resolution/pinning per input; multiple FROMs = multiple pinned universes.
3. Interpolation: `${<ns>.<attrpath>}` resolves against its universe; unknown namespace →
   error listing declared ones.
4. Examples: add the one `FROM nixpkgs AS pkgs` line everywhere; one example (node-app) additionally
   demonstrates `AS` with a second pinned universe ONLY if it stays tasteful — otherwise show
   `AS` in docs alone (your judgment, note it).
5. Docs: cixfile.md — FROM row in the directives table (with the truthful-meaning framing
   from D32's amendment), worked example updated, lock section updated; docker.md FROM row
   updated.
6. Tests: missing-FROM error, AS namespaces, per-input lock create/reuse/update/migrate,
   unknown-namespace error; golden fixture.

Gate: fmt/clippy/`cargo test --workspace` ×2; nginx+postgres+compose-stack sudo demos; VM
check; no leftovers; committed; clean; LOG summary.
