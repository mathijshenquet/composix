# DRAFT — Kubernetes migration wave 1 (CIP-light)

Status: **draft for Mathijs's adoption; no cases launched**.

## Problem

The Kubernetes corpus has a candidate list but no executed case. The draft teaching contract now
has enough shape to test, but one easy success would not exercise its honesty boundaries. Wave 1
should cover a small Deployment, a finite scheduled workload, a multi-workload Service graph, and
a state/config/probe-heavy workload. It should measure which concepts dissolve cleanly and which
are product gaps; it must not reward YAML-shaped approximations.

## Proposal

Run four cases in this order after the teaching draft is adopted and CIP-109's accepted URL probe
grammar is implemented:

1. **Minimal nginx Deployment — baseline, S.** One container and one declared port make the item
   boundary obvious. It tests image-to-package/source disposition and the mandatory honest result
   for `replicas: 2`: one path-keyed instance can run, replica reconciliation cannot. This is the
   smallest check that the prompt does not confuse “the process runs” with “the Deployment was
   translated.”
2. **Canonical CronJob — timer-shaped positive case, S.** Translate the finite container to an
   `APP` and the schedule to systemd `OnCalendar=`. It should yield a strong behavioral receipt
   while recording cron-syntax conversion, concurrency policy, retry/history, and controller
   losses. This proves the prompt can dissolve a controller where systemd has the matching
   mechanism.
3. **Kubernetes guestbook — Service/topology case, M.** Frontend and Redis Deployments plus
   Services exercise multiple items, compose grouping, ports, environment, and client discovery.
   The expected verdict is partial: a one-instance localhost or explicit-address restructure may
   be proved, while replicas, Service DNS/VIPs, selector reconciliation, and load balancing remain
   gaps. The case must not put every workload in one pod merely to hide missing discovery.
4. **Bitnami PostgreSQL primary StatefulSet — state/config/probe case, M/L.** Pin the chart and
   values and translate a checked-in rendered singleton, not the Helm template. The case exercises
   durable data, ConfigMap files, secret-file delivery, named ports, startup/readiness/liveness
   intent, init work, and optional sidecars. Keep the render deliberately narrow. Expected verdict
   is partial: a singleton service and private state are plausible; StatefulSet identity,
   claim-template/provisioning behavior, exec probes, replica/ordered rollout, and Kubernetes
   secret/config projection are not.

The node-exporter DaemonSet and ingress-nginx controller stay out of wave 1. Both lead with
cluster/host-controller boundaries—automatic per-node placement, host namespaces, Kubernetes API
watch/RBAC, admission, and ingress reconciliation—before the ordinary application mappings have
an executed baseline. They are valuable refusal/partial cases for a later wave.

Every selected case must carry pinned inputs, `SOURCE`, per-service Cixfiles, `compose.json`,
`GAPS.md`, `receipt.md`, and a bounded `check.sh` with separate `k8s` and `cix` modes. A wave result
is an observed value from both modes or an explicitly labelled missing reference receipt. Build,
apply, activation, and detached output are not behavioral success by themselves.

## Language and product gaps surfaced by the prompt design

- **OCI image adoption is missing at this entry point.** A manifest usually provides an image ref,
  not its build/runtime contract. Today the translator needs a nixpkgs package, existing cix item,
  or separately pinned source/Dockerfile. Wave 1 should measure how often this blocks the manifest
  axis before proposing an image-import bridge.
- **Replicas and controllers remain deliberately out (D30/CIP-85).** Deployment, StatefulSet, and
  DaemonSet desired-count/reconciliation semantics have no field. Hand-duplicating compose
  children is not a substitute.
- **Kubernetes Service discovery and load balancing are absent.** Pod-local localhost and explicit
  host binds cover narrower shapes; named Service DNS, virtual IPs, selector/endpoints
  reconciliation, replica balancing, and named network policy remain D26/D27-era work.
- **ConfigMap projection has no deploy-time immutable object.** Checked-in config can enter an item
  and operator-owned directories can be host-bound, but key projection, atomic updates, watches,
  modes, and rollout coupling do not exist. PostgreSQL should establish whether this warrants a
  future config materialization surface or remains item repinning/operator policy.
- **Probe fidelity is intentionally narrower.** CIP-109 gives natural HTTP/TCP URL forms, but exec,
  gRPC, HTTPS/header variants, continuous readiness, independent startup probes, and Kubernetes's
  full delay/timeout/threshold matrix are not expressible. Record demand before widening CIP-79.
- **Volume lifecycle covers only a subset.** Private role dirs, retained same-host shared dirs, and
  explicit host binds exist. Shared ephemeral `emptyDir`, PVC provisioning/access modes/capacity,
  CSI, projected volumes, and multi-host RWX semantics do not.
- **Pod grouping currently means shared netns, not the whole Pod API.** Init-container sequencing,
  shared ephemeral filesystem semantics, PID/IPC options, service-account projection, and pod
  status remain separate gaps.

No new syntax is proposed in this draft. Adopt the teaching contract, execute the four cases, and
promote only repeated, behavior-blocking evidence into follow-up CIPs.
