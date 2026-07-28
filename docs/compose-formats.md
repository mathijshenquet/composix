# Compose surface-language prototypes

> **Exploratory:** this document is a decision input, not a design decision or an
> implementation contract. Every key and directive below is prototype syntax.

The purpose of compose is to turn already-built, self-describing items into one
operator-owned deployment. The item's `cix-spec.json` remains the app contract:
services and their lifecycle, declared environment surface, ports, writable role
directories, and capabilities. Compose must not restate those facts. It supplies
operator decisions: which item refs and item services to use, spec-value
overrides, credentials, host storage backing, scale and resource policy,
network membership, permitted communication, and published edges.

The non-negotiable mechanism is D9:

1. **Resolve** refs for the current system. A ref such as
   `cix.example.com/dashboard:latest#dashboard` names tag `latest` and service
   `dashboard` inside the resolved item.
2. **Lock** the selected store path, `narHash`, substituters, and provenance in
   the generated sibling `cix.lock`. The lock is data, never hand-maintained
   syntax embedded in the compose file.
3. **Build** a per-composite derivation containing generated units and network
   policy.
4. **Activate** it through the composite's Nix profile, atomically and with
   rollback.

All tags are mutable; the lock is the reproducibility boundary. `hold` means a
tag moves only after an explicit lock update. `watch` means a reconciler
periodically resolves it and, when it moves, writes a new lock, builds, and
atomically activates it. A watched tag is not an unpinned runtime input.

## Prototype semantic vocabulary

These are the same concepts in every rendering. Differences in spelling are
intentional; differences in meaning are not.

- A **service** selects `<ref>#<item-service>`. `override.env` changes only
  environment names declared by that service's spec. `override.dirs` maps a
  spec-declared app path to operator-selected host backing. Unknown overrides
  are schema errors after resolution, not new app contracts.
- A **credential** names a root-readable source. Mapping it to a service's
  spec-declared secret causes `LoadCredential=` delivery; secret bytes never
  enter the compose file, Nix store, generated unit, or ordinary environment.
- A **publish** maps a spec-declared port to a host edge. `socket` requests
  socket activation/proxying rather than granting the service ambient host
  networking.
- A named **socket** is a shared runtime path for apps whose specs already
  declare configurable Unix-socket endpoints. Compose chooses the path; it
  does not invent socket support in either app.
- A named **network** has a realization and stable addressing/IPAM state
  persisted beside composite state. Services share one composite network
  namespace (D23); per-service network membership is an enforced cgroup/eBPF
  allow-list, not one veth per service (D26).
- A **talks-to** edge is permission, not startup ordering and not service
  discovery. For a Unix socket it can be a real per-service capability. At the
  IP tier, services in one composite share an address, so enforcement may be
  only composite-granular; the compiler must report that honestly (D27).
- A **limit profile** is a first-class schema object, included because the
  gnarly case needs repeated policy without smuggling in a general template
  language. Services refer to profiles by name.
- An **endpoint** is either an address on a named network or another
  composite's named published port. It is not an unmanaged service definition.

The prototypes assume schema-aware resolution: syntax parsing alone is
insufficient. Validation must resolve each ref, load its `cix-spec.json`, select
the item service, reject unknown overrides/credentials/ports/directories, detect
fixed-port collisions inside the shared namespace, validate graph references,
and only then write `cix.lock`.

## Scenario definitions

### 1. Minimal capability-first web stack

`nginx:1.27#nginx` fronts `hello:v1#app`. Both item specs declare the
configuration knobs needed for a Unix listener/upstream, so the operator chooses
`/run/minimal/app-http.sock`. The only communication edge is nginx to app over
that socket. Nginx's spec-declared `http` port is published through socket
activation on `0.0.0.0:8080`. The app's spec-declared `/var/lib/hello` state
directory is backed by `/srv/cix/minimal/hello` on the host. Both version tags
are held.

This scenario is intentionally hostile to Docker-shaped defaults: granting a
shared IP network would be broader and therefore wrong.

### 2. `the private fleet repo` dashboard, as found

This was inspected read-only at `/home/mathijs/the private fleet repo`, commit
`(redacted)`. The premise "Node frontend + Rust dashboard + presumably nginx"
is stale:

- `frontend/` is React/TypeScript built with pnpm, but Node is a **build-time
  tool**, not a production service. The Nix package builds the frontend and the
  Rust Axum `dashboard` serves the resulting static files and `/api` from the
  same process.
- Development runs Vite on `5173`, proxying `/api` to Rust on `8787`; neither
  Vite nor `5173` belongs in the production composite.
- Production binds the dashboard to `127.0.0.1:8787`. The deploy script asks
  the existing the host tunnel host service to expose HTTP `80` and HTTPS `443`.
  There is no nginx or nginx configuration in this deployment.
- The dashboard reads PostgreSQL on `data-host:5432`, dataplane gRPC on
  `data-host:50051`, Chain WebSocket RPC on `archive-host:9944`, and a
  read-only bot-log rsync module on `host-c:8873`.
- Mutable data is cache/state, not a database owned by this deployment:
  block views, subnet assets, opportunity shards, analysis results, mirrored
  bot logs, and small user metadata. The deployed cache root is
  `/var/cache/host-dashboard`, backed by a Btrfs subvolume with a `10G` quota;
  service home/state is `/var/lib/host-dashboard`.

The prototype models one runtime service,
`cix.example.com/host-dashboard:latest#dashboard`, with `watch` update policy. It
overrides the declared listener and upstream settings, maps the
spec-declared `DATABASE_URL` secret to a credential sourced from
`/run/keys/host-dashboard-database-url`, and attaches to an externally realized
`tunnelnet` network. Four `talks-to` edges grant the observed outbound
dependencies. The composite publishes only its host-loopback `8787` origin;
the already-installed the host tunnel daemon's `80`/`443` exposure remains explicit
external host configuration, not a fictional nginx service.

This is a faithful deployment model, not a transcription of the repository's
older Docker Compose files. Those files describe other bot/dataplane/node
deployments and a superseded Streamlit dashboard on `8501`.

### 3. Gnarly shared-namespace stack

The `commerce` composite has seven services:

1. `edge` on `frontend`, held at `edge:v7`;
2. `api` on `frontend` and `backend`, watching `api:latest`;
3. `realtime` on `frontend` and `backend`, held at `realtime:v3`;
4. `worker` on `backend`, watching `worker:latest`, scaled to three instances;
5. `db`, adopted from `cixpkgs.org/postgresql:16#postgres`, on `backend`, with
   operator overrides for `POSTGRES_DB` and `POSTGRES_USER` plus a password
   credential;
6. `redis`, adopted from `cixpkgs.org/redis:7#redis`, on `backend`;
7. `metrics` on `backend`, held at a collision-resolving item variant.

`frontend` and `backend` are named bridge networks with stable subnets.
Resource profiles `small`, `standard`, `worker`, and `database` are reused by
name. The edge's HTTPS port is published. Explicit edges allow edge→api,
edge→realtime, api→db, api→redis, realtime→redis, worker→db, worker→redis, and
metrics→api. The API may also call the `ledger` published port of a separate
`payments-prod` composite.

The adversarial wrinkle is real: the initially considered metrics item and the
API both declare an env-blind fixed-value port `9000`. Because all services
share one network namespace, they cannot both bind it. cix-spec v2 explicitly
forbids overriding a fixed-value port, and a host-side proxy cannot repair an
inside-the-namespace bind collision. Every rendering therefore selects
`observability/metrics:port-9100#metrics`, an operator-rebuilt item whose spec
declares `9100`, and leaves a comment at the decision site. A proposed format
that renders this as an innocent `port: 9100` override fails the semantic
contract.

## Candidate renderings

The examples below are complete prototypes, not fragments to be mentally
merged. Comments are part of the test: an operator must be able to explain an
exception next to it.
