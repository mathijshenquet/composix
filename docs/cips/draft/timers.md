# Scheduled work: the timer surface

Status: draft, amended 2026-08-01 after Mathijs's review ("yes good") —
ready to adopt. Mechanism was already decided (D48e: raw `OnCalendar`,
no translation layer); this doc is the *surface*.

## 1. The problem

Scheduled/batch work has no cix spelling: the Renovate CronJob row and
"every app's internal schedulers" (docs/corpus.md §4.4), plus the
migrate-Job class that D48(f) already dissolved into ordinary oneshot
units. systemd timers are sitting right there. What is missing is only
where the schedule is written and what the defaults are.

## 2. Prior work

**Docker has nothing** — the wild runs host cron against `docker run`,
or supercronic/ofelia scheduler *sidecars*. The absence is telling: a
platform without a native timer grows parasitic schedulers.

**Kubernetes CronJob**: `schedule` (cron syntax) plus a surprisingly
load-bearing option set learned from production: `concurrencyPolicy`
(Allow/Forbid/Replace — overlapping runs), `startingDeadlineSeconds`
(missed-window tolerance), `successfulJobsHistoryLimit`, `suspend`.
Cron-syntax ambiguity (timezones, @reboot) is a recurring pain source.

**systemd timers**: `OnCalendar=` (richer than cron, checkable with
`systemd-analyze calendar`), `Persistent=` (catch up on a missed window —
k8s startingDeadline, better), `RandomizedDelaySec=` (jitter),
`systemctl list-timers` (observability for free). Concurrency: a timer
firing while its service's job is still active coalesces — overlap
protection (k8s `Forbid`) is the *native default*; Allow/Replace would
need building. History is journald's job. D48(e) recorded the house
principle off exactly this case: use systemd transparently; timers = raw
OnCalendar.

**D48(f)**: migrate-on-upgrade = ordinary oneshot + `After=`/`Requires=`;
content-addressing re-runs it exactly when the app changes. So one-shot
*event*-driven jobs are already solved; this doc is only about
*time*-driven recurrence.

## 3. Recommendation

The schedule is **deploy-side, in compose**, not in the manifest — when
something runs is instance knowledge, like egress usage (D49a polarity)
and env. Surface: a per-service compose field
`schedule: "<OnCalendar expression>"`, valid only for oneshot-shaped
services; generation emits the paired `.timer` into the composite like
any other unit. `persistent:` and `jitter:` are explicit sibling fields
mapping to `Persistent=`/`RandomizedDelaySec=`, absent = systemd's
defaults — no cix-invented defaults: compose.json is machine-written, so
explicitness beats magic (review call; docs recommend
`persistent: true` for the Renovate class) — `cix up` activates timers with the target, `list-timers` is the
status surface. Raw OnCalendar syntax per D48(e), documented with a
pointer to `systemd-analyze calendar`; no cron-syntax translation, ever
(docker.md gets the migration row: cron expression → rewrite as
OnCalendar).

Concurrency: native coalescing only (= k8s Forbid). Replace/Allow,
history limits beyond journald, and `suspend` (that's `systemctl disable
--now` on the timer) are refused until a corpus case demands them.

Manifest-side there is no timer field: an artifact does not know its
schedule — *when* something runs is instance knowledge without
exception (reviewed: no pack-layer scheduling argument found; even an
inherently periodic pack like a cert renewer only knows it is
oneshot-shaped, not its cadence). A batch-shaped service is just a
service whose exec exits.

`cix run --schedule "<OnCalendar>"` exists from day one (cix run is
degenerate unary compose — same field, flag spelling). A `schedule:` on
a non-oneshot service is a hard `compose check` error.

## 4. Open questions

None — all resolved in review (explicit `persistent:`, run flag yes,
hard error yes).

## Changelog

- 2026-08-01: drafted; amended after review — no invented defaults
  (`persistent:`/`jitter:` explicit), `cix run --schedule` in, hard
  error on non-oneshot, pack-layer scheduling ruled out.
