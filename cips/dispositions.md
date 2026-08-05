# Ledger dispositions

Blessed verdicts on `docs/docker.md` ledger rows that needed a decision
but not a CIP. Batches are proposed in open-questions.md, blessed by
Mathijs, recorded here permanently, and then applied to the ledger as a
mechanical track. Each entry keeps enough context to stand alone (the
standing self-containedness rule); the ledger rows cite the verdicts,
this file is where they were decided.

Markers as in docker.md: ❌ refused (recorded, with why), ⏳ accepted
but unbuilt (waiting for a consumer), 🔁 different mechanism.

## Batch 2026-08-04 (blessed by Mathijs; docker.md application queued)

Two items of the proposed batch were not blessed as proposed and became
drafts instead: the `ENV NAME=value` grammar question →
[draft/env-equals.md](draft/env-equals.md) (proposal inverted: switch
to the `=` grammar rather than keep ours), and `ARG`/build args →
[draft/build-args.md](draft/build-args.md) (lock-pinned ARG rather than
the proposed no-mechanism re-mark). The rest were blessed as follows:

- **`docker cp`** (copy files in/out of a running container) → ❌.
  A cix service's writable state lives in role directories, which are
  ordinary host paths you can reach with `cp`; `cix inspect` prints
  where they are. There is nothing to tunnel through a daemon.
- **`--name` (stable container handle)** → ⏳. Compose members already
  have stable names; a `cix run --name` for one-off runs is mechanical
  sugar on the existing run path — build it when someone actually asks.
- **`STOPSIGNAL` and stop timeouts** (which signal stops the process,
  how long to wait before SIGKILL) → ⏳, small mechanical track: these
  map one-to-one onto systemd's `KillSignal=` and `TimeoutStopSec=`
  unit fields; no design needed. Two corpus cases (adminer, nginx)
  note the upstream signal contract as a gap, so it has a real
  consumer.
- **Namespace sharing (`--ipc`, `--pid`, `--uts` between containers)**
  → pods are the answer; standalone flags stay ❌. The compose tree
  already realizes shared network namespaces for a subtree ("pods",
  CIP-86); sharing IPC/PID follows the same pod mechanism (systemd's
  `JoinsNamespaceOf=`) if a case ever needs it, and per-pair ad-hoc
  sharing flags are refused.
- **Restart policy knobs** (`--restart=always` etc.) → covered, tuning
  later. LIVENESS (CIP-79) is the deliberate restart opt-in with a
  fixed bounded policy; making the interval/burst configurable is
  compose-mechanical follow-up when a case demands it.
- **`docker init`** (generates a starter Dockerfile) → ⏳: the migrate
  teaching prompt is our generator today; a `cix init` skeleton
  belongs to a later tooling era.
- **Docker Offload** (paid remote builds) → ❌: nix remote builders
  are the native answer.
- **AppArmor/SELinux label options** → out of manifest scope — that is
  host security policy; revisit only if a real SELinux-host user
  appears.
- **Docker Desktop "Enhanced Container Isolation"** → ❌: a
  desktop-product threat model we do not share.
- **Authorization plugins** (pluggable allow/deny on engine calls) →
  no plugin interface ever; policy questions return in the
  server/reconciler era (the decided deferral of a long-running cix
  daemon — docs/design.md D9).
- **Engine API / SDKs** → same reconciler-era deferral as above.
- **Remote contexts / `DOCKER_HOST`** → ssh is the transport; any
  sugar is ⏳.
- **`docker mcp`** → ❌: unrelated to the runtime thesis.
- **Linux capabilities beyond NET_BIND_SERVICE** → grow the `CLAIM`
  vocabulary case-by-case as dogfood demands (CIP-78 added gpu/device
  this way); never a raw `--cap-add` passthrough.
