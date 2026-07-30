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

## Resolved (D48c — Mathijs's round)

- **Health is an edge to a consumer, not a property.** k8s names probes by consumer
  (liveness→restart, readiness→traffic, startup→boot-gate) and keeps deep/business
  health at the application layer — we adopt both stances. Our consumers: `cix up`
  convergence, restart policy, dependent ordering (no traffic layer, so readiness ≈
  ordering). Manifest declares the probe; compose declares which consumers use it.
- **Unhealthy action**: per-service policy field, DEFAULT report-only (restarts that
  mask crash-loops are docker's least honest habit); restart-after-N is opt-in.
- **Readiness does not gate the listener fd handoff in v0**: for fd-tier services,
  activation IS the readiness signal; the gap is documented, not machinerized.

## Scope & gate

crates/cix-run (unit generation) + cix-compose; scenario tier gets a lifecycle
assertion (healthy gating observable; unhealthy → policy action). Full workspace +
scenario gates.
