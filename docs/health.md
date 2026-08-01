# Health: liveness and readiness without the health graph

Status: proposal, 2026-08-01. Amends design.md D48(c); decision pending.

## 1. The problem

Composix runs each service as a systemd unit. Docker-shaped workloads
arrive with health checks (`HEALTHCHECK`, compose `healthcheck:`), and the
wild corpus says health is our most-demanded deferral: 10 of 18 surveyed
compose files use it, plus probes in essentially every k8s chart
(docs/corpus.md §4.1). D48(c) sketched "health = a probe plus explicit
consumers" and listed three consumers: restart policy, `cix up`
convergence, and *dependent ordering* (start B only once A is healthy —
the compose `depends_on: condition: service_healthy` graph).

The question: what shape does health take in the manifest and in the
generated units — and specifically, do we build that health-gated startup
graph at all?

## 2. Prior work

**Docker** runs the `HEALTHCHECK` command inside the container from the
daemon on an interval, producing one status bit (healthy/unhealthy). The
bit has remarkably few consumers: docker does not restart an unhealthy
container (third-party "autoheal" sidecars exist to fill that hole);
swarm removes unhealthy tasks from its VIP; the main single-host consumer
is `depends_on: condition: service_healthy` — a boot-ordering DAG. In
practice that graph is where compose stacks grow fragile: 6 of the 10
health-using corpus files exist to sequence startup.

**Kubernetes** splits health by consumer, and names each probe after its
consumer: *liveness* → restart the container; *readiness* → route or
withhold traffic (Ready=false removes the pod IP from EndpointSlices, so
load balancers stop sending new connections); *startup* → suppress the
other probes until first success. Two structural lessons: (a) there is
**no startup ordering between workloads at all** — pods start
concurrently, crash, and retry with backoff until their dependencies
answer; convergence replaces sequencing, and that model won; (b) probes
are executed by the **kubelet**, the node daemon — platform
infrastructure, not a per-pod sidecar. The only sidecar variant is the
service mesh, where envoy performs its *own* active health checking of
upstreams — i.e. the traffic-carrier probes its backends directly, the
same pull model HAProxy uses (`check` on backends).

**systemd** already has both k8s probes, under other names:

- *Readiness* is `Type=notify` + `sd_notify(READY=1)`: the start job does
  not complete until the service says ready, and `TimeoutStartSec=` bounds
  the wait. Anything ordered `After=` it (and any `systemctl start` caller)
  observes readiness for free. Readiness is a **one-way latch**: there is
  no "un-ready" notification.
- *Liveness* is the watchdog: with `WatchdogSec=N`, the service must send
  `WATCHDOG=1` periodically or pid 1 kills it and `Restart=`/`RestartSec`/
  `StartLimitBurst` apply — enforcement and backoff are native, and the
  watchdog only arms after startup completes.
- A *startup probe* therefore dissolves: `TimeoutStartSec` is the startup
  budget, and the watchdog not arming until READY is the k8s
  "suppress liveness during startup" semantics.
- For processes that cannot speak sd_notify, `ExecStartPost=` blocks the
  start job until it exits (a readiness gate without notify), and
  `NotifyAccess=all` lets any cgroup member feed the watchdog.

## 3. Recommendation

**Ban the health graph.** Compose never grows
`condition: service_healthy`; edges stay structural (`Requires=`/`After=`,
socket edges). Convergence is per-service: crash, restart with backoff,
retry — the k8s model, which our unix-socket edges improve on (socket
activation buffers the connect, so a consumer does not even crashloop).
Record as an explicit refusal in docs/docker.md.

**Adopt liveness/readiness as the vocabulary**, hung on systemd natively
(D48e):

- `READY notify` → `Type=notify`; `READY http|tcp|exec …` → the unit stays
  `Type=exec` and `ExecStartPost=cix probe await` blocks until the first
  probe success. Either way `cix up`'s restart-changed gets
  rollout-status semantics for free: start jobs complete on ready, time
  out on `TimeoutStartSec`.
- `LIVE notify [every]` → `WatchdogSec=`; `LIVE http|tcp|exec …` → the
  same `cix probe await` invocation forks a resident pinger into the
  cgroup which translates probe success into `WATCHDOG=1`
  (`NotifyAccess=all`, `WatchdogSec = every × failures`). Pid 1 enforces;
  no timer units, no restart privileges, no counter state outside systemd.
  Declaring `LIVE` *is* the restart opt-in (replaces D48c's report-only
  default).
- No startup probe: `TimeoutStartSec` covers it.
- The manifest `health {exec, interval}` field (v0) is replaced; docker
  `HEALTHCHECK` migrates to the same probe spelled as `READY` + `LIVE`.

**Defer continuous readiness** (post-startup un-ready, the k8s
traffic-gating half). systemd cannot represent it (one-way latch) and
single-host there is no consumer: no VIP exists, a not-ready backend
refuses connections and client retry converges. Docker is the cautionary
tale of collecting the bit without a consumer. When the publish proxy
(D49b proxyd) lands, prefer the **pull model**: the proxy actively probes
its backends (envoy/HAProxy precedent) rather than a per-service sidecar
pushing state over a unix socket — the probe then lives and dies with its
consumer, per D48(c) "health is an edge to a consumer", and no health bus
ever needs to exist.

Honest note: `NotifyAccess=all` means the workload itself could spoof
READY/WATCHDOG. This is k8s-equivalent (an app owns its own `/healthz`),
and it is visible in the generated unit per D48(e) transparency.

## 4. Open questions

1. Vocabulary: `READY` / `LIVE` as two consumer-named directives
   (proposed; matches `EXEC`-style brevity), vs one `PROBE` with roles,
   vs long forms `READINESS`/`LIVENESS`.
2. Parameter canon: proposed minimum — `READY` takes only a budget
   (→ `TimeoutStartSec`); `LIVE` takes `EVERY <dur>` (default 10s) and
   `FAILURES <n>` (default 3). k8s `initialDelaySeconds` dissolves
   (watchdog arms after ready). Enough?
3. Confirm the graph ban as an explicit ❌ row in docs/docker.md
   (`depends_on: condition: service_healthy` — refused, convergence
   instead), shifting that row from 🔁.
4. Pull vs push for continuous readiness at proxyd time: fix the pull
   preference now in the D-number, or leave it open until the networking
   era?
