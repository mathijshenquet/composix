# phase-blocks — explicit `{ }` delimiters for BUILDER/ITEM/SERVICE/APP

Status: **draft** (2026-08-05; from the nodes-and-edges design round —
Mathijs: the vague phase-indent convention deserves its own decision).
Companion to [nodes-and-edges](nodes-and-edges.md); independent enough
to decide separately.

## 1. The problem

Phase blocks are keyword-delimited with an implicit end: a BUILDER
runs until the next top-level block starts, indentation is cosmetic,
and membership is positional. That was fine when Cixfiles were a
dozen lines. The nodes-and-edges round makes files structurally
richer — LET preludes, per-node WITH/EXPECT clauses, heredoc bodies —
and implicit termination becomes the weakest part of reading a long
file: where a phase ends, and whether a stray step belongs to the
block above it, is knowable only by scanning for the next keyword.
Docker culture papers over the same gap with `######` comment fences,
which is a smell, not a mechanism.

## 2. Prior work

- **Caddyfile** — the closest analog and the strongest precedent:
  keyword+name header, `{ }` block, line-oriented directives inside,
  shorthand for trivial cases. Widely considered one of the most
  pleasant config grammars.
- **HCL/Terraform** — `resource "a" "b" { … }`: keyword-headed brace
  blocks as the backbone of a beloved declarative language; nesting
  and tooling (folding, matching, auto-indent) come free.
- **nginx** — `server { location / { … } }`: same lineage, decades of
  familiarity.
- **systemd INI** — `[Section]` headers: explicit but flat (cannot
  nest), and visually dated; our SERVICE blocks map to units yet the
  unit *file* format is not a grammar to emulate.
- **YAML/GHA** — significant whitespace as the block mechanism: the
  anti-example; nodes-and-edges already rejected significant
  indentation for clauses, and phase blocks should not reintroduce it.
- **Docker** — no blocks at all; `######` fences by convention.
- **Our own grammar** — keyword-driven blocks, cosmetic indent, `cix
  fmt` canonicalizes: works, but the end-of-block is the one piece of
  structure the text does not state.

## 3. Recommendation

Braces at phase level, and only there:

```
BUILDER runtime {
  IMPORT ${pkgs.bash} ${pkgs.curl}
  FETCH curl --fail … 
    EXPECT sha256-…
  RUN tar -xf nats-server.tar.gz
}

SERVICE web {
  EXEC nats-server -c ${CONFIG}
  ENV PORT=4222
}
```

- Opening brace on the header line (Caddy/HCL convention), closing
  brace on its own line; `cix fmt` owns the layout as today.
- Node attachments (WITH/EXPECT) stay adjacency-bound and indented —
  braces never appear below phase level; the two mechanisms answer
  different questions (phase membership vs node attachment).
- The file prelude (FROM/LET/ARG before the first block) stays
  braceless: it is the file's own scope, not a phase.
- `######` fences lose their reason to exist; fmt keeps comments but
  the language stops needing them structurally.
- Migration: mechanical repo-wide sweep (corpus/examples/tour),
  alpha-cheap, same class as the ENV-grammar sweep.

## 4. Open questions

- **Nesting**: compose-side subtrees and ITEM-in-context cases — do
  any current constructs nest phases, and if so do braces nest with
  them, or is single-level explicitly the rule?
- **Shorthand**: Caddy allows one-line blocks; do we want
  `SERVICE probe { EXEC true }` or is multi-line always canonical
  (fmt could expand one-liners)?
- **Epoch coupling**: land together with nodes-and-edges as one
  language epoch (one corpus sweep, one migrate.md rewrite) or as a
  separate earlier/later track? One epoch means one round of churn;
  separate means smaller reviewable steps.
- **Error ergonomics**: the unclosed-brace diagnostic must beat
  today's silent phase-absorption — what does the parser say when a
  `}` is missing at EOF?

## Effort

M as its own sweep; marginal if folded into the nodes-and-edges epoch.
