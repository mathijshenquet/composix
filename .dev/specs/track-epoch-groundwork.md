# track/epoch-groundwork — CIP-111/112/113 parser, fmt, executor (NO corpus sweep)

Charter: the Decision chapters of `cips/accepted/0111-nodes-and-edges.md`,
`0112-phase-blocks.md`, `0113-build-args.md`. This track builds the
language behind tests; the corpus sweep + migrate.md rewrite are a
LATER track — no corpus file changes here beyond keeping the suite
green on the OLD corpus (old grammar keeps parsing until the sweep).

Scope:

1. **Grammar + AST + fmt** (CIP-112 first, it is the smallest): phase
   braces `BUILDER name { … }` single-level; parser whitespace-
   agnostic, fmt canonicalizes multi-line; missing `}` errors with
   the opening line; fmt tolerant-mode auto-close when unambiguous.
2. **CIP-111 nodes and edges**: argv-first RUN/FETCH (one command per
   node; `$X` in argv position is the teaching parse error);
   mandatory-interpreter heredocs (body to file, interpreter invoked
   with the filename); adjacency-bound clauses WITH/EXPECT (inline
   forms parse, fmt canonicalizes to indented clause position);
   `LET NAME = value` juxtaposition word-lists (fish semantics: every
   value is a list, scalar = singleton; quoting binds spaces; `${NAME}`
   expands list values into that many argv elements); `WITH NAME=v` /
   bare `WITH NAME` env edges; builder-scope ENV becomes a parse-time
   teaching error (leaf-phase SERVICE/APP ENV untouched);
   `WITH UNSAFE IGNORE <path>` parses, excludes the path from
   read-set evidence/seal/keys, and emits the waived-evidence
   diagnostic at use; no SHELL directive.
3. **CIP-113 build-args**: `ARG NAME from v1 v2 …` (first value =
   default), `cix build --arg NAME=value` selects a declared cell
   (undeclared → error listing declared), `--all-args`, per-cell
   append-only lock entries via the existing resolved-statement
   keying, manifest records the selection.
4. **Keying**: consume the canonical-serialization seam that
   track/fmtkey-impl owns (coordinate through the merged API once it
   lands; until then a clearly-marked internal stub with the same
   shape). New constructs key per CIP-111's edge-granularity section.
5. **Tests**: parser/fmt round-trips for every construct, executor
   tests for argv/heredoc/WITH env/LET expansion/ARG selection, the
   teaching-error messages asserted verbatim (D73 doc anchors, no
   CIP numbers), fmt idempotence.

Old grammar: keep existing corpus/examples parsing and passing
untouched this track — dual acceptance until the sweep flips the
corpus; mark old-form acceptance clearly so the sweep track can
delete it.

Gates: fmt, examples fmt, warning-denied clippy,
`cargo test --workspace`, tour regen+drift if touched, focused VM via
`devenv shell -- nix run .#progressive-vm-check`.

Discipline: branch `track/epoch-groundwork`, LOG `crates/cix/LOG.md`
(append, timestamped, FRICTION section). If a CIP decision turns out
ambiguous or contradictory in implementation, STOP on that item,
record the exact conflict, and continue with the rest. Synchronous
value-checked receipts only. Clean committed branch; do not merge.
