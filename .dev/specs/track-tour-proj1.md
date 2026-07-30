# track/tour-proj1 — make tour 14 speak: show the Cixfile, prove the CACHE

Small follow-up round; run AFTER track/blocks (D47) merges — the page must show the
new block syntax. Read AGENTS.md first. Scope: the proj1 tour scenario in
`crates/cix/tests/tour.rs`, `examples/build/proj1/**`, regenerated docs/tour. Mathijs's
ask, verbatim intent: "laat in tour 14 ook een cat van de Cixfile, en laat zien dat
die cache daadwerkelijk wat doet — laat het spreken."

## Additions to the proj1 tour page

1. **`cat` the Cixfile** near the top (`$ cat examples/build/proj1/Cixfile` style, the
   real file, D47 blocks syntax) so the reader sees the language before the build runs.
2. **Prove the CACHE deterministically.** HARD CONSTRAINT: no raw timings in tour
   output (the tour is drift-checked; ms values are normalized away — a `time` won't
   survive). Mechanism (or an equally deterministic alternative of your choosing):
   the RUN step records a cold/warm marker derived from whether the CACHE dir carried
   prior build state, e.g. `test -e target/.cix-warm && echo warm || echo cold`
   written to `output/cache-state` (plus `touch target/.cix-warm`), the marker COPY'd
   into an item or cat'ed from `${build}` in the scenario. The page then shows, in
   order:
   - first build: memo miss + `cache-state: cold`
   - edit only the worker source, rebuild: memo miss (source changed!) but
     `cache-state: warm`, api item store path UNCHANGED — one screen that separates
     the two mechanisms: memo = skip identical steps, CACHE = make changed steps
     incremental
   - `cix build --no-cache`: `cache-state: cold` again, item paths byte-identical —
     the soundness check, visible.
   A `build.rs` cargo:warning variant is acceptable instead IF its output is captured
   deterministically in the toured output; the marker-file approach avoids depending
   on cargo's warning surfacing.
3. Keep prose short and declarative; the page should read as the D40 story told by
   the terminal itself.

## Gate

Tour regeneration + drift + determinism (run the determinism test twice), fmt/clippy/
`cargo test --workspace`, and the proj1 e2e test still green. Exact repro commands in
crates/cix-cixfile/LOG.md (append-only).
