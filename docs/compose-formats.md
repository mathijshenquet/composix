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

### TOML

#### Scenario 1

```toml
format_version = 1
name = "minimal"

[services.nginx]
ref = "cix.example.com/nginx:1.27#nginx"
update = "hold"

[services.nginx.override.env]
UPSTREAM = "unix:/run/minimal/app-http.sock"

[services.app]
ref = "cix.example.com/hello:v1#app"
update = "hold"

[services.app.override.env]
LISTEN = "unix:/run/minimal/app-http.sock"

[services.app.override.dirs]
"/var/lib/hello" = { host = "/srv/cix/minimal/hello" }

[sockets.app_http]
path = "/run/minimal/app-http.sock"
provider = "app"

[publishes.web]
from = "nginx.http"
listen = "0.0.0.0:8080"
mode = "socket"

[[talks_to]]
from = "nginx"
to = "app"
via = "socket.app_http"
```

#### Scenario 2

```toml
format_version = 1
name = "host-dashboard"

# the host tunnel itself is existing host infrastructure, not a service invented here.
[networks.tunnelnet]
driver = "host-tunnel"
external = true

[endpoints.postgres]
address = "data-host:5432"
network = "tunnelnet"

[endpoints.dataplane]
address = "data-host:50051"
network = "tunnelnet"

[endpoints.chain]
address = "archive-host:9944"
network = "tunnelnet"

[endpoints.bot_logs]
address = "host-c:8873"
network = "tunnelnet"

[credentials.database_url]
source = "/run/keys/host-dashboard-database-url"

[services.dashboard]
ref = "cix.example.com/host-dashboard:latest#dashboard"
update = "watch"
networks = ["tunnelnet"]

[services.dashboard.credentials]
DATABASE_URL = "database_url"

[services.dashboard.override.env]
DASHBOARD_LISTEN = "127.0.0.1:8787"
DATAPLANE_GRPC_URL = "http://data-host:50051"
CHAIN_RPC_URL = "ws://archive-host:9944"
APP_BOT_LOG_RSYNC_SOURCE = "rsync://host-c:8873/bot-logs/"

[services.dashboard.override.dirs]
"/var/lib/host-dashboard" = { host = "/var/lib/host-dashboard" }
"/var/cache/host-dashboard" = { host = "/var/cache/host-dashboard", quota = "10G" }

# Existing the host tunnel Serve forwards tunnelnet :80/:443 to this host-loopback origin.
[publishes.origin]
from = "dashboard.http"
listen = "127.0.0.1:8787"
mode = "proxy"

[[talks_to]]
from = "dashboard"
to = "endpoint.postgres"
via = "network.tunnelnet"

[[talks_to]]
from = "dashboard"
to = "endpoint.dataplane"
via = "network.tunnelnet"

[[talks_to]]
from = "dashboard"
to = "endpoint.chain"
via = "network.tunnelnet"

[[talks_to]]
from = "dashboard"
to = "endpoint.bot_logs"
via = "network.tunnelnet"
```

#### Scenario 3

```toml
format_version = 1
name = "commerce"

[limits.small]
cpu = "250m"
memory = "256MiB"

[limits.standard]
cpu = "1"
memory = "1GiB"

[limits.worker]
cpu = "2"
memory = "2GiB"

[limits.database]
cpu = "4"
memory = "8GiB"

[networks.frontend]
driver = "bridge"
subnet = "10.42.10.0/24"

[networks.backend]
driver = "bridge"
subnet = "10.42.20.0/24"

[credentials.postgres_password]
source = "/run/keys/commerce-postgres-password"

[endpoints.payments_ledger]
composite = "payments-prod"
published = "ledger"

[services.edge]
ref = "cix.example.com/commerce/edge:v7#edge"
update = "hold"
networks = ["frontend"]
limits = "small"

[services.api]
ref = "cix.example.com/commerce/api:latest#api"
update = "watch"
networks = ["frontend", "backend"]
limits = "standard"

[services.realtime]
ref = "cix.example.com/commerce/realtime:v3#realtime"
update = "hold"
networks = ["frontend", "backend"]
limits = "standard"

[services.worker]
ref = "cix.example.com/commerce/worker:latest#worker"
update = "watch"
networks = ["backend"]
limits = "worker"
scale = 3

[services.db]
ref = "cixpkgs.org/postgresql:16#postgres"
update = "hold"
networks = ["backend"]
limits = "database"

[services.db.credentials]
POSTGRES_PASSWORD = "postgres_password"

[services.db.override.env]
POSTGRES_DB = "commerce"
POSTGRES_USER = "commerce"

[services.redis]
ref = "cixpkgs.org/redis:7#redis"
update = "hold"
networks = ["backend"]
limits = "small"

[services.metrics]
# :latest declares fixed 9000 and collides with api; compose cannot override it.
ref = "cix.example.com/observability/metrics:port-9100#metrics"
update = "hold"
networks = ["backend"]
limits = "small"

[publishes.https]
from = "edge.https"
listen = "0.0.0.0:443"
mode = "socket"

[[talks_to]]
from = "edge"
to = "api"
via = "network.frontend"

[[talks_to]]
from = "edge"
to = "realtime"
via = "network.frontend"

[[talks_to]]
from = "api"
to = "db"
via = "network.backend"

[[talks_to]]
from = "api"
to = "redis"
via = "network.backend"

[[talks_to]]
from = "api"
to = "endpoint.payments_ledger"
via = "published"

[[talks_to]]
from = "realtime"
to = "redis"
via = "network.backend"

[[talks_to]]
from = "worker"
to = "db"
via = "network.backend"

[[talks_to]]
from = "worker"
to = "redis"
via = "network.backend"

[[talks_to]]
from = "metrics"
to = "api"
via = "network.backend"
```

#### TOML: what survives contact

Pros:

- Strings, booleans, arrays, and tables have a small, stable grammar. `NO`,
  `on`, and `2026-07-28` do not silently become unrelated types.
- Table headers make ownership explicit. The ugly
  `[services.dashboard.override.dirs]` is at least unambiguous.
- Dotted references remain ordinary schema-validated strings, and named limit
  profiles solve the required reuse without another language.
- Comments are predictable, diffs are stable, duplicate tables/keys are
  errors, and Taplo supplies a credible formatter, editor integration, and
  schema-aware completion.
- Rust support is cheap and mature. Deserialization plus a post-resolution
  semantic validator fits the existing implementation.

Cons:

- The gnarly graph is a wall of `[[talks_to]]` tables. TOML is bad at repeated
  structured records and makes no attempt to hide it.
- Context is carried by distant headers. Moving a stanza can silently change
  which object following keys belong to; reviewers must read upward.
- Inline tables must stay on one physical line. Directory backing is compact
  here, but adding several policy fields forces yet more headers.
- There is no native structural reuse. Named schema objects work; arbitrary
  partial-service inheritance does not. That is a feature until users demand
  it, at which point they will invent preprocessors.
- Bare dates and times are typed TOML values, so quote version-like dates and
  every unit-bearing scalar. A schema catches this, a text review may not.
- Docker Compose migration is conceptual rather than mechanical. The syntax is
  familiar enough to read but not familiar enough to paste.

### YAML

The YAML examples deliberately avoid anchors and merge keys. Anchors can share
nodes, but the commonly used `<<` merge key is not part of YAML 1.2 and gives
different results across loaders. Required reuse goes through the same named
limit-profile schema as every other candidate.

#### Scenario 1

```yaml
format-version: 1
name: minimal

services:
  nginx:
    ref: cix.example.com/nginx:1.27#nginx
    update: hold
    override:
      env:
        UPSTREAM: unix:/run/minimal/app-http.sock
  app:
    ref: cix.example.com/hello:v1#app
    update: hold
    override:
      env:
        LISTEN: unix:/run/minimal/app-http.sock
      dirs:
        /var/lib/hello:
          host: /srv/cix/minimal/hello

sockets:
  app-http:
    path: /run/minimal/app-http.sock
    provider: app

publishes:
  web:
    from: nginx.http
    listen: 0.0.0.0:8080
    mode: socket

talks-to:
  - from: nginx
    to: app
    via: socket.app-http
```

#### Scenario 2

```yaml
format-version: 1
name: host-dashboard

networks:
  # the host tunnel itself is existing host infrastructure, not a service invented here.
  tunnelnet:
    driver: host-tunnel
    external: true

endpoints:
  postgres:
    address: data-host:5432
    network: tunnelnet
  dataplane:
    address: data-host:50051
    network: tunnelnet
  chain:
    address: archive-host:9944
    network: tunnelnet
  bot-logs:
    address: host-c:8873
    network: tunnelnet

credentials:
  database-url:
    source: /run/keys/host-dashboard-database-url

services:
  dashboard:
    ref: cix.example.com/host-dashboard:latest#dashboard
    update: watch
    networks: [tunnelnet]
    credentials:
      DATABASE_URL: database-url
    override:
      env:
        DASHBOARD_LISTEN: 127.0.0.1:8787
        DATAPLANE_GRPC_URL: http://data-host:50051
        CHAIN_RPC_URL: ws://archive-host:9944
        APP_BOT_LOG_RSYNC_SOURCE: rsync://host-c:8873/bot-logs/
      dirs:
        /var/lib/host-dashboard:
          host: /var/lib/host-dashboard
        /var/cache/host-dashboard:
          host: /var/cache/host-dashboard
          quota: 10G

publishes:
  # Existing the host tunnel Serve forwards tunnelnet :80/:443 to this host-loopback origin.
  origin:
    from: dashboard.http
    listen: 127.0.0.1:8787
    mode: proxy

talks-to:
  - { from: dashboard, to: endpoint.postgres, via: network.tunnelnet }
  - { from: dashboard, to: endpoint.dataplane, via: network.tunnelnet }
  - { from: dashboard, to: endpoint.chain, via: network.tunnelnet }
  - { from: dashboard, to: endpoint.bot-logs, via: network.tunnelnet }
```

#### Scenario 3

```yaml
format-version: 1
name: commerce

limits:
  small: { cpu: 250m, memory: 256MiB }
  standard: { cpu: "1", memory: 1GiB }
  worker: { cpu: "2", memory: 2GiB }
  database: { cpu: "4", memory: 8GiB }

networks:
  frontend: { driver: bridge, subnet: 10.42.10.0/24 }
  backend: { driver: bridge, subnet: 10.42.20.0/24 }

credentials:
  postgres-password:
    source: /run/keys/commerce-postgres-password

endpoints:
  payments-ledger:
    composite: payments-prod
    published: ledger

services:
  edge:
    ref: cix.example.com/commerce/edge:v7#edge
    update: hold
    networks: [frontend]
    limits: small
  api:
    ref: cix.example.com/commerce/api:latest#api
    update: watch
    networks: [frontend, backend]
    limits: standard
  realtime:
    ref: cix.example.com/commerce/realtime:v3#realtime
    update: hold
    networks: [frontend, backend]
    limits: standard
  worker:
    ref: cix.example.com/commerce/worker:latest#worker
    update: watch
    networks: [backend]
    limits: worker
    scale: 3
  db:
    ref: cixpkgs.org/postgresql:16#postgres
    update: hold
    networks: [backend]
    limits: database
    credentials:
      POSTGRES_PASSWORD: postgres-password
    override:
      env:
        POSTGRES_DB: commerce
        POSTGRES_USER: commerce
  redis:
    ref: cixpkgs.org/redis:7#redis
    update: hold
    networks: [backend]
    limits: small
  metrics:
    # :latest declares fixed 9000 and collides with api; compose cannot override it.
    ref: cix.example.com/observability/metrics:port-9100#metrics
    update: hold
    networks: [backend]
    limits: small

publishes:
  https:
    from: edge.https
    listen: 0.0.0.0:443
    mode: socket

talks-to:
  - { from: edge, to: api, via: network.frontend }
  - { from: edge, to: realtime, via: network.frontend }
  - { from: api, to: db, via: network.backend }
  - { from: api, to: redis, via: network.backend }
  - { from: api, to: endpoint.payments-ledger, via: published }
  - { from: realtime, to: redis, via: network.backend }
  - { from: worker, to: db, via: network.backend }
  - { from: worker, to: redis, via: network.backend }
  - { from: metrics, to: api, via: network.backend }
```

#### YAML: what survives contact

Pros:

- It is the Docker Compose incumbent. A Docker user immediately recognizes
  `services`, `networks`, mappings, lists, and inline records.
- The gnarly graph is materially shorter than TOML. Nested ownership is visible
  without repeating full table paths.
- Comments are good, JSON Schema tooling is widespread, and editors already
  know how to associate schemas with YAML filenames.
- Named objects and string references read naturally. Anchors are available
  for raw node reuse when portability is deliberately abandoned.
- Parser, formatter, and LSP costs for us are low if we select one YAML version
  and one strict loader.

Cons:

- "Select one strict loader" is the trap. YAML 1.1 turns `NO`, `On`, `yes`, and
  `off` into booleans; YAML 1.2 fixes that, but deployed parsers and editor
  validators still disagree. Timestamps and sexagesimal/legacy number forms
  add more implicit typing failures. Quotes become defensive programming.
- Duplicate keys are legal or rejected depending on loader settings. We would
  have to reject them explicitly or accept silent policy replacement.
- Indentation is structure with weak visual delimiters. One misplaced level
  can move a credential or override to another owner while still parsing.
- `:` followed by space, `#` after whitespace, aliases, tags, block scalars,
  and multiple documents are language features composix does not need but its
  parser and threat model inherit.
- Anchors copy syntax nodes, not domain objects. Alias mutation semantics vary
  by library, merge precedence is review-hostile, and cross-file reuse still
  needs a preprocessor.
- Familiarity is dangerous here: Docker-shaped syntax encourages users to
  paste `command`, `depends_on`, app port declarations, and environment
  contracts that belong in the item spec. Schema errors must be blunt.
- Formatters are not culturally standard for YAML. Semantically identical
  flow/block-style churn is common in review.
