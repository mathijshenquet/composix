# track/absdest — D66: absolute artifact destinations

Read AGENTS.md first. Authoritative: design.md **D66** (read it fully). Scope:
crates/cix-cixfile (+cix-run only if manifest loading needs it — the manifest
format itself should NOT change: item-relative storage stays as is), examples,
corpus/migrate (all 14 Cixfiles — fetch contexts first: `cd corpus/migrate &&
./fetch.sh --all`), docs (cixfile.md, migrate.md), tour, README.

1. Parser: in SERVICE/APP blocks, COPY/FILE destinations and LINK linkpaths
   REQUIRE a leading `/` (item-world absolute); a relative destination is a
   migration-grade error naming the absolute spelling. `EXEC`/`SETUP` path
   forms: `/bin/x`-style absolute stays, `bin/x`-style relative becomes the
   same migration-grade error; bare `EXEC x` (D64) unchanged. BUILDER blocks
   are UNTOUCHED: workdir-relative destinations (`.`) stay, and an absolute
   destination in a BUILDER stays illegal as today (verify; if currently
   accepted, make it an error — the workshop has no `/`).
2. Storage/realization unchanged: `/srv/www/x` still lands at `<item>/srv/www/x`;
   manifests, mounts, PATH-for-bin (D64) all byte-identical for equivalent
   inputs — prove with one before/after manifest comparison test.
3. Sweep every artifact-block destination spelling: examples/**, corpus
   Cixfiles (re-run each pair's `./check.sh cix` that was green in the
   2026-07-31 receipts — they must stay green; append results to
   corpus/migrate/LOG.md), docs/cixfile.md, docs/migrate.md (its verified
   samples: re-verify them per its own scratch procedure), README sample
   (keep the verbatim property!), tour regen.
4. Gate: `devenv shell -- cargo fmt --all --check`; warning-denied workspace
   all-target clippy; `cargo test --workspace`; tour regen + drift +
   determinism twice; `devenv shell -- nix build
   .#checks.x86_64-linux.vm-dogfood --no-link -L`; the corpus re-checks and
   migrate.md sample re-verifications recorded. Exact repros + unit cleanup in
   crates/cix-cixfile/LOG.md. Commit on this branch when green.
