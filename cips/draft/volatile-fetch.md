# volatile-fetch — EXPECT-hostile fetch content: teach, lint, normalize (CIP-light)

Status: **draft, CIP-light** (2026-08-04; promoted out of open-questions).

**Problem.** Several corpus cases pin FETCH outputs whose bytes change
on every refetch even though the *consumed* content is stable: GitHub
release-metadata JSON carries download counters (traefik), package-
manager caches retain nondeterministic subsets (dozzle's Go sumdb
tiles, node_modules trees for echo-server/parse-server), and a mirror
redirect changed phpmyadmin's tree. A pin taken today fails on any
cache loss, so "green" quietly depends on warm state.

**Proposal.** Three layers: (1) migrate.md teaches the normalization
idiom per class (jq the consumed fields out of API JSON, keep manager
caches out of the workdir, fetch stable asset URLs directly); (2) a
build-time lint flags a FETCH whose pinned tree contains known-volatile
shapes (API JSON with counters, cache directories) — informational,
never failing; (3) the affected corpus cases get normalization fixes in
one sweep once (1) lands.

**Effort.** Small-medium; mostly teaching + lint, corpus sweep is
mechanical.
