# track/index2 — the index re-founded: names → content-addressed tag tables (D45)

Read AGENTS.md first. Authoritative design: docs/design.md **D45** (context: D44, D35;
story: docs/compose-tree.md §5). D45 wins on conflict. Scope: `crates/cix-index` (+ the
thin CLI wiring in `crates/cix`). Do NOT touch compose/cixfile/run semantics.

## Model

Replace the per-tag storage model (JSON sidecar per ref + GC-root symlink per ref) with:

1. **Tag table item**: per *name*, one store item containing `table.json`:
   - `{"cixTagTable": 1, "name": "<name>", "parent": "<narHash of predecessor table item
     or null>", "tags": { "<tag>": { "storePath": "...", "narHash": "...",
     "meta": { ...existing TagMetadata fields... } } } }`
   - Deterministically serialized (sorted keys, stable pretty-print) so identical tables
     hash identically. Added via `nix store add-path` like other items.
2. **Name pointer map**: the only mutable cell. A directory `names/` with one small file
   per name: `names/<encoded-name>` containing the current table item's store path +
   narHash (JSON, atomic tmp+rename write, same encoding scheme as today's refs).
3. **GC roots**: one root per name for the *current table item*, plus roots for the
   store paths the current table references (a table flip re-registers roots and drops
   roots only for paths no longer referenced by the new table). Yanked-but-historical
   table items are NOT rooted (history survives only while the store keeps them — this
   is D45's honest yank semantics; do not build a keep-history GC policy in this track).

## Operations (rework the existing Store API in place; keep public fn names where they fit)

- `publish(name, tag, artifact)` → load current table (or empty), insert/replace tag,
  set `parent` to the current table's narHash, add new table item, **CAS the pointer**:
  the write fails with a distinct error if the pointer changed since the read (re-read,
  retry once, then surface the conflict). Multi-tag publish in one flip:
  `publish_many(name, [(tag, artifact)…])`.
- `yank(name, tag)` → new table without the tag, same CAS flip. Removing the last tag
  leaves an empty table (name keeps its history chain); a separate `remove_name` deletes
  the pointer + roots.
- `resolve(name:tag)` → pointer → table item → entry. `all()` / `ls` iterate names then
  tags. Preserve the existing output shapes of `cix ls` / `cix inspect` (SYSTEMS column
  etc.) — golden-test them before refactoring so the surface provably does not drift.
- `history(name)` (new, minimal): walk parent hashes while the store has them; print
  table hash + tag summary per step. Wire as `cix index history <name>` (plumbing-level;
  no porcelain polish needed this track).

## Explicitly OUT of scope

Signing, auth enforcement (D45 records name-level auth as the *model*; there is no
remote publish path yet — leave a `// D45: auth = may-move-this-name, enforced at the
serve/publish boundary when it exists` marker where the CAS happens), transparency log,
TTL/staleness handling for remote pointers, parametric composes (D46, separate track),
serve/pull protocol changes beyond what resolution requires.

## Migration

`Store::open()` on a store with old-format sidecars migrates once: group sidecars by
name, build one table item per name (parent = null), write pointers, move old sidecars
to `meta.legacy/` (do not delete). Migration is idempotent and covered by a test using
a fixture of the old layout.

## Gate (leave exact repro commands in your LOG)

- `cargo test -p cix-index` covering: publish/resolve round-trip, CAS conflict path,
  multi-tag atomicity (a reader mid-publish sees old table or new table, never a mix),
  yank advisory semantics (resolve fails fresh, direct store path still loads), history
  walk, migration fixture, GC-root accounting after flips.
- `cargo test --workspace` + clippy + fmt clean; tour regenerated with **zero diff**
  (the index rework must be invisible at the CLI surface except `cix index history`).
- Keep `crates/cix-index/LOG.md` current, append-only, timestamped.
