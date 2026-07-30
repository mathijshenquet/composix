# track/health — health wiring for compose (corpus demand #1)

STATUS: design-position spec. Hard choices pending Mathijs (marked ⚖); do not launch
until D-numbered. Corpus evidence: 10/18 wild compose files use healthchecks, 6 gate
startup on `condition: service_healthy`; probes are in essentially every k8s chart.
The manifest has carried `health` (exec + interval) since v1 — what is missing is
compose *semantics* for it.

## Design position

Two mechanisms, both compiled from the existing manifest `health` field:

1. **Readiness (startup gating)** — a service with `health` is not "started" until
   its first probe passes: generate `ExecStartPost=` running the probe in a bounded
   retry loop (timeout → unit fails startup). Dependents keep plain `After=` — no new
   compose surface, `depends_on: service_healthy` semantics fall out of systemd
   ordering. `cix up` therefore blocks on health, and a failed generation switch
   surfaces at up-time, not discovery-time.
2. **Liveness (periodic)** — a generated `cix-<path>-<svc>-health.timer` + `.service`
   running the probe on the manifest interval; consecutive-failure threshold N →
   action.

## ⚖ Hard choices (Mathijs)

- **Unhealthy action**: docker restarts; k8s separates liveness (restart) from
  readiness (traffic gating). We have no traffic layer, so the honest menu is
  (a) report-only (status surfaces it, `cix ps`-era), (b) restart after N failures
  (docker-shaped), (c) per-service policy field defaulting to report-only.
  Recommendation: (c) with default report-only — restarts that mask crash-loops are
  docker's least honest habit.
- **Does readiness gate the listener fd handoff?** A socket-activated service gets
  connections before first health pass. Holding the socket until healthy = truthful
  readiness but adds machinery; not holding = documented gap. Recommendation: v0
  documents the gap (activation IS the readiness signal for fd-tier services).

## Scope & gate

crates/cix-run (unit generation) + cix-compose; scenario tier gets a lifecycle
assertion (healthy gating observable; unhealthy → policy action). Full workspace +
scenario gates.
