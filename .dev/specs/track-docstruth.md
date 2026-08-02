# track/docstruth — the design docs catch up with the implemented board

Read AGENTS.md first (focused agent gate; synchronous receipts). Pure
docs track — no code. Work in
`/home/mathijs/worktrees/composix/track-docstruth` (herdr worktree) on
branch `track/docstruth`. Keep `nix/LOG.md`? No — use `.dev/` — this
track's LOG is `crates/cix-cixfile/LOG.md` per convention.
FENCE: touch only docs/*.md, docs/index.md, README.md,
docs/open-questions.md, cips/accepted/*.md changelogs. No code, no
generated docs (tour/corpus pages regenerate only if a source doc they
embed changed — avoid that).

Context: every adopted CIP (75–90) plus D70 is now implemented and
CI-confirmed; netns/tree/health/secrets/dirs/closed-root all landed
TODAY. The prose has not caught up.

1. docs/design.md "Building now": rewrite to the current truth — the
   adopted board is fully built; name what is actually next (phase-2
   closed-root flip, D26/D27 named networks/talks-to, publish era,
   reconciler) as the honest frontier. Keep it short; design.md is a
   registry, not a diary.
2. docs/index.md: "networking and the broader compose tree are still
   being built" is false — replace with the current status line.
3. README.md status table: compose row "in progress" understates
   (trees, pods/netns, health, secrets, dirs materializations,
   observability all landed) — update the four-part table honestly,
   including manifest version truth (v0 only, from the thin1 audit).
4. docs/open-questions.md: the era-parked list is stale — D49/netns is
   BUILT (split the row: D26/D27 named-network/talks-to remain
   parked); sweep the whole file against today's landings; add to
   "open for agents": the systemd-257 adapter-liveness finding from
   the Mastodon receipt (CIP-79 adapter pinger not retained on 257 —
   needs a version-gate or mechanism fix; cite the receipt).
5. cips/accepted changelogs: one dated "implemented" line per CIP that
   landed today and lacks one (79, 81, 82 leg 2, 84 phase 1, 85 leg 1,
   86, 89 leg 1, 90 leg A) — one line each, no prose.

Gate (agent side): fmt-noop, but run: corpus + tour drift tests (must
stay green — you touched no sources they embed), link check by
inspection, `git diff --stat` sanity. Commit when green.
