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

## ⚖ Hard choice (Mathijs)

- **Schedule syntax**: raw systemd `OnCalendar=` strings (powerful, documented,
  `systemd-analyze calendar` validates them — no invented DSL, minimal-magic) vs
  cron syntax (docker/k8s muscle memory, but we'd own a translator forever).
  Recommendation: raw OnCalendar, validated at `cix compose check` via
  systemd-analyze; the migrate-prompt can teach `0 3 * * *` → `*-*-* 03:00:00` as a
  lesson instead of cix owning it.
- Defaults taken without ceremony (veto if wrong): `Persistent=false` (missed runs
  don't fire on boot; k8s CronJob's default-ish), no overlap (systemd's natural unit
  activeness = concurrencyPolicy Forbid), failure surfaces in status only.

## Scope & gate

cix-compose (schema/check/generation); scenario: a scheduled APP fires (short
OnCalendar in VM), writes into its state dir, does not overlap itself, survives
rollback. Renovate-shaped corpus pair as receipt when migrate reaches it.
