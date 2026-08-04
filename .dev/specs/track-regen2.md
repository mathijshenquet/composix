# track/regen2 — assemble wave-2 regenerated corpus cases (luna, cold)

Read AGENTS.md and docs/corpus.md §"How this corpus is maintained", then
.dev/specs/track-regen1.md — this track mirrors it exactly unless stated
below. Work in the herdr worktree on branch `track/regen2`. Keep
`corpus/migrate/LOG.md` current. Twelve staging dirs:
`/home/mathijs/regen-stage/{adminer,wallos,tomcat,phpmyadmin,echo-server,
whoami,verdaccio,renovate,excalidraw,parse-server,watchtower,dozzle}`.

Per case: same copy/GAPS/receipt/probe procedure as regen1 (header
`Generated: migrate.md@<staging commit> · gpt-5.6-luna · 2026-08-04`;
adminer/wallos/tomcat staged at `e1978b6`-era, the rest at `4e05987`-era
— read the staging MIGRATE.md's git match if in doubt, else `unknown`).
Calibration unchanged: progress-driven, no taste-nit churn.

## Two-layer evidence rule (new vs regen1)

The day's GC runs wiped warm FETCH snapshots, so orchestrator rebuilds
ran effectively cold and several cases diverged. Record BOTH honestly:

- The worker's warm receipts (from staging NOTES) and your own warm
  rebuild where the memo still hits.
- Where a fresh build now fails with `FETCH EXPECT hash mismatch` or a
  cold read-set divergence (observed: phpmyadmin, excalidraw?,
  parse-server, dozzle — verify each), that is a **cold-stability case
  defect**: land the Cixfile as-is, gap bullet `→ case: fetch output is
  cold-unstable (EXPECT-hostile …); normalize in the volatile-fetch fix
  round`, and the receipt records both the warm pass and the cold
  failure with exact errors. Do NOT re-pin EXPECT values to make it
  pass today — that hides the instability.

## Orchestrator findings to fold in

- **adminer**: all morning gaps resolved (version binders, php -d
  tuning flags, webroot layout, design/plugin mutation contract via
  STATEDIR-seeded webroot, PHP extension imports) — say so; the
  `__cix_unset__` ENV sentinel is a smell worth one gap bullet
  (`→ language?: no way to declare an optional ENV without a default`).
- **verdaccio**: honest wall stands (pnpm --filter build fails under
  the sandbox); carried as before, now with luna's evidence.
- **phpmyadmin**: no dissolved twin because nixpkgs lacks phpMyAdmin —
  verify that claim (search the locked universe) and record it either
  way.
- **echo-server / parse-server**: their NOTES document cold read-set
  divergences (warm node_modules Directory vs cold Absent;
  cross-builder node_modules comparison) → each gets a gap bullet
  routed `→ language (cold divergence audit)`, citing CIP-87's cold
  divergence machinery.
- **watchtower**: the warm-root duplicate-COPY product finding from its
  NOTES gets promoted verbatim into docs/open-questions.md "Open for
  agents" (grep first — the orchestrator may already have filed it; if
  not, you file it).
- **excalidraw**: the declared-port-80-vs-served-18090 note — verify
  which is right against the Dockerfile and record.
- **dozzle / watchtower**: remain ❌ refused workloads (Docker control
  plane); their regen modernizes the build side only — Fidelity cells
  keep the refusal loud.

## Ledger + close

docs/corpus.md: re-grade the twelve rows achievement-first (regen1
merge style — load-bearing caveats only); regenerate the corpus
browser; migrate.md addenda ONLY if a `→ prompt` gap emerged that
regen1's addenda don't cover. Delete nothing else.

FENCE: track/cip94 and track/tour2 run concurrently — do not touch
nix/, crates/, docs/tour/, docs/nix-build.md. Your domain: corpus/,
docs/corpus.md, docs/corpus/, docs/migrate.md (addenda only),
docs/open-questions.md (the watchtower promotion), your LOG.

## Gate

Standard agent tier + focused closed-root audit scenario. df-guard
first (`df -h /` — need >40G free; if lower, tell the orchestrator
before running VM work). Bounded. Synchronous receipts.
