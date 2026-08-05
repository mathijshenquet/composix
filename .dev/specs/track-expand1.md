# track/expand1 — expansion wave 1 assembly (homer, it-tools, mailpit)

Process per the regen3/regen4 assembly pattern (docs/corpus.md
maintenance rules), but these are NEW cases: create
`corpus/migrate/docker/{homer,it-tools,mailpit}/` from the staging
outputs at `/home/mathijs/regen-stage/new-{homer,it-tools,mailpit}`
(Cixfile(+locks), Dockerfile, SOURCE, check.sh, GAPS.md authored from
NOTES, receipt.md from your independent re-verification — worker
greens are claims). Context dirs stay gitignored; extend
`corpus/migrate/fetch.sh`'s coverage so the contexts are refetchable
from SOURCE. Add the three rows to docs/corpus.md's living table (new
ribbon vocabulary; count 21 -> 24), mark them consumed in
CANDIDATES.md, regenerate the browser.

Per-case specifics:
- **homer** — honest pnpm-registry wall. Do not force; verify the wall
  reproduces, then write `cips/draft/pnpm-wall.md` (CIP-light): the
  npm/pnpm ecosystem-fetch wall now has FIVE exhibits (homer staging,
  dozzle fetch-hang, verdaccio cold volatility, directus offline
  metadata, filestash 69k-file seal as the adjacent lock-scale face) —
  one design round beats five case bullets. Cite each exhibit
  precisely.
- **it-tools** — assemble per its receipts; harvest its FRICTION
  (bash-import stumble = third datapoint, add to
  draft/nodes-and-edges.md evidence).
- **mailpit** — its ENOSPC wall was environmental (host disk hit 100%
  mid-build; since recovered to 74%). Re-verify the build in this
  clean state; grade on the re-verified outcome.

Also: append a dated amendment line to
`cips/accepted/0101-tmp-relocate.md`'s changelog and implement the
small tweak: the startup orphan sweep's one-day threshold left 82
multi-GB interrupted-build trees on /var/tmp today (root hit 100%) —
lower the sweep age (hours, not a day) and/or make it size-aware;
interrupted/killed builds are the leak path cleanup-on-exit cannot
cover. Keep the change minimal and receipted.

Discipline: branch `track/expand1`, LOG `corpus/migrate/LOG.md`; full
agent gate tier (the selector will price your diff honestly),
capture-as-epilogue value-checked receipts; bounded VM parallelism;
df-guard before big builds. Merge semantically if main moves. Clean
branch; do not merge.
