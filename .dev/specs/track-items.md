# track/items — ITEM plucks, CACHE dirs, manifest v4 def-node (D40 + D41)

Read AGENTS.md first. Authoritative design: docs/design.md **D40, D41** (context: D39,
D31/D32; story: docs/compose-tree.md §1). D40/D41 win on conflict. Scope:
`crates/cix-cixfile` (grammar, build, codegen), `crates/cix-run` (manifest v4 parsing),
ripple through `crates/cix`, examples, tour. Do NOT build the compose tree (D42/D43) or
touch `crates/cix-index` beyond what multi-item tagging needs — track/index2 owns that
crate concurrently; if you need an index API that doesn't exist, note it in your LOG and
work around with the current API.

## Cixfile surface

1. **`ITEM <name>` supersedes `SERVICE <name>`** as the block keyword (hard rename, no
   alias — pre-1.0 honesty; update every example/fixture/tour). A Cixfile may contain
   multiple ITEM blocks; each produces its OWN store item and its own tag on build
   (`cix build . -t v7` tags `<cixfile-dir-name>-<item>:v7`… no: tag naming rule below).
2. **`TAKE <build-path> <item-path>`** (repeatable per ITEM): declared plucks. The item's
   filesystem = exactly the TAKE'd subpaths of the final build snapshot, placed at their
   declared item paths. `EXEC`/`LINK`/`PATH` inside an ITEM block resolve against the
   item filesystem (relative) or interpolated store paths as today. `${build}` remains
   valid inside `TAKE` source position; an ITEM with no TAKE and an `EXEC ${pkg}/bin/x`
   stays valid (pure-closure item, e.g. the pack examples).
3. **`CACHE <dir>`** (repeatable, prelude position like PATH): advisory per-step cache
   directories. Mechanics: mounted writable into every RUN sandbox at `<dir>` (relative
   to workdir), persisted across builds in a host-local cache location keyed by
   (Cixfile identity, dir) — OUTSIDE the memo key, OUTSIDE workdir snapshots, OUTSIDE
   the store. A cold cache must produce the same items as a warm cache; add a
   `cix build --no-cache` flag that ignores cache dirs (this is the sampled-clean-rebuild
   hook; wire it, don't build a sampling scheduler).
4. Tag naming for multi-item builds: `cix build . -t <tag>` applies `<tag>` to every
   item as `<item-name>:<tag>` (the ITEM name is the index name). A single-ITEM Cixfile
   therefore keeps ~today's ergonomics. `-t name:tag` (explicit full ref) is an error
   when the Cixfile has >1 ITEM.

## Manifest v4 (D41) — `crates/cix-run`

- `cix-manifest.json` v4 = ONE bare def-node: top-level `cixManifest: 4` plus the
  service body fields directly (`exec`, `setup`, `env`, `ports`, `listeners`, `dirs`,
  `health`, `jit`, and new `outbound: bool` default false — parse + carry it; no
  enforcement this track, that's netns-land). No `services` map.
- Runner accepts 1–4 (D15): v1–v3 files keep parsing (internally: normalize a v≤3
  multi-service manifest to N virtual single services as today), but `cix run <item>`
  on a v≤3 manifest with >1 service now emits a loud deprecation pointing at D41.
  v4 codegen is what `cix build` emits — one manifest per ITEM's store item.
- Kill the `service:` selector in compose (`crates/cix-compose/src/model.rs`
  `ComposeService.service`): field removed, compose resolve errors on multi-service
  items with a message citing D41. (This is the only compose touch allowed.)

## Gate: proj1 (D40c)

Create `examples/build/proj1/`: a real two-item application — a rust workspace building
`proj1-api` (HTTP, declared port, uses CACHE for the cargo target dir) and
`proj1-worker` (`outbound: true` in its manifest via the Cixfile, no ports), each an
ITEM with TAKE plucks of just its binary. The gate, all e2e and left as exact repro
commands in your LOG:

1. `cix build . -t v1` produces two items; each manifest is v4 bare; each item contains
   ONLY its plucked binary (no cargo bookkeeping — assert via a file listing in a test).
2. Warm rebuild after touching only worker source: api item store path UNCHANGED,
   worker item changes, cargo CACHE made the rebuild incremental (assert memo behavior;
   wall-time assertions are informative, not gating).
3. `cix build --no-cache` reproduces both item store paths byte-identically (the
   D39.1/D40b soundness check, run once in CI).
4. `cix run` of proj1-api serves; curl proves it (tour scenario).

## Meta loops

Regenerate the tour (expect real diffs: ITEM keyword, v4 manifests — review them, the
build-with-run and inspecting pages change). Update examples (pack/*, build/*,
dstyle keeps its .nix path — dstyle emits manifests directly: port its generated
manifests to v4 single-service form). docs/docker.md rows that cite SERVICE/multi-service
manifests: update honestly. `cargo test --workspace`, clippy `-D warnings`, `fmt --check`
all green; keep your assigned LOG.md current, append-only, timestamped.
