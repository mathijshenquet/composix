# track/regen3 — wave-3 regenerated corpus cases (luna cold staging → assembly)

DO NOT LAUNCH until track/envgrammar and track/cip102 are merged: both
change migrate.md and corpus Cixfile canon; regenerating against the
older prompt would go stale on arrival.

Process mirrors regen1/regen2 exactly (read .dev/specs/track-regen2.md
and docs/corpus.md §"How this corpus is maintained") unless stated
below. Branch `track/regen3`, herdr worktree; LOG:
`corpus/migrate/LOG.md`. Four cold staging dirs (fresh luna agents,
each gets ONLY docs/migrate.md + the case Dockerfile + context):
`/home/mathijs/regen-stage/{caddy,parse-server,directus,watchtower}`.
Header: `Generated: migrate.md@<staging commit> · gpt-5.6-luna ·
<date>`. Two-layer warm/cold evidence rule applies unchanged.

## Per-case expectations (the point of this wave)

- **caddy** — regenerate with CONFIGDIR path freedom and the landed
  localhost skeleton: the minimal `/etc/hosts` `FILE` workaround and
  the CONFIGDIR deviation must be GONE from the fresh translation;
  verify the four-socket contract (incl. `udp:443`) still probes. The
  old GAPS links a now-removed open-questions anchor — the fresh GAPS
  simply drops it.
- **parse-server** — CONFIGDIR deviation gone. The cold node_modules
  read-set divergence may well persist: record honestly per the
  two-layer rule, do not re-pin to hide it.
- **directus** — the headline: CIP-95 imported glibc + the traced
  ENOTDIR fix should carry the build past both former walls. If an
  item builds, probe it and grade the remaining offline-deploy
  boundary honestly (its row is ⏳ pending exactly this). If a new
  wall appears, exact error + gap bullet, no forcing.
- **watchtower** — remains ❌ refused (Docker control plane); this
  regen only removes the stale duplicate-COPY workaround from the
  build side. Fidelity cell keeps the refusal loud.

## Ledger + close

Re-grade the four docs/corpus.md rows and any affected docs/docker.md
rows in the same track; GAPS `Status: current` (or honest stale-with-
reason); rerun `check.sh` receipts synchronously per case; regenerate
the corpus browser. Track gates per AGENTS.md (fmt / examples fmt /
clippy / workspace tests / tour drift / progressive-vm-check for what
the diff selects). Leave the branch clean; do not merge to main.
