# track/migrate-r5 — build-class round on the no-escape set

Read AGENTS.md first. The method: docs/migrate.md IS the migration prompt —
follow it as written; where it fails you, that is a finding, not a license to
improvise silently. Corpus conventions per the existing pairs in
corpus/migrate/ (SOURCE pin + fetch.sh contexts [gitignored], check.sh with
docker and cix modes, receipt.md, honest fails welcome). Candidate data:
the "No-escape additions" table in corpus/migrate/CANDIDATES.md. Runs parallel
to track/famref (crates) — do NOT touch crates/**; use the cix binary built in
THIS worktree (`devenv shell -- cargo build`).

## Batch (in order; stop when out of budget, record where you stopped)
1. excalidraw (easy, node/yarn → static)
2. parse-server (easy, node)
3. wallos (middling, PHP — NOTE: `docker-php-ext-install` composes PHP
   extensions; `php.withExtensions` is a function call and NOT Cixfile
   territory per D32 — this pair deliberately exercises the `.nix` escape
   hatch (D4). Document how that feels; it is a primary finding either way.)
4. directus (middling, pnpm workspace monorepo)
5. filestash (middling, Go+cgo against libvips/ffmpeg/libraw/libheif — `lib`
   is deliberately absent from the IMPORT union (D58); expect explicit env
   flags (CGO_CFLAGS/LDFLAGS via builder ENV) or an honest gap finding)
6. LEGACY: tomcat — diagnose why the built item never becomes reachable
   (receipt exists; get to a root cause, fix or document)
7. LEGACY: dozzle — do not fix; DOCUMENT the FETCH-pin instability precisely
   (which bytes differ between two `go mod download` runs) as input for the
   coming pin-stability design round.

## Rules
- Dual receipts: docker build+probe once per candidate (record honestly, note
  date), then the cix side. A pair passes only when the natural probe from
  CANDIDATES.md succeeds against the hardened unit.
- Class-split grading is the deliverable: report per class (node, php, go+clib)
  attempted/passed, and classify every failure as language gap, product gap,
  prompt gap, or upstream flake. The build-class loss rate is the real grade.
- Time-box: a candidate whose docker OR cix build exceeds ~20 minutes is
  recorded as slow-tier and skipped, not fought.
- No crate changes; no design.md edits (propose amendments in the LOG for the
  orchestrator). New product/prompt findings go in corpus/migrate/LOG.md.
- Cleanup: stop/reset all test units; `git ls-files 'corpus/migrate/*/context/**'`
  stays empty.

## Gate
Every new pair's check.sh runs recorded with exact commands in
corpus/migrate/LOG.md; `devenv shell -- cargo test --workspace`
untouched-green as smoke; diff confined to corpus/migrate/**. Commit on this
branch when done — "done" includes honest fails; do not stall polishing.
