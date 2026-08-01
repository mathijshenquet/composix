# track/timers — CIP-75: compose `schedule:` on raw OnCalendar

Read AGENTS.md first. Authoritative: docs/cips/0075-timers.md
(§3 Recommendation + §5 Decision). This spec supersedes the earlier
design-position version of this file (pre-CIP-75). Work in
`.worktrees/timers` on branch `track/timers`. Keep
`crates/cix-compose/LOG.md` current.

1. Compose schema: per-member fields `schedule: "<OnCalendar>"` plus
   optional explicit `persistent: bool` and `jitter: "<dur>"` — absent
   fields mean systemd's own defaults, cix invents NO defaults.
   `schedule:` is valid ONLY on kind-app (run-to-completion) members; on
   a service member it is a hard `cix compose check` error.
2. Generation: a `schedule:`d member gets a paired
   `cix-<comp>-<svc>.timer` in the composite (same store-item + profile
   flow as other units): `OnCalendar=` verbatim, `Persistent=` /
   `RandomizedDelaySec=` only when the fields are present. The .timer is
   enabled/activated with the composite target by `cix up`; `cix down`
   stops it; the app unit itself is NOT wanted by the target (the timer
   triggers it). Manual `cix run` of the same member keeps working —
   same unit, same context as scheduled runs.
3. Validation: empty/whitespace schedule is a check error; expression
   validity is checked via `systemd-analyze calendar` at
   `cix compose check` time (systemd is the authority — no cix parser);
   if systemd-analyze is unavailable, skip with a stderr note and let
   activation surface the error. Docs point at `systemd-analyze
   calendar` for authoring.
4. `cix run --schedule "<OnCalendar>"` per CIP-77: mechanical flag, same
   semantics for a transient run — pick the systemd-native route
   (transient timer properties vs generated pair) and record the choice
   in the LOG.
5. Docs: compose docs section (schedule/persistent/jitter, the
   `systemctl list-timers` observability line); docs/docker.md +
   docs/migrate.md: cron/CronJob migration rows → rewrite as OnCalendar
   (no cron translation ever).
6. Tests: generation snapshots (with/without persistent+jitter); check
   errors (service-kind schedule, empty schedule, invalid expression);
   timer-enabled-in-target assertion; extend a compose VM scenario
   minimally — a scheduled member's .timer is active and
   `systemctl list-timers` shows it (do NOT wait for a real firing).

Gate: fmt / `cix fmt --check examples` / warning-denied clippy /
workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Exact repros in the LOG. Commit on
this branch when green.
