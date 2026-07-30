# track/timers — scheduled APPs (corpus demand #4)

STATUS: design-position spec. One hard choice pending Mathijs (⚖); depends on D47's
APP block landing (track/blocks). Corpus evidence: Renovate CronJob row; every
self-hosted app's "run this nightly" need. systemd timers are sitting in the
substrate unused.

## Design position

Compose child field on an APP ref: `"schedule": "<OnCalendar expression>"` →
generated `<unit>.timer` + run-to-completion service (the APP's kind=app semantics
from D47). Timer lifecycle follows the composite (up/down/rollback like any unit).
`cix run <comp>/<app>` still works for manual runs — same unit, same context (netns,
edges) as the scheduled runs.

## Resolved (D48e — Mathijs's round)

- **Raw `OnCalendar`**, validated at `cix compose check` via `systemd-analyze
  calendar`; no cron DSL — the migrate-prompt teaches the translation as a lesson.
  House principle recorded in D48e: *use systemd as transparently as possible; build
  only at a real impedance mismatch.*
- Defaults stand: `Persistent=false`, no overlap (unit activeness), failure surfaces
  in status only.

## Scope & gate

cix-compose (schema/check/generation); scenario: a scheduled APP fires (short
OnCalendar in VM), writes into its state dir, does not overlap itself, survives
rollback. Renovate-shaped corpus pair as receipt when migrate reaches it.
