# track/argvenv — D59: builder ENV + quote-aware EXEC/SETUP argv

Read AGENTS.md first. Authoritative: design.md **D59**. SEQUENCED AFTER track/keys
merges (same crate). Scope: crates/cix-cixfile (+cix-run only if argv carriage
touches manifest loading), examples, docs/cixfile.md, tour where shown.

1. `ENV NAME = value` legal in BUILDER blocks: applies to steps after the line;
   plain values; participates in chain keys as declared text; injected as an
   export-prelude THROUGH the step shell (so `$PWD` expands per step — test this).
2. EXEC/SETUP tokenizer: single+double quotes preserve spaces; unterminated quote =
   line-numbered error; a manifest test proves `EXEC nginx -g 'daemon off;'` yields
   argv element `daemon off;`. Other directives unchanged.
3. Gate: fmt/clippy/workspace tests, tour regen+drift+determinism, vm-dogfood.
   Exact repros in crates/cix-cixfile/LOG.md.
