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
3. D52 addendum: rename `STATE` → `STATEDIR` (migration-grade error; sweep
   examples/docs/tour). Ask-in-LOG whether LOGS/CONFIG should follow
   (LOGSDIR/CONFIGDIR) — flag it, don't decide it.
3b. DECIDED — design.md **D60**: the `GRANT <capability>` family. Implement the
   day-one flip: `GRANT jit` / `GRANT egress` replace the `JIT`/`EGRESS`
   directives (hard flip, migration-grade errors pointing at the new spelling).
   Grammar: repeatable, ONE grant per line (no multi-arg), legal in SERVICE and
   APP blocks only; unknown grant name = error listing the vocabulary. Manifest:
   a `grants` string list replaces the `jit`/`egress` booleans (version gate per
   D15); egress keeps its D49(a) compose-level usage override semantics
   unchanged. No new vocabulary members — jit and egress only, semantics
   untouched. Sweep examples, docs (cixfile.md, docker.md cap-add row), and tour
   chapters where JIT/EGRESS appear.
4. Gate: fmt/clippy/workspace tests, tour regen+drift+determinism, vm-dogfood.
   Exact repros in crates/cix-cixfile/LOG.md.
