# Health: liveness and readiness without the health graph

Status: **CIP-79, adopted 2026-08-01** (Mathijs: "HEALTH gewoon
akkoord"; READINESS/LIVENESS blessed). Amends design.md D48(c).
Decision in §5.

## 1. The problem

Composix runs each service as a systemd unit. Docker-shaped workloads
arrive with health checks (`HEALTHCHECK`, compose `healthcheck:`), and the
wild corpus says health is our most-demanded deferral: 10 of 18 surveyed
compose files use it, plus probes in essentially every k8s chart
(docs/corpus.md §4.1). D48(c) sketched "health = a probe plus explicit
consumers" and listed three consumers: restart policy, `cix up`
convergence, and *dependent ordering* (the compose
`depends_on: condition: service_healthy` graph). The question: what shape
does health take — and do we build that health-gated startup graph at all?

## 2. Prior work

**Docker** runs the `HEALTHCHECK` command inside the container from the
daemon on an interval, producing one status bit. The bit has remarkably
few consumers: docker does not restart an unhealthy container
(third-party "autoheal" sidecars fill the hole); the main single-host
consumer is `depends_on: condition: service_healthy` — a boot-ordering
DAG, and the place compose stacks grow fragile. In practice the check
command is nearly always `curl -f http://localhost/...` — an exec shape
used to spell an HTTP probe, because exec is the only probe type docker
has.

**Kubernetes** splits health by consumer and names each probe after its
consumer: *liveness* → restart; *readiness* → traffic (Ready=false pulls
the pod from EndpointSlices); *startup* → suppress the others until first
success. Probe types are `httpGet`, `tcpSocket`, `exec`, `grpc` — with
http/tcp carrying the overwhelming majority and exec as escape hatch.
Structural lessons: (a) **no startup ordering between workloads at all**
— pods start concurrently and converge by crash-retry; that model won.
(b) *Within* a pod there IS gated sequencing (init containers, and
native sidecars whose startup probes gate later containers) — ordering
exists exactly where colocation makes it cheap. (c) Probes run from the
**kubelet**, platform infrastructure, not a per-pod sidecar.

**systemd** already has both probes under other names. *Readiness* is
`Type=notify` + `READY=1`: the start job completes only on ready,
`TimeoutStartSec=` bounds it — and because `After=` waits for start-job
completion, **ordering follows readiness automatically wherever a
structural edge exists**. Readiness is a one-way latch (no "un-ready").
*Liveness* is the watchdog: `WatchdogSec=N` + periodic `WATCHDOG=1` or
pid 1 kills the service and `Restart=`/`StartLimitBurst` apply. The
watchdog arms only after startup completes, so a *startup probe*
dissolves into `TimeoutStartSec`. For processes that cannot speak
sd_notify: `ExecStartPost=` blocks the start job until it exits, and
`NotifyAccess=all` lets any cgroup member feed the watchdog.

## 3. Recommendation

**Ban the health graph.** Compose never grows
`condition: service_healthy`; edges stay structural. This does NOT mean
dependents start blind: a structural edge (`After=` via unix edges etc.)
automatically waits for the dependency's *readiness*, because readiness
is start-job completion — systemd already has this right. What is banned
is a separate health-condition vocabulary on top of edges. Between
unrelated services: crash-retry convergence, the k8s model, improved by
socket activation (the connect buffers; consumers do not even crashloop).
Recorded as an explicit ❌ in docs/docker.md.

**Two directives, consumer-named** (spelling pending §4.1; using the
proposed spelling here):

- `READINESS http :8080/healthz IN 90s` — probe adapter path: unit stays
  `Type=exec`; `ExecStartPost=cix probe await` blocks until first
  success; `IN` → `TimeoutStartSec=`. `READINESS notify IN 90s` — the
  app-native path: `Type=notify`. Either way `cix up` gets
  rollout-status semantics for free.
- `LIVENESS http :8080/livez EVERY 10s` — the same `cix probe`
  invocation leaves a resident pinger in the cgroup translating probe
  success into `WATCHDOG=1` (`NotifyAccess=all`); `LIVENESS notify
  EVERY 10s` → plain `WatchdogSec=`. Declaring liveness IS the restart
  opt-in (replaces D48c's report-only default). The watchdog window is
  fixed at 3× `EVERY`; there is no `FAILURES` knob — systemd speaks only
  the window (k8s's `failureThreshold` is not replicated; YAGNI per
  review).

**Probe types: `http`, `tcp`, `notify` only.** No `exec` probe in v0
(YAGNI per review): docker's exec-shaped checks are nearly always curl
spelling an HTTP probe — migration rewrites them to `http`; native
http/tcp probing lives in cix's own prober (no curl in the item's
closure, no shell). `exec` returns if a corpus case genuinely needs it
(redis-cli-ping-class), as k8s-style escape hatch.

No startup probe: `IN` covers it. The manifest `health {exec, interval}`
v0 field is replaced (D72: schema moves freely).

**Continuous readiness** (post-startup un-ready) stays deferred until the
publish proxy (D49b): **pull model decided** — the proxy actively probes
its backends (envoy/HAProxy precedent); no health bus. "Until we get bit
by it" — recorded.

Honest note: `NotifyAccess=all` lets the workload spoof READY/WATCHDOG —
k8s-equivalent (an app owns its own `/healthz`), visible in the unit per
D48(e).

## 4. Open questions

1. **Vocabulary bikeshed** (the one open item): proposed
   `READINESS`/`LIVENESS` — the exact industry terms k8s standardized
   (note: *liveness*, not *liveliness*), maximally greppable, and
   Cixfile directives are words already. Alternatives considered:
   `READY`/`LIVE` (terse but cryptic per review),
   `READYCHECK`/`LIVECHECK` (the CHECK suffix restates what a probe is).
2. (spun off) The `EXEC` naming bikeshed raised in review is a separate
   inventory item — prior art: docker `ENTRYPOINT`/`CMD` (shell-form
   footguns), k8s `command`/`args`, compose `command`, systemd
   `ExecStart=`, Procfile. `EXEC` matches exec(2)/`ExecStart=` and is
   honest about no-shell semantics (D55); recommendation there: keep.

## 5. Decision

Vocabulary: **`READINESS`/`LIVENESS`** (§4.1 as proposed). The
`LIVELINESS` near-miss (and every directive typo) is covered by the
parser's existing did-you-mean fuzzy suggestions (the crunchy round) —
verify `LIVELINESS → did you mean LIVENESS` lands as a suggestion
fixture when the directives are implemented. Everything else as
recommended: health graph banned (docker.md ❌ for
`condition: service_healthy`; ordering-follows-readiness via structural
edges), probe types `http`/`tcp`/`notify` only, `IN`/`EVERY` params,
watchdog window 3×EVERY, no FAILURES, LIVENESS = restart opt-in, `IN` =
startup budget, pull model at proxyd time. The `EXEC` naming bikeshed
raised during review is spun off to its own draft CIP
(draft/exec-naming.md).

## Changelog

- 2026-08-01: drafted; amended after review — FAILURES dropped, exec
  probe dropped (YAGNI), IN/EVERY spelling, ordering-follows-readiness
  clarified (§3), pull model for continuous readiness decided,
  vocabulary narrowed to §4.1. Adopted same day as CIP-79.
- 2026-08-02: implemented READINESS/LIVENESS probes, rollout gating, bounded watchdog restarts, and structural readiness ordering.
