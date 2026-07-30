# track/tourbook — regroup the tour into chapters; D50 ITEM removal; D51 language ergonomics

Read AGENTS.md first. Authoritative: docs/design.md D50, D51, and Mathijs's tour
review (2026-07-30, transcribed below as requirements). Scope: `crates/cix/tests/tour.rs`
(scenario grouping + prose), `crates/cix-cixfile` (D50 removal + D51 grammar),
`crates/cix-run` (kind=item removal), examples, README, docs/tour regeneration.
SEQUENCING: run after track/repinfix merges (shared cix-run/compose territory).

## 1. Tour regrouping (Mathijs's review, point by point)

Restructure from 14 single-scenario pages into grouped chapter pages. Target shape:

- **Chapter: the index** — old 01/02/03 (tag, move, untag) + 13 (inspecting) as one
  page: tag → inspect → move → untag reads as one story.
- **Chapter: distribution** — old 04/05/06 (serving, pulling, pull-follows) one page.
- **Chapter: build, run, debug** — old 08 THEN 07 THEN 11 (build before run reads
  naturally; debug closes it). Requirements: `cat` the Cixfile in the build section;
  07's manifest must be `cat`-ed and explained (and the word "spec" is dead: it is
  the *manifest* — sweep 07/09/index.md prose, and check README for lingering
  "spec" vocabulary); 11 (debugging) addresses its target by TAG syntax, not a store
  path.
- **Chapter: building with RUN** — old 12, now `cat`-ing (and `ls`-ing) the working
  directory it builds from.
- **Chapter: proj1** — old 14, updated to the D51 ergonomics (below).
- **Advanced chapter** — old 09 (listeners): `cat` the listener fixture so the
  reader sees what "listener-fixture" IS; old 10 (composing): drive it from a real
  Cixfile-built item instead of tagging an opaque fixture store path.
- **House rule, applied throughout**: any data/directory an example uses gets an
  `ls`/`tree`/`cat` first — the reader must see what exists before it is used.
- Renumber pages/links/index accordingly; the tour harness (generator, drift,
  determinism, normalization) keeps working unchanged mechanically.

## 2. D50: remove the ITEM block

Parser: ITEM keyword removed; migration-grade error ("ITEM was dropped (D50); use
SERVICE or APP; a future content-only block would be ASSETS"). cix-run: kind="item"
handling removed (manifest kind service|app only). Remove any fixtures/tests for
ITEM; adjust artifact_kinds tests.

## 3. D51: COPY dir-preference and RUN continuations

- **`\` line continuation** on directive lines (docker-familiar), and a heredoc form
  for RUN: `RUN <<EOF … EOF` (workshop scripts; body executed via the builder shell;
  same memo semantics — the body is part of the command key). Line-numbered errors
  keep pointing at the physical line of the failure.
- **proj1 simplified**: the eight COPY lines become `COPY ${src}/rust/ .`; the
  cache-state marker logic in tour 14's RUN moves into a readable heredoc. Sweep
  other examples for enumerate-COPY that has no memo-granularity reason (keep
  deliberate manifest-first COPYs like projB-chef, with a comment saying why).
- Glob COPY (`**/Cargo.toml`) is NOT built (D51 records it noted-not-built,
  evidence-gated).
- **No-op BUILDER sweep (Mathijs's README review)**: all five pack examples (and the
  README's opening Cixfile) grew a BUILDER during the D47 migration that only COPYs
  from `${src}` and is then re-COPY'd into the SERVICE — pure assembly needs NO
  builder; COPY sources belong directly in the SERVICE block. Remove the no-op
  BUILDERs everywhere (README + examples/pack/*), rebuild each example as proof, and
  state the doctrine in the directive docs: *a BUILDER exists only when there is
  RUN/FETCH work to do*. The README opening example must be the shortest honest form.

## 4. D52: directive consistency (added while you may already be reading — check
design.md D52)

- Service-block `CACHE` → **`CACHEDIR`** (role dir, compiles to `CacheDirectory=`;
  kills the collision with builder CACHE; family now mirrors systemd's
  `*Directory=` names). Migration-grade error on old spelling in service blocks.
- **`LINK` flips to `LINK <target> <linkpath>`** (ln -s and COPY source-first
  conventions restored). Migration-grade error mentioning the old order. Sweep all
  examples/docs/tour accordingly.

## Gate

`cargo fmt --all --check` · clippy `-D warnings` · `cargo test --workspace` · tour
regenerated (expect real diffs — review every page as prose, this track IS the
prose) + drift + determinism twice · `vm-dogfood` · `compose-fallback-vm` ·
scenario-lifecycle (guard: tour/grammar changes must not disturb the tier). Exact
repro commands in crates/cix-cixfile/LOG.md.
