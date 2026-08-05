# DRAFT — Converting Kubernetes manifests to composix

This is a draft teaching prompt for translating concrete Kubernetes manifests into per-service
Cixfiles and one `compose.json`. It is not a Kubernetes YAML importer and it is not a claim of
Kubernetes compatibility. The goal is behavioral fidelity on one composix host: preserve what
runs, how processes communicate, what configuration they read, what data survives, and what
health signals mean. Discard control-plane mechanisms that Nix or systemd already supplies, and
record every remaining mismatch.

The probe examples below target accepted CIP-109. Its URL-shaped grammar must be present in the
compiler before this draft is used to launch a corpus wave; do not temporarily translate them back
to the superseded two-token probe form.

Read every object in the application before writing output. Follow selectors from Services to pod
templates, keys from environment entries and projected volumes to ConfigMaps and Secrets, and
volume mounts back to their volume sources. For Helm or another generator, pin the chart and
values, render it first, and translate the checked-in rendered result. A template without its
values is missing context; do not guess a branch of it.

Account for every workload, container, init container, sidecar, port, probe, environment source,
volume, identity, policy, and controller field before calling the conversion complete. Each one
must be translated, explicitly dissolved with a Nix/systemd reason, deliberately restructured
with the behavioral difference named, or recorded in `GAPS.md`. Silence is the only forbidden
disposition.

There are four honest outcomes for a Kubernetes mechanism:

1. **Translate it.** Use an existing Cixfile or compose surface with matching behavior.
2. **Dissolve it.** Remove image, init, or controller ceremony that Nix or systemd makes
   unnecessary, and state why the observable behavior remains.
3. **Restructure it.** Use a different explicit topology only when the important behavior can be
   proved; record the difference instead of calling it exact.
4. **Report a gap.** Write `not expressible — record as gap` when composix lacks the behavior. Do
   not invent fields, sidecars, shell loops, or host services to imitate a control plane.

## The output model

Translate each distinct long-running container contract to a `SERVICE` item and each finite job to
an `APP` item. Keep each service's Cixfile and required source/configuration files together. A
multi-container pod becomes multiple items; compose, not a supervisor inside one item, groups
them.

The root `compose.json` is a strict `cixCompose: 1` tree. Every leaf points at an explicit tagged
item ref. An inline group with `network: "pod"` is the closest existing match for a Kubernetes
Pod: its descendants share one network namespace and localhost. The path through the compose tree
is the instance identity. It is not a replica controller, scheduler, or Kubernetes namespace.

Use the pod boundary even for a one-container pod when its network isolation matters. Omitting
`network: "pod"` gives host networking; it is not a harmless shorthand for a Kubernetes pod.
Kubernetes pod sharing beyond the network namespace—PID, IPC, hostname, process lifecycle, and
arbitrary shared ephemeral volumes—is not implied.

A host-facing, socket-binding service can have this shape:

```json
{
  "cixCompose": 1,
  "name": "example",
  "children": {
    "web-pod": {
      "network": "pod",
      "children": {
        "web": {"item": "example-web:v1"}
      },
      "publish": {
        "http": {"child": "web", "port": "http"}
      },
      "bind": {
        "http": "127.0.0.1:8080"
      }
    }
  }
}
```

`publish` lifts one declared surface across one pod boundary; `bind` chooses the host address.
Neither field creates Kubernetes Service discovery or load balancing.

Every container image reference needs a separate disposition. Composix does not consume an OCI
image as a Cixfile base. Prefer a package from the bound nixpkgs universe, an existing cix item, or
a source-built Cixfile whose source revision and build inputs are pinned. A manifest that supplies
only `image:` often omits the filesystem, default entrypoint, and build provenance needed for that
work. Say `missing image build/runtime context — record as gap`; do not infer it from an image name
or re-create an unknown root filesystem.

## A migration workflow

### 1. Freeze and inventory the real input

Store the exact manifests used by the case. Record their upstream URL and revision in `SOURCE`.
For Helm, also record the chart revision, values, render command, and rendered output. Never
translate a moving `main` URL or an unrendered conditional template as though it were a receipt.

Build an inventory before translating:

- workload objects and the pod templates they own;
- every normal, init, and sidecar container, including `command` plus `args`;
- every image and the source or package that can replace it;
- environment literals and every `valueFrom` or `envFrom` source;
- named and numeric container ports and every Service `port`/`targetPort` relation;
- readiness, liveness, and startup probes with all timing fields;
- volumes, mounts, `subPath`, read-only flags, and ownership expectations; and
- replicas, rollout policy, placement, resources, security context, service accounts, RBAC,
  namespace use, and host namespace requests.

Resolve selectors on paper. A Service selector that matches no translated workload, or matches
several replica sets that composix cannot realize, is not a connection to approximate silently.

### 2. Turn each container contract into an item

Use the [Docker migration contract](migrate.md) for the Cixfile build graph. The same rules apply:
bind one package universe, import whole packages, keep network access in `FETCH`, keep `RUN`
offline, invoke imported commands by bare name, and assemble only declared runtime files.

Kubernetes `command` replaces the image entrypoint and `args` replaces its default arguments. If
both are concrete and the executable is present in the assembled item, combine them into one
shell-free `START` argv. If either relies on image defaults that are unavailable, the start
contract is missing. A string such as `sh -c ...` is shell behavior only when the manifest
explicitly invokes a shell; import that shell and preserve the argv rather than treating every
command as shell text.

Map literal application environment to `ENV NAME=value` when it belongs to the artifact contract.
Use `ENV NAME required` for an operator-supplied non-secret value and put the concrete value in the
compose child's `env` map. Use a bare optional `ENV NAME` only when absence is valid. Expand
`envFrom` and key references explicitly so the output remains reviewable; composix has no ambient
ConfigMap namespace.

`imagePullPolicy`, registry lookup, and mutable image tags dissolve into cix refs plus locks. Every
compose `item` ref still has an explicit tag, and `cix.lock` records the resolved store path. Moving
a tag takes effect only through the selected `pin`/`track` policy and a deliberate activation.

Do not merge multiple pod containers into one `SERVICE`. Sidecars remain separate services inside
one `network: "pod"` group when their contracts are independently expressible. An init container
may dissolve into immutable build assembly or role-directory creation. A same-item, idempotent
pre-start action may become `START_PRE`; a separate init image, ordered pod lifecycle, or shared
mutation that does not fit those rules is **not expressible — record as gap**.

### 3. Translate ConfigMaps and environment sources

Classify configuration by ownership and update behavior before choosing a surface:

| Kubernetes use | Cix treatment | Required honesty |
| --- | --- | --- |
| ConfigMap literal used as application env | Artifact default via `ENV NAME=value`, or `ENV NAME required` plus compose `env` | Expand individual keys. There is no ConfigMap object or live projection. |
| ConfigMap files that are versioned with the deployment | Keep real files beside the Cixfile and `COPY` them into the service item | A config edit builds and repins a new immutable item; it is not a live in-place update. |
| Operator-owned configuration directory | Declare `DIR /app/config:ro`; map it with compose `dirs: {"/app/config": {"host": "/operator/path"}}` and a stable `identity` | The host directory must already exist. Composix does not create it, populate keys, or implement ConfigMap rollout. |
| `configMapKeyRef` / `envFrom` | Resolve and disposition each key as an artifact default or compose override | Missing/optional-key behavior and bulk namespace updates are not preserved automatically. |
| `fieldRef` / resource-field injection | Use a literal only when the value is truly static in the one-host deployment | Pod IP, node name, namespace, resource-derived values, and other downward API data are **not expressible — record as gap**. |

`CONFIGDIR` means a private, writable, systemd-managed configuration role. It does not mean a
read-only ConfigMap projection. Do not select it merely because a Kubernetes volume is named
"config".

Secret values never become Cixfile `ENV` defaults or compose `env` strings. When the program can
read a file, declare `SECRET <name> AS <ENV_FILE>` and supply the compose secret from a host file or
supported encrypted source. Kubernetes secret-as-environment behavior requires changing the
application to file delivery; if that is not possible, it is **not expressible — record as gap**.
Service-account tokens, projected token rotation, and Kubernetes API credentials are not generic
application secrets and must not be replaced with an egress claim.

### 4. Translate volumes by lifecycle, not by object name

Keep the path the application already uses. A role directory declares the path the process
touches; it does not reproduce a Kubernetes storage API.

| Kubernetes volume intent | Cix treatment | Important limit |
| --- | --- | --- |
| Private durable application data | `STATEDIR /path` | Cix-managed and service-private on one host. PVC provisioning, capacity, storage classes, access modes, snapshots, and attachment are absent. |
| Private disposable runtime files, pid files, or sockets | `RUNDIR /path` | Match the observed lifecycle; do not claim general `emptyDir` equivalence without restart/stop evidence. |
| Private disposable cache | `CACHEDIR /path` | Cache may survive service restarts; use only when that is acceptable. |
| Service-written logs | Prefer stdout/stderr; otherwise `LOGDIR /path` | Kubernetes logging agents and logging backends do not translate. |
| Pre-existing operator storage | `DIR /path[:ro|:rw]` plus compose `host:` and a stable `identity` | The host path must exist; Kubernetes `hostPath.type` creation/checking and node placement are absent. |
| Same-host retained data shared by services | Each item declares `DIR` or a compatible role; compose maps the paths with one `shared` name | It is retained compose-local storage, not RWX storage and not an ephemeral shared `emptyDir`. It constrains all users to one host. |
| ConfigMap or Secret projection | Immutable `COPY`, read-only host config, or `SECRET` file delivery as described above | Atomic key projection, update watches, per-key modes, and projected-source merging are absent. |

An ordinary private `emptyDir` may dissolve into an existing runtime/cache role only when its
actual lifecycle matches. Arbitrary shared ephemeral directories, memory-backed `emptyDir` with
size limits, CSI volumes, device plugins, projected/downward-API volumes, mount propagation, and
generic block devices are **not expressible — record as gap**. `subPath` is only dissolved when
immutable assembly or an explicit directory mapping proves the same visible tree; otherwise
record it.

Read-only `/proc`, `/sys`, or host-root observation is still a host capability, not normal app
data. A narrow pre-existing host directory can use `DIR ...:ro` plus `host:`, but host PID/IPC,
mount propagation, automatic one-per-node placement, privileged setup, or mutation of those trees
does not follow. Never replace those requirements with `CLAIM egress` or a broad `/` bind.

### 5. Translate probes by their consumer

Kubernetes already separates probes by consumer, which aligns with Cixfile
`READINESS`/`LIVENESS`. Use CIP-109's canonical URL-shaped form:

```dockerfile
PORT http = 8080
READINESS http://127.0.0.1:8080/ready IN 60s
LIVENESS http://127.0.0.1:8080/live EVERY 10s

PORT database = 5432
READINESS tcp://127.0.0.1:5432 IN 60s
```

When a service declares exactly one `PORT`, an HTTP probe of that same port may use path-only
sugar such as `READINESS /ready IN 60s`. Prefer the full URL in migration output when it makes a
named-port resolution or non-default port auditable. `notify` is valid only when the application
natively sends systemd readiness/watchdog notifications; do not infer it from a Kubernetes probe.

Map an `httpGet` host, numeric or named port, and path to one `http://` URL. Map `tcpSocket` to
`tcp://`. `READINESS ... IN ...` is a bounded startup gate and becomes a one-way ready latch;
composix does not continuously remove an active service from a Kubernetes-style endpoint set.
`LIVENESS ... EVERY ...` opts into watchdog restart with a fixed three-miss window.

The timing vocabularies are not isomorphic. Kubernetes has per-attempt timeout, initial delay,
period, success threshold, and failure threshold; readiness `IN` is one total startup budget, and
liveness exposes only its interval with the fixed watchdog window. Choose a bounded value that
matches the intended startup/runtime contract, explain the disposition, and record any material
timing loss. Do not pretend that copying one integer preserves all fields.

A startup probe may dissolve into `READINESS ... IN ...` only when it is the same startup gate and
no independent post-start behavior is lost. Exec, gRPC, HTTPS/client-auth, custom HTTP headers,
multiple success thresholds, and any probe whose semantics cannot become native HTTP/TCP/notify
are **not expressible — record as gap**. Never ship curl, a shell, or a probe sidecar merely to make
the YAML shape survive.

### 6. Translate Services to declared surfaces

Treat `containerPort` and Service objects separately. A container port describes the workload's
inbound socket; a Service selects workloads and gives clients a stable virtual endpoint.

- If the application binds its own TCP or UDP socket, declare a named Cixfile `PORT`. Preserve the
  Service's `targetPort` number or resolve its named target explicitly.
- Use `LISTENER` only when the process really accepts a systemd-activated socket. A Kubernetes
  Service does not make an ordinary application socket-activated.
- For a host-facing TCP surface, compose `bind` chooses the host address. Inside a pod group,
  `publish` lifts the named child port or listener across the boundary before it is bound.
- For a single-instance, same-trust-boundary group, an internal Service may be deliberately
  restructured to shared localhost and an explicit client address. Record that the Service VIP,
  selector, DNS name, and load-balancing behavior dissolved; prove the rewritten client path.

Independent Kubernetes Services do not have a faithful general mapping today. ClusterIP/virtual
IPs, service DNS, endpoint reconciliation, load balancing across replicas, headless discovery,
ExternalName, session affinity, topology-aware routing, NodePort allocation, LoadBalancer
controllers, ingress integration, and named multi-network policy are **not expressible — record as
gap**. Pod-local localhost is not a substitute when it changes isolation, replica selection, or
failure behavior. Compose pod publication is TCP-only; a UDP Service that must cross the pod or
host boundary remains a gap even though a leaf can declare a UDP `PORT`.

Outbound access is separate from every Service declaration. Add `CLAIM egress` only when the
application initiates external connections. Under a pod ancestor composix can enforce it; without
one it is a loud host-network no-op. Egress does not grant Kubernetes API access, service DNS, or
host socket access.

### 7. Dispose controller and pod policy explicitly

Use this table as a floor, not as permission to ignore unlisted fields:

| Kubernetes concept | Disposition |
| --- | --- |
| Deployment/ReplicaSet with `replicas: 1` | One path-keyed compose instance can preserve the process contract. The controller, self-healing object status, rollout strategy, and revision history still do not exist. |
| `replicas: 0` or `replicas > 1`, autoscaling | **Not expressible — record as gap.** D30 deliberately leaves scale/replicas out; never copy-paste children and call that reconciliation. |
| StatefulSet | A singleton's private state may map to `STATEDIR`. Ordinals, stable network identity, volume-claim templates, ordered rollout, and replica semantics are gaps. |
| DaemonSet | **Not expressible — record as gap.** One explicit service on one host is not automatic one-per-node placement or reconciliation. |
| Job | A finite command maps to `APP`; verify exit status and output. Backoff limits, parallelism/completions, deadlines, and job history are gaps. |
| CronJob | `APP` plus compose `schedule` using systemd `OnCalendar=`; validate with `systemd-analyze calendar` | Cron syntax, concurrency policy, missed-run deadline, suspend, job history limits, and Kubernetes job retries do not translate. Use `persistent`/`jitter` only when explicitly intended. |
| Multiple containers in one Pod | Separate items in one `network: "pod"` group preserve shared localhost. Lifecycle ordering and non-network namespace/volume sharing require separate dispositions. |
| `terminationGracePeriodSeconds` | Compose `stopTimeout` can bound a member's stop; Cixfile `STOPSIGNAL` can preserve a known signal | PreStop hooks and full Kubernetes termination ordering are not implied. |
| resources requests/limits, QoS, priority | **Not expressible — record as gap.** Do not invent compose resource fields. |
| `securityContext`, Linux capabilities, seccomp/AppArmor/SELinux | Composix applies its own strict DynamicUser/systemd sandbox. Only an existing narrow Cixfile claim may be used for its stated app semantic | Exact Kubernetes identity/capability/profile requests are gaps; never weaken the host out of band and call it translated. |
| serviceAccount, RBAC, API watch, admission/injection | **Not expressible — record as gap.** An API-watching controller remains a controller-shaped refusal unless a non-Kubernetes contract is supplied. |
| node selectors, affinity, tolerations, topology spread | **Not expressible — record as gap.** Composix activates a declared tree on one chosen host; it is not a scheduler. |
| hostNetwork/hostPID/hostIPC, DNS policy, hostname/subdomain | Only the documented pod netns or deliberate host-network absence exists | Requested namespace modes and Kubernetes DNS/identity behavior that differ are gaps. |
| labels and annotations | Dissolve only after resolving selectors and any behavior-bearing integrations | Display metadata and arbitrary controller annotations have no generic runtime effect in composix. Integration annotations must be understood or recorded as gaps. |

Systemd restarting a failed process is not a Deployment controller. A Nix profile generation and
`cix rollback` restore earlier unit definitions and item refs, not Kubernetes rollout strategy or
application data. State this whenever the source relies on controller status, surge/unavailable
budgets, readiness-based traffic shifting, or automatic rollback.

## Verification is part of the conversion

Format and build every Cixfile with the real compiler. Tag each built member with an explicit,
case-local ref, then validate and dry-diff the compose tree before activation:

```sh
cix fmt --check path/to/case
cix build path/to/web -t migration-check
cix compose check path/to/case/compose.json
cix compose diff path/to/case/compose.json
```

For source-built items, also run the relevant locked and cold build receipts from the Docker
migration contract. A successful Cixfile build proves syntax and assembly only. A successful
compose check proves references and wiring shape only. Neither proves Kubernetes behavior.

Write a bounded behavioral check with the same central probe in two modes:

- `./check.sh k8s` applies the pinned/rendered manifests in a disposable namespace or otherwise
  runs the declared reference system, waits with explicit bounds, probes behavior, and always
  cleans up; and
- `./check.sh cix` builds/tags the items, activates `compose.json`, waits with explicit bounds,
  performs the same probe, and always runs `cix down`.

Do not use `down --purge` in an ordinary receipt when persistence is part of the contract. Run a
separate, explicit purge test only when deletion behavior is the subject. Capture synchronous exit
statuses; a detached activation or a successful `kubectl apply`/`cix up` is not a runtime receipt.
Compare observable values, not only command success. If the Kubernetes reference cannot be run,
label the result `desk` or `unverified`; do not promote it to parity.

Each corpus case must contain the pinned manifests, `SOURCE`, per-service Cixfiles and their locks,
`compose.json` and its lock, required checked-in context files, `check.sh`, `GAPS.md`, and
`receipt.md`. The receipt names exact commands, bounds, observed values, item paths, and the
remaining semantic differences. `GAPS.md` uses the provenance/status header and routed bullets
from the corpus maintenance contract.

The final migration answer includes every generated file, exact build/check commands, a compact
object-to-output disposition, and the gap ledger. Never turn a one-host approximation, an
unrendered template, or an unverified build into a successful Kubernetes conversion claim.
