# Compose surface-language prototypes

> **Exploratory:** this document is a decision input, not a design decision or an
> implementation contract. Every key and directive below is prototype syntax.

The purpose of compose is to turn already-built, self-describing items into one
operator-owned deployment. The item's `cix-manifest.json` remains the app contract:
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
insufficient. Validation must resolve each ref, load its `cix-manifest.json`, select
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

### 2. the fleet dashboard (private repo), as found

This was inspected read-only at `a private fleet repo`, commit
`redacted`. The premise "Node frontend + Rust dashboard + presumably nginx"
is stale:

- `frontend/` is React/TypeScript built with pnpm, but Node is a **build-time
  tool**, not a production service. The Nix package builds the frontend and the
  Rust Axum `dashboard` serves the resulting static files and `/api` from the
  same process.
- Development runs Vite on `5173`, proxying `/api` to Rust on `8787`; neither
  Vite nor `5173` belongs in the production composite.
- Production binds the dashboard to `127.0.0.1:8787`. The deploy script asks
  an existing host-level tunnel service to expose HTTP `80` and HTTPS `443`.
  There is no nginx or nginx configuration in this deployment.
- The dashboard reads PostgreSQL on `data-host:5432`, dataplane gRPC on
  `data-host:50051`, an external chain WebSocket RPC on `archive-host:9944`, and a
  read-only bot-log rsync module on `host-c:8873`. It also needs outbound
  HTTPS for GitHub identities, subnet images, and optionally ntfy/Slack.
- Mutable data is cache/state, not a database owned by this deployment:
  block views, subnet assets, opportunity shards, analysis results, mirrored
  bot logs, and small user metadata. The active `dashboard-deploy` path uses
  `/home/mthq/.cache/host-dashboard-block-cache` for block/opportunity/analysis
  data, backed by a Btrfs subvolume with a `10G` quota, and other caches below
  `/home/mthq/.cache`. A separate checked-in NixOS module uses
  `/var/cache/host-dashboard`; that is not the user-service deployment modeled
  here.

The prototype models one runtime service,
`cix.example.com/host-dashboard:latest#dashboard`, with `watch` update policy. It
overrides the declared listener and upstream settings, maps the
spec-declared `DATABASE_URL` secret to a credential sourced from
`/run/keys/host-dashboard-database-url`, and attaches to an externally realized
`tunnelnet` plus a local egress network. Four `talks-to` edges grant the observed
tunnelnet dependencies and one grants public HTTPS egress. The composite
publishes only its host-loopback `8787` origin. The already-installed the host tunnel
daemon's `80`/`443` exposure remains explicit
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
share one network namespace, they cannot both bind it. Spec v2 explicitly
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

[networks.egress]
driver = "bridge"
subnet = "10.42.30.0/24"
egress = true

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

[endpoints.public_https]
address = "0.0.0.0/0"
port = 443
network = "egress"

[credentials.database_url]
source = "/run/keys/host-dashboard-database-url"

[services.dashboard]
ref = "cix.example.com/host-dashboard:latest#dashboard"
update = "watch"
networks = ["tunnelnet", "egress"]

[services.dashboard.credentials]
DATABASE_URL = "database_url"

[services.dashboard.override.env]
DASHBOARD_LISTEN = "127.0.0.1:8787"
DATAPLANE_GRPC_URL = "http://data-host:50051"
CHAIN_RPC_URL = "ws://archive-host:9944"
APP_BOT_LOG_RSYNC_SOURCE = "rsync://host-c:8873/bot-logs/"
DASHBOARD_BLOCK_CACHE = "/var/cache/host-dashboard-block-cache/block-views"
DASHBOARD_SUBNET_ASSET_CACHE = "/var/cache/host-dashboard/subnets"
APP_MEV_ANALYSIS_CACHE_DIR = "/var/cache/host-dashboard-block-cache/opportunities"
APP_MEV_ANALYSIS_RESULT_CACHE_DIR = "/var/cache/host-dashboard-block-cache/analysis-results"
APP_BOT_LOG_CACHE_DIR = "/var/cache/app/bot-logs/host-c"

[services.dashboard.override.dirs]
"/var/cache/host-dashboard-block-cache" = { host = "/home/mthq/.cache/host-dashboard-block-cache", quota = "10G" }
"/var/cache/host-dashboard" = { host = "/home/mthq/.cache/host-dashboard" }
"/var/cache/the-org" = { host = "/home/mthq/.cache/the-org" }

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

[[talks_to]]
from = "dashboard"
to = "endpoint.public_https"
via = "network.egress"
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
  egress:
    driver: bridge
    subnet: 10.42.30.0/24
    egress: true

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
  public-https:
    address: 0.0.0.0/0
    port: 443
    network: egress

credentials:
  database-url:
    source: /run/keys/host-dashboard-database-url

services:
  dashboard:
    ref: cix.example.com/host-dashboard:latest#dashboard
    update: watch
    networks: [tunnelnet, egress]
    credentials:
      DATABASE_URL: database-url
    override:
      env:
        DASHBOARD_LISTEN: 127.0.0.1:8787
        DATAPLANE_GRPC_URL: http://data-host:50051
        CHAIN_RPC_URL: ws://archive-host:9944
        APP_BOT_LOG_RSYNC_SOURCE: rsync://host-c:8873/bot-logs/
        DASHBOARD_BLOCK_CACHE: /var/cache/host-dashboard-block-cache/block-views
        DASHBOARD_SUBNET_ASSET_CACHE: /var/cache/host-dashboard/subnets
        APP_MEV_ANALYSIS_CACHE_DIR: /var/cache/host-dashboard-block-cache/opportunities
        APP_MEV_ANALYSIS_RESULT_CACHE_DIR: /var/cache/host-dashboard-block-cache/analysis-results
        APP_BOT_LOG_CACHE_DIR: /var/cache/app/bot-logs/host-c
      dirs:
        /var/cache/host-dashboard-block-cache:
          host: /home/mthq/.cache/host-dashboard-block-cache
          quota: 10G
        /var/cache/host-dashboard:
          host: /home/mthq/.cache/host-dashboard
        /var/cache/the-org:
          host: /home/mthq/.cache/the-org

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
  - { from: dashboard, to: endpoint.public-https, via: network.egress }
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

### nix-lite

This candidate is intentionally not "a Nix module." The file
`compose.cix.nix` has one optional, non-recursive top-level `let` followed by
one plain attrset.

Allowed features are:

- static attrsets, lists, double-quoted strings, integers, booleans, and `null`;
- one top-level `let ... in`, whose bindings must be scalar literals;
- `${name}` interpolation of those scalar bindings inside double-quoted
  strings;
- static attribute names and `#` or `/* ... */` comments.

Banned features are functions and application, `rec`, `with`, `inherit`,
`import`, all `builtins`, flakes, overlays/modules, `if`, `assert`, operators
including `//` and `+`, attribute selection as an expression, dynamic attribute
names, path/search-path/URI literals, indented strings, and interpolation of
anything except a scalar top-level binding. In particular, limit-profile reuse
below is a compose-schema reference by string, not Nix object sharing.

Evaluation has no API function and no injected `pkgs`: parse with a Nix syntax
tree library, reject every node outside the whitelist, run the installed Nix
evaluator with `--pure-eval --restrict-eval --json`, then validate the JSON
against the compose schema and resolved item specs. Calling `nix eval` directly
is unsupported because it skips the whitelist. This is real Nix evaluation,
but almost none of the language.

#### Scenario 1

```nix
{
  formatVersion = 1;
  name = "minimal";

  services = {
    nginx = {
      ref = "cix.example.com/nginx:1.27#nginx";
      update = "hold";
      override.env.UPSTREAM = "unix:/run/minimal/app-http.sock";
    };
    app = {
      ref = "cix.example.com/hello:v1#app";
      update = "hold";
      override = {
        env.LISTEN = "unix:/run/minimal/app-http.sock";
        dirs."/var/lib/hello".host = "/srv/cix/minimal/hello";
      };
    };
  };

  sockets.app-http = {
    path = "/run/minimal/app-http.sock";
    provider = "app";
  };

  publishes.web = {
    from = "nginx.http";
    listen = "0.0.0.0:8080";
    mode = "socket";
  };

  talksTo = [
    {
      from = "nginx";
      to = "app";
      via = "socket.app-http";
    }
  ];
}
```

#### Scenario 2

```nix
let
  dataHost = "data-host";
  archiveHost = "archive-host";
  botHost = "host-c";
in
{
  formatVersion = 1;
  name = "host-dashboard";

  networks.tunnelnet = {
    # the host tunnel itself is existing host infrastructure, not a service invented here.
    driver = "host-tunnel";
    external = true;
  };
  networks.egress = {
    driver = "bridge";
    subnet = "10.42.30.0/24";
    egress = true;
  };

  endpoints = {
    postgres = {
      address = "${dataHost}:5432";
      network = "tunnelnet";
    };
    dataplane = {
      address = "${dataHost}:50051";
      network = "tunnelnet";
    };
    chain = {
      address = "${archiveHost}:9944";
      network = "tunnelnet";
    };
    bot-logs = {
      address = "${botHost}:8873";
      network = "tunnelnet";
    };
    public-https = {
      address = "0.0.0.0/0";
      port = 443;
      network = "egress";
    };
  };

  credentials.database-url.source =
    "/run/keys/host-dashboard-database-url";

  services.dashboard = {
    ref = "cix.example.com/host-dashboard:latest#dashboard";
    update = "watch";
    networks = [ "tunnelnet" "egress" ];
    credentials.DATABASE_URL = "database-url";
    override = {
      env = {
        DASHBOARD_LISTEN = "127.0.0.1:8787";
        DATAPLANE_GRPC_URL = "http://${dataHost}:50051";
        CHAIN_RPC_URL = "ws://${archiveHost}:9944";
        APP_BOT_LOG_RSYNC_SOURCE =
          "rsync://${botHost}:8873/bot-logs/";
        DASHBOARD_BLOCK_CACHE =
          "/var/cache/host-dashboard-block-cache/block-views";
        DASHBOARD_SUBNET_ASSET_CACHE =
          "/var/cache/host-dashboard/subnets";
        APP_MEV_ANALYSIS_CACHE_DIR =
          "/var/cache/host-dashboard-block-cache/opportunities";
        APP_MEV_ANALYSIS_RESULT_CACHE_DIR =
          "/var/cache/host-dashboard-block-cache/analysis-results";
        APP_BOT_LOG_CACHE_DIR = "/var/cache/app/bot-logs/host-c";
      };
      dirs = {
        "/var/cache/host-dashboard-block-cache" = {
          host = "/home/mthq/.cache/host-dashboard-block-cache";
          quota = "10G";
        };
        "/var/cache/host-dashboard" = {
          host = "/home/mthq/.cache/host-dashboard";
        };
        "/var/cache/the-org".host = "/home/mthq/.cache/the-org";
      };
    };
  };

  # Existing the host tunnel Serve forwards tunnelnet :80/:443 to this host-loopback origin.
  publishes.origin = {
    from = "dashboard.http";
    listen = "127.0.0.1:8787";
    mode = "proxy";
  };

  talksTo = [
    {
      from = "dashboard";
      to = "endpoint.postgres";
      via = "network.tunnelnet";
    }
    {
      from = "dashboard";
      to = "endpoint.dataplane";
      via = "network.tunnelnet";
    }
    {
      from = "dashboard";
      to = "endpoint.chain";
      via = "network.tunnelnet";
    }
    {
      from = "dashboard";
      to = "endpoint.bot-logs";
      via = "network.tunnelnet";
    }
    {
      from = "dashboard";
      to = "endpoint.public-https";
      via = "network.egress";
    }
  ];
}
```

#### Scenario 3

```nix
{
  formatVersion = 1;
  name = "commerce";

  limits = {
    small = {
      cpu = "250m";
      memory = "256MiB";
    };
    standard = {
      cpu = "1";
      memory = "1GiB";
    };
    worker = {
      cpu = "2";
      memory = "2GiB";
    };
    database = {
      cpu = "4";
      memory = "8GiB";
    };
  };

  networks = {
    frontend = {
      driver = "bridge";
      subnet = "10.42.10.0/24";
    };
    backend = {
      driver = "bridge";
      subnet = "10.42.20.0/24";
    };
  };

  credentials.postgres-password.source =
    "/run/keys/commerce-postgres-password";

  endpoints.payments-ledger = {
    composite = "payments-prod";
    published = "ledger";
  };

  services = {
    edge = {
      ref = "cix.example.com/commerce/edge:v7#edge";
      update = "hold";
      networks = [ "frontend" ];
      limits = "small";
    };
    api = {
      ref = "cix.example.com/commerce/api:latest#api";
      update = "watch";
      networks = [ "frontend" "backend" ];
      limits = "standard";
    };
    realtime = {
      ref = "cix.example.com/commerce/realtime:v3#realtime";
      update = "hold";
      networks = [ "frontend" "backend" ];
      limits = "standard";
    };
    worker = {
      ref = "cix.example.com/commerce/worker:latest#worker";
      update = "watch";
      networks = [ "backend" ];
      limits = "worker";
      scale = 3;
    };
    db = {
      ref = "cixpkgs.org/postgresql:16#postgres";
      update = "hold";
      networks = [ "backend" ];
      limits = "database";
      credentials.POSTGRES_PASSWORD = "postgres-password";
      override.env = {
        POSTGRES_DB = "commerce";
        POSTGRES_USER = "commerce";
      };
    };
    redis = {
      ref = "cixpkgs.org/redis:7#redis";
      update = "hold";
      networks = [ "backend" ];
      limits = "small";
    };
    metrics = {
      # :latest declares fixed 9000 and collides with api; compose cannot override it.
      ref = "cix.example.com/observability/metrics:port-9100#metrics";
      update = "hold";
      networks = [ "backend" ];
      limits = "small";
    };
  };

  publishes.https = {
    from = "edge.https";
    listen = "0.0.0.0:443";
    mode = "socket";
  };

  talksTo = [
    {
      from = "edge";
      to = "api";
      via = "network.frontend";
    }
    {
      from = "edge";
      to = "realtime";
      via = "network.frontend";
    }
    {
      from = "api";
      to = "db";
      via = "network.backend";
    }
    {
      from = "api";
      to = "redis";
      via = "network.backend";
    }
    {
      from = "api";
      to = "endpoint.payments-ledger";
      via = "published";
    }
    {
      from = "realtime";
      to = "redis";
      via = "network.backend";
    }
    {
      from = "worker";
      to = "db";
      via = "network.backend";
    }
    {
      from = "worker";
      to = "redis";
      via = "network.backend";
    }
    {
      from = "metrics";
      to = "api";
      via = "network.backend";
    }
  ];
}
```

#### nix-lite: what survives contact

Pros:

- Attrsets and lists render all three cases without YAML's implicit typing or
  TOML's repeated-table ceremony. Braces make ownership and moved blocks
  obvious in review.
- Duplicate attributes are errors. Strings have one interpolation model, and
  `nixfmt` provides deterministic formatting.
- Scalar interpolation removes the only repetition that was genuinely noisy
  in scenario 2 without admitting structural programming.
- Nix is already a product dependency, and Rust Nix parsers exist. The
  evaluated JSON can use exactly the same semantic validator as TOML/YAML.
- The syntax makes the resolve/build boundary unsurprising to existing Nix
  users, while `cix.lock` still owns mutable-ref resolution.

Cons:

- It is bait. The file looks like Nix, has a `.nix` suffix, and rejects almost
  every useful answer from a Nix search result: `inherit`, `rec`, `//`,
  functions, imports, modules, and even attrset-valued `let` reuse.
- Conversely, non-Nix users still pay for braces, semicolons, dotted
  attributes, unusual list syntax, and error messages containing Nix terms.
  D4 explicitly chose a Dockerfile-ish Cixfile because no one wants to write
  Nix; compose has no evidence that it is exempt.
- Safe evaluation is not free. We need an AST whitelist kept in lockstep with
  the parser, a restricted evaluator invocation, and tests proving rejected
  constructs cannot smuggle paths, imports, or string contexts into output.
- Generic Nix language servers will recommend banned features and report
  scalar-only rules too late. A composix-specific diagnostic layer is still
  required.
- Reuse is no better than TOML after the restrictions: named schema objects
  and scalar interpolation. Relaxing that boundary starts the slide toward
  functions-as-API and a module system.
- Docker Compose migration is worst-in-class. The format communicates the
  implementation substrate instead of the operator domain.

### Cixfile-sibling directives

Call the prototype file `Cixcompose`. The grammar is deliberately less magical
than a shell:

- top-level directives start in column 1; service child directives have
  exactly two leading spaces; tabs and any other indentation are errors;
- directive and enum names are case-sensitive uppercase tokens;
- arguments are whitespace-delimited; a value containing whitespace uses a
  double-quoted JSON string;
- `#` starts a comment only when it is the first non-whitespace character, so
  `ref#service` remains one token; trailing comments are not supported;
- a `SERVICE` block ends at the next top-level directive; child directives are
  illegal at top level and top-level directives are illegal when indented;
- unknown directives, duplicate singleton children, duplicate names, and
  unresolved references are errors.

This extends the Cixfile family but not the Cixfile grammar itself. The
top-level directives are `COMPOSITE`, `LIMITS`, `NETWORK`, `CREDENTIAL`,
`ENDPOINT`, `SOCKET`, `PUBLISH`, `SERVICE`, and `TALKS-TO`. Service children are
`UPDATE`, `JOINS`, `LIMITS`, `SCALE`, `SECRET`, and `OVERRIDE ENV|DIR`.

#### Scenario 1

```dockerfile
COMPOSITE minimal

SERVICE nginx cix.example.com/nginx:1.27#nginx
  UPDATE hold
  OVERRIDE ENV UPSTREAM = unix:/run/minimal/app-http.sock

SERVICE app cix.example.com/hello:v1#app
  UPDATE hold
  OVERRIDE ENV LISTEN = unix:/run/minimal/app-http.sock
  OVERRIDE DIR /var/lib/hello = /srv/cix/minimal/hello

SOCKET app-http PATH /run/minimal/app-http.sock PROVIDER app
PUBLISH web FROM nginx.http LISTEN 0.0.0.0:8080 MODE socket
TALKS-TO nginx -> app VIA socket.app-http
```

#### Scenario 2

```dockerfile
COMPOSITE host-dashboard

# the host tunnel itself is existing host infrastructure, not a service invented here.
NETWORK tunnelnet DRIVER host-tunnel EXTERNAL
NETWORK egress DRIVER bridge SUBNET 10.42.30.0/24 EGRESS

ENDPOINT postgres ADDRESS data-host:5432 NETWORK tunnelnet
ENDPOINT dataplane ADDRESS data-host:50051 NETWORK tunnelnet
ENDPOINT chain ADDRESS archive-host:9944 NETWORK tunnelnet
ENDPOINT bot-logs ADDRESS host-c:8873 NETWORK tunnelnet
ENDPOINT public-https CIDR 0.0.0.0/0 PORT 443 NETWORK egress

CREDENTIAL database-url FILE /run/keys/host-dashboard-database-url

SERVICE dashboard cix.example.com/host-dashboard:latest#dashboard
  UPDATE watch
  JOINS tunnelnet egress
  SECRET DATABASE_URL FROM database-url
  OVERRIDE ENV DASHBOARD_LISTEN = 127.0.0.1:8787
  OVERRIDE ENV DATAPLANE_GRPC_URL = http://data-host:50051
  OVERRIDE ENV CHAIN_RPC_URL = ws://archive-host:9944
  OVERRIDE ENV APP_BOT_LOG_RSYNC_SOURCE = rsync://host-c:8873/bot-logs/
  OVERRIDE ENV DASHBOARD_BLOCK_CACHE = /var/cache/host-dashboard-block-cache/block-views
  OVERRIDE ENV DASHBOARD_SUBNET_ASSET_CACHE = /var/cache/host-dashboard/subnets
  OVERRIDE ENV APP_MEV_ANALYSIS_CACHE_DIR = /var/cache/host-dashboard-block-cache/opportunities
  OVERRIDE ENV APP_MEV_ANALYSIS_RESULT_CACHE_DIR = /var/cache/host-dashboard-block-cache/analysis-results
  OVERRIDE ENV APP_BOT_LOG_CACHE_DIR = /var/cache/app/bot-logs/host-c
  OVERRIDE DIR /var/cache/host-dashboard-block-cache = /home/mthq/.cache/host-dashboard-block-cache QUOTA 10G
  OVERRIDE DIR /var/cache/host-dashboard = /home/mthq/.cache/host-dashboard
  OVERRIDE DIR /var/cache/the-org = /home/mthq/.cache/the-org

# Existing the host tunnel Serve forwards tunnelnet :80/:443 to this host-loopback origin.
PUBLISH origin FROM dashboard.http LISTEN 127.0.0.1:8787 MODE proxy

TALKS-TO dashboard -> endpoint.postgres VIA network.tunnelnet
TALKS-TO dashboard -> endpoint.dataplane VIA network.tunnelnet
TALKS-TO dashboard -> endpoint.chain VIA network.tunnelnet
TALKS-TO dashboard -> endpoint.bot-logs VIA network.tunnelnet
TALKS-TO dashboard -> endpoint.public-https VIA network.egress
```

#### Scenario 3

```dockerfile
COMPOSITE commerce

LIMITS small CPU 250m MEMORY 256MiB
LIMITS standard CPU 1 MEMORY 1GiB
LIMITS worker CPU 2 MEMORY 2GiB
LIMITS database CPU 4 MEMORY 8GiB

NETWORK frontend DRIVER bridge SUBNET 10.42.10.0/24
NETWORK backend DRIVER bridge SUBNET 10.42.20.0/24

CREDENTIAL postgres-password FILE /run/keys/commerce-postgres-password
ENDPOINT payments-ledger COMPOSITE payments-prod PUBLISH ledger

SERVICE edge cix.example.com/commerce/edge:v7#edge
  UPDATE hold
  JOINS frontend
  LIMITS small

SERVICE api cix.example.com/commerce/api:latest#api
  UPDATE watch
  JOINS frontend backend
  LIMITS standard

SERVICE realtime cix.example.com/commerce/realtime:v3#realtime
  UPDATE hold
  JOINS frontend backend
  LIMITS standard

SERVICE worker cix.example.com/commerce/worker:latest#worker
  UPDATE watch
  JOINS backend
  LIMITS worker
  SCALE 3

SERVICE db cixpkgs.org/postgresql:16#postgres
  UPDATE hold
  JOINS backend
  LIMITS database
  SECRET POSTGRES_PASSWORD FROM postgres-password
  OVERRIDE ENV POSTGRES_DB = commerce
  OVERRIDE ENV POSTGRES_USER = commerce

SERVICE redis cixpkgs.org/redis:7#redis
  UPDATE hold
  JOINS backend
  LIMITS small

# :latest declares fixed 9000 and collides with api; compose cannot override it.
SERVICE metrics cix.example.com/observability/metrics:port-9100#metrics
  UPDATE hold
  JOINS backend
  LIMITS small

PUBLISH https FROM edge.https LISTEN 0.0.0.0:443 MODE socket

TALKS-TO edge -> api VIA network.frontend
TALKS-TO edge -> realtime VIA network.frontend
TALKS-TO api -> db VIA network.backend
TALKS-TO api -> redis VIA network.backend
TALKS-TO api -> endpoint.payments-ledger VIA published
TALKS-TO realtime -> redis VIA network.backend
TALKS-TO worker -> db VIA network.backend
TALKS-TO worker -> redis VIA network.backend
TALKS-TO metrics -> api VIA network.backend
```

#### Cixfile sibling: what survives contact

Pros:

- It says the domain out loud. `TALKS-TO`, `JOINS`, `SECRET`, `UPDATE`, and
  `PUBLISH` are harder to confuse with app-contract declarations than generic
  mapping keys.
- The gnarly communication graph is the only rendering that can be scanned as
  a graph instead of a list of records. Refs and hold/watch policy are adjacent.
- There are no implicit scalar types, aliases, merge rules, expression
  evaluation, or invisible environment substitution.
- Full-line comments and one-statement-per-line make diffs disciplined. The
  fixed-port decision is exactly where a reviewer looks: above `SERVICE
  metrics`.
- The Cixfile parser's directive model, source diagnostics, name validation,
  and runtime/build interpolation distinctions provide implementation prior
  art. Users learn one broad syntax family for build and compose.

Cons:

- This is a new language. We own the parser, formatter, syntax highlighting,
  JSON-schema equivalent, documentation generator, and eventual LSP forever.
  Reusing ideas from the Cixfile parser does not make those costs disappear.
- Indentation now has meaning even though Cixfile v1 is mostly flat. Exact
  two-space indentation avoids YAML ambiguity by turning deviations into
  errors, but it is still a new family rule.
- Whitespace-token syntax ages badly when values need spaces, empty strings,
  lists with annotations, or future structured policy. Quoting then becomes a
  second sublanguage.
- Repetition is low because `TALKS-TO` is positional. That also makes large
  lines harder for schema-aware completion and makes argument-order mistakes
  parser concerns rather than labeled-field errors.
- Reuse stops at named `LIMITS`, exactly as it should for now. There is no
  principled route to richer reuse except more directives, includes, or a
  macro language.
- Similar-looking `SERVICE` blocks do different jobs in Cixfile and
  Cixcompose: one declares an app contract, the other selects and overrides
  one. The syntax-family story can conceal that boundary as easily as it can
  teach it.
- Docker Compose migration gets no structural help. Familiar uppercase
  directives resemble Dockerfile, not Compose, and generic YAML tooling is
  lost.

## Comparison

Scores are 1 (actively bad) through 5 (strong). They are equal-weight prompts
for argument, not procurement arithmetic. A format does not win by hiding a
severe safety problem behind several cosmetic fives.

| Criterion | TOML | YAML | nix-lite | Cixfile sibling |
| --- | ---: | ---: | ---: | ---: |
| References/reuse | 3 | 2 | 2 | 3 |
| Tag refs + lock interaction | 4 | 4 | 4 | 5 |
| Diffability + code review | 4 | 2 | 4 | 5 |
| Comments | 4 | 4 | 4 | 3 |
| Schema validation story | 4 | 3 | 3 | 3 |
| Spec overrides | 3 | 4 | 4 | 4 |
| Networks + `talks-to` | 3 | 4 | 4 | 5 |
| Docker Compose migration | 3 | 5 | 1 | 2 |
| Tooling cost for composix | 4 | 4 | 2 | 1 |
| Concrete footgun resistance | 4 | 1 | 1 | 3 |
| **Unweighted total / 50** | **36** | **33** | **29** | **34** |

### References and reuse

None of the formats gets arbitrary service inheritance in this proposal.
That is deliberate. Scenario 3 proves a need for reusable limit policy, so
`limits` is a named domain object. The same approach can later cover restart or
rollout policy if repeated real configurations justify it.

TOML and the directive DSL make these references boring strings/tokens. YAML
can additionally alias syntax nodes, which is the wrong level: an anchor can
copy half a service without the schema knowing that the copy is policy. The
merge-key extension makes precedence worse. nix-lite has been stripped of the
structural operations that make Nix good at reuse, so its score must reflect
the restricted language actually proposed, not full Nix.

No candidate supports includes or cross-file templates. Adding either before a
real need would split the reproducibility boundary between a compose file and
an untracked input graph.

### Refs, updates, and `cix.lock`

All four make `name:tag#service`, `hold`, and `watch` legible. The directive
format wins narrowly because the ref and update policy are adjacent and
visually dominant. In mapping formats a formatter cannot guarantee that
adjacency.

The lock behavior is identical and format-independent. `cix resolve` records,
per system, each source ref, selected item service, store path, `narHash`,
substituters/trusted keys, and resolution time. Network IPAM allocations also
need persisted state per D26, but should be a distinct lock section so a tag
update does not masquerade as a network change. `hold` changes only through an
explicit update; `watch` proposes a new complete lock and activates only after
the full build succeeds. The compose source never accepts a `narHash` that
purports to be its own lock.

The fixed-port collision is a lock-time semantic check over resolved specs.
Syntax validation cannot catch it. The selected metrics variant proves that the
operator decision is an item/ref change, not a prettier spelling for an
illegal override.

### Review quality and comments

The directive file is the fastest to review for topology: one `TALKS-TO` per
line, no repeated labels. TOML is verbose but stable. nix-lite has strong
delimiters and a canonical formatter, but reviewers must remember that
apparently ordinary Nix may be forbidden. YAML is last: indentation carries
ownership, flow and block styles create gratuitous alternatives, and there is
no universally expected formatter.

All candidates retain comments in source and discard them from evaluated data.
TOML, YAML, and Nix allow trailing comments. The directive grammar intentionally
does not, because `#` is already meaningful inside `ref#service`; full-line
comments are adequate but less convenient.

### Validation

There are three validation layers regardless of syntax:

1. parse with duplicate/unknown construct rejection and source spans;
2. validate the unresolved compose schema and its internal references;
3. resolve items, validate overrides and credentials against each selected
   selected spec, detect shared-namespace collisions, and validate enforcement
   feasibility.

TOML has the cleanest implementation path: strict deserialization for the CLI,
the same model exported as JSON Schema for Taplo, then one semantic validator.
YAML can reuse that model only after locking the parser to YAML 1.2 core schema,
rejecting duplicate keys, aliases/tags, merge keys, and multi-document streams.
At that point we are selling "YAML, except the YAML you already have."

nix-lite must validate its AST before evaluation and retain an AST-to-output
source map, or post-evaluation errors degrade to unhelpful JSON paths. A generic
Nix LSP does not know the whitelist. The directive DSL can produce excellent
domain errors, but every editor/schema facility is bespoke.

JSON Schema is useful for shape and completion, not a substitute for resolution.
It cannot prove that `dashboard.http` exists in a fetched spec, that
`DATABASE_URL` is declared secret, or that two fixed ports collide.

### How overrides and networking read

YAML and nix-lite render nested overrides most naturally. TOML's quoted
directory-path keys and header changes are clumsy. The directive form is
explicit but positional; `OVERRIDE DIR app-path = host-path` is excellent until
that statement accumulates six optional policies.

For topology the order reverses. `TALKS-TO api -> db VIA network.backend` is
better than any record rendering. YAML is acceptable. nix-lite is long but
bounded by braces. TOML's array-of-table graph is punitive.

That does not justify changing semantics by format. In every candidate:

- membership says which named network subnets a service may access;
- `talks-to` says which dependency is intended;
- a socket edge is enforceable per service;
- an intra-composite IP edge may compile only to composite-address ACLs and
  must be reported as coarse;
- a cross-composite edge targets a named published port, never another
  composite's private loopback;
- `depends_on` is absent because reachability and startup order are different
  facts.

### Migration and tooling

YAML is the only credible migration leader. A converter can map Docker Compose
services/networks/volumes into a draft and emit hard failures for `build`,
`command`, undeclared environment, health/order semantics, and Docker socket
mounts that have no honest composix equivalent. Familiar layout reduces the
first-reading cost.

That advantage is not enough to choose YAML as the source of truth. A migration
tool can read YAML and write TOML; the permanent format need not preserve the
incumbent's parser hazards. TOML gets mature Rust parsing, formatting, and
Taplo at low cost. YAML tooling is broad but loader configuration becomes part
of the security contract. nix-lite adds an AST gate and misleading generic Nix
tooling. The directive language leaves the whole toolchain with us.

### Footguns, without euphemism

- **TOML:** table context can be far above a key; arrays of tables are noisy;
  inline tables cannot span lines; bare dates/times acquire types; dotted and
  quoted keys are easy to mix up. These are annoying and testable, not
  loader-folklore semantics.
- **YAML:** the Norway problem (`NO`), `on`/`off` booleans, implicit timestamps,
  loader-dependent duplicate keys, merge-key precedence, alias expansion,
  indentation ownership, tag constructors in unsafe loaders, and `:`/`#`
  plain-scalar traps. "Use quotes carefully" is not a language design.
- **nix-lite:** it advertises Nix and rejects Nix; generic tools suggest invalid
  constructs; a whitelist miss can expose imports or path/string-context
  behavior; relaxing the subset invites functions-as-API, overlays, and module
  fixpoints. The subset is socially unstable even if its parser is perfect.
- **Cixfile sibling:** positional arguments, eventual quoting pressure,
  bespoke-tooling drift, directive proliferation, and the same-looking
  `SERVICE` keyword on opposite sides of the spec/compose boundary. The grammar
  is controllable; its growth pressure is not.

## Recommendation

Choose **TOML** for the first compose surface and call the file
`compose.toml`. Keep it deliberately data-only:

- no includes, anchors, interpolation, profiles other than named domain
  objects, or environment-variable substitution;
- strict unknown/duplicate-key rejection and source-spanned errors;
- unit-bearing values represented as strings and normalized by schema;
- generated `cix.lock` kept beside the source and never edited through TOML;
- one resolver/semantic validator shared with any future input format;
- `cix fmt`, Taplo schema metadata, and `cix compose graph` available from the
  first usable release.

TOML does not make scenario 3 pretty. That is the honest trade: the syntax is
boring, its parser is unsurprising, and complexity remains visible instead of
escaping into anchors, expressions, or a language implementation. Composition
already has hard semantics—mutable refs, capabilities, credentials, shared
namespaces, enforcement tiers, and atomic activation. It does not need an
additional clever subsystem.

The **Cixfile-sibling DSL is the runner-up**. Its topology and update-policy
review experience is plainly best, and a syntax-family story has product value.
Do not build it now. Preserve these examples as the challenger and test them
against real operator edits after a TOML implementation exists. If repeated
blind review tests on at least five real composites show materially fewer
wrong-edge, wrong-owner, or missed-update-policy errors with directives—and
the maintenance estimate includes formatter and editor support—that evidence
would change the recommendation.

YAML would become preferable only if source-level Docker Compose compatibility,
not migration, becomes a primary product requirement and composix is willing
to define and enforce a hostile strict subset. nix-lite would become preferable
only if the target audience becomes explicitly Nix-native and asks for actual
Nix reuse; in that case the honest answer is a full `.nix` escape hatch, not
this crippled dialect.

No fifth candidate is added. KDL would trade TOML's mature schema/tooling path
for nicer nested records without solving reuse. CUE would add a powerful
constraint/programming system precisely when this prototype argues to keep
compose as validated data. Either deserves a new prototype only after real
composites demonstrate a requirement the four candidates cannot express.

---

## Addendum (2026-07-29): the question, dissolved — D28

After this document was written, the format question was reframed by Mathijs and decided as
D28 (see `design.md`): compose's canonical form is **machine-format `compose.json`**; human
authoring is a generator concern (config-as-code in any language against the published
schema), with hand-written JSON fine for trivial composites. At production complexity,
compose config is program output, not a document — the working precedent is a real fleet
config in Python computing placements and rendering gitignored artifacts, regenerated before
every command.

This document remains the *encoding archive*: its TOML recommendation and the Cixfile-DSL
challenger are now candidate **sugar encodings** over the same schema, evidence-gated on
people demonstrably hand-writing composites at scale. Its scenario definitions and the
adversarial criteria remain the test-bed for any such encoding — and for the JSON Schema
itself.
