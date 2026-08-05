# track/regen4 — wave-4 assembly (adminer/nginx/wallos) + env-wall re-verification

Process mirrors the regen3 assembly (read .dev/specs/track-regen3.md
"Ledger + close" and docs/corpus.md maintenance rules). Branch
`track/regen4`, this worktree; LOG `corpus/migrate/LOG.md`. Staging
outputs: `/home/mathijs/regen-stage/docker-{adminer,nginx,wallos}`.
Worker greens are claims — re-verify independently (check.sh receipts
synchronous; captured statuses).

Per case:
- **adminer** — STOPSIGNAL-era regen; carries an honest cold read-set
  wall (generated `output` dir absent in cold workspace) with a
  language finding: "no way to declare generated directories as
  non-inputs" → promote that to a `cips/draft/` entry (CIP-light)
  citing the case.
- **nginx** — STOPSIGNAL-era regen; Mathijs live-corrected the pid
  path mid-staging (`/tmp/nginx.pid` → `/run/nginx/nginx.pid` in the
  declared RUNDIR) and the worker verified HTTP 200 after rebuild.
  Add the migrate.md addendum this teaches: runtime state paths (pid
  files, sockets) belong in declared role dirs, never /tmp.
- **wallos** — the CIP-98 exhibit: upstream `/var/www` layout with
  nested state roles, no `/app` relocation. Its NOTES record the
  probe-grammar stumble that produced draft/probe-url.md — harvest
  any further FRICTION content into that draft's evidence.
- **caddy + filestash env-wall re-verification** (from regen3's
  merge): caddy's assembly rerun saw one identical 769-byte body for
  distinct pinned GitHub assets; filestash died at DNS resolution.
  Both smell environmental, not case-truth. Re-run both check/build
  paths in this clean worktree; if they now behave, re-grade the rows
  and record the earlier receipts as environment-tainted; if they
  reproduce, the walls are real — say which.

Ledger: re-grade affected docs/corpus.md rows (new ribbon vocabulary),
GAPS Status current/stale-with-reason, docker.md rows if affected,
browser regen. Harvest every staging NOTES FRICTION section into
draft evidence or LOG findings — that harvest is a deliverable.

Gates: full agent tier, capture-as-epilogue (value-checked), bounded
VM parallelism. Parallel tracks in flight — merge semantically. Clean
branch; do not merge.
