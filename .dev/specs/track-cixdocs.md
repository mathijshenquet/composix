# track/cixdocs — docs/cixfile.md currency pass for CIP-91/92

Read AGENTS.md first, then cips/accepted/0091-artifact-import.md and
0092-port-protocols.md (Decision sections). Work in the herdr worktree
on branch `track/cixdocs`. Keep `crates/cix-cixfile/LOG.md` current
(dated track heading; commit it).

docs/cixfile.md still teaches the pre-CIP-91 world throughout: LINK as
a first-class directive (§ block table, §LINK section, examples at
lines ~32/85/134/271/483–550), interpolated `START ${pkgs.bash}/bin/sh`
canon, no IMPORT in the SERVICE/APP/ITEM directive table, no
store-aware COPY semantics, no `udp:` ports, no `cix build --file`.
docs/migrate.md is already current — use it as the canonical phrasing
source and keep the two documents consistent without duplicating
migrate.md's teaching voice (cixfile.md is the reference, migrate.md
the tutorial).

Sweep the whole file:

1. Block/directive table gains IMPORT everywhere it now applies; LINK
   becomes one line: deprecated alias for the equivalent COPY (corpus
   transition only).
2. A store-aware COPY section (link-by-rule for store sources, the two
   static materialization triggers) replacing/absorbing the LINK
   section.
3. All examples to the canon: IMPORT + bare argv; no interpolated
   START; role dirs at app-native paths where shown.
4. PORT documents the `udp:<port>` form + the Docker-form hint;
   `cix build --file` documented where build invocation is described
   (sibling lock naming included).
5. The /usr/bin/env alias paragraph stays; do not document CIP-95's
   FHS surface (in flight on another track).
6. Grep the rest of docs/ (excluding docs/tour/ generated content,
   docs/migrate.md, docs/corpus*) for stale LINK-first or interpolated
   START teaching and fix in the same pass; list what you touched in
   the LOG.

FENCE: track/fhspaths runs concurrently (crates + migrate.md + a future
cixfile.md section) — touch ONLY documentation files, never crates/,
and not docs/migrate.md.

## Gate

Standard docs tier: fmt (no-op), full workspace tests, tour regen+drift,
corpus browser drift (should be untouched — verify, don't regenerate).
Synchronous receipts.
