# The wild corpus — real Dockerfiles, compose files, and k8s shapes, triaged

Status: surveyed 2026-07-30; regraded 2026-08-02 after the D47/D74 and
CIP-75/76/80/82/83 implementation wave. Grading is maintained per-track from
this sweep onward. Ledger-style sibling of docs/docker.md: docker.md maps
*features*, this maps *real artifacts*. **[Browse every checked-in migration
side by side](corpus/index.html)**: upstream on the left, Cixfile/Nix and
compose artifacts on the right, with the receipt scope attached.

> **Honesty caveat (Mathijs's review): the Evidence column distinguishes a
> reading-based `desk` grade from a `receipt` produced in the cited round. A
> ✅ desk grade is still a mechanism claim, not a successful port. Receipts prove
> only the behavior they name, not untested configuration or version parity: a
> Dockerfile-pinned release and the package selected by pinned nixpkgs may differ.**

Ribbons:

- ✅ expressible today (with which mechanism)
- 🔶 expressible with a known workaround / honest loss (stated)
- ⏳ blocked on a recorded-but-unbuilt decision (D-number)
- ❌ outside the thesis (with the honest why)
- Effort = S/M/L/XL: Cixfile+compose lines plus thinking, for a competent adopter.

## The living migration corpus (20)

These are the checked-in conversions under `corpus/migrate/`, not a second
historical grade set. The ribbon describes the whole conversion represented by
the receipt; “receipt” still proves only the named build/runtime probe. Open any
case in the [side-by-side browser](corpus/index.html).

| # | Case | What the receipt establishes | Ribbon | Effort | Evidence |
|---:|---|---|---|:---:|---|
| 1 | Adminer | Cix build and login-page probe pass; dynamic design/plugin entrypoint behavior is outside the probe | 🔶 base service passes; optional runtime mutation remains unconverted | S | [receipt](../corpus/migrate/adminer/receipt.md) |
| 2 | Caddy | Cix build and HTTP probe pass | ✅ package selection + direct service contract | S | [receipt](../corpus/migrate/caddy/receipt.md) |
| 3 | Directus | Docker passes; Cix reaches an FHS-loader-dependent native binary and fails the build | 🔶 blocked on the native-binary/FHS loader gap | M | [receipt](../corpus/migrate/directus/receipt.md) |
| 4 | Dozzle | Conversion evidence records unstable fetch output; the runtime contract also requires Docker’s socket/API | ❌ Docker-control-plane workload; build receipt is non-green | M | [receipt](../corpus/migrate/dozzle/receipt.md) |
| 5 | Echo Server | Cix offline build and bounded HTTP probe pass | ✅ named builders + pinned package-manager fetch | M | [receipt](../corpus/migrate/echo-server/receipt.md) |
| 6 | Excalidraw | Docker and Cix HTTP probes pass; declarative health remains absent | 🔶 runtime passes; health awaits CIP-79 implementation | M | [receipt](../corpus/migrate/excalidraw/receipt.md) |
| 7 | Filestash | Docker passes; Cix stops at unavailable static-library linkage | 🔶 package/static-link composition gap | L | [receipt](../corpus/migrate/filestash/receipt.md) |
| 8 | Memcached | Cix build and protocol version probe pass | ✅ package selection + direct service contract | S | [receipt](../corpus/migrate/memcached/receipt.md) |
| 9 | NATS | Cix build and monitoring health probe pass | ✅ package selection + direct service contract | S | [receipt](../corpus/migrate/nats/receipt.md) |
| 10 | nginx | Cix build, HTTP request, and indexed journald query pass | ✅ package selection + native logging projection | S | [receipt](../corpus/migrate/nginx/receipt.md) |
| 11 | Parse Server | Stable pinned Cix build passes; Mongo-backed runtime was not exercised | 🔶 build-only receipt; runtime/config path remains open | M | [receipt](../corpus/migrate/parse-server/receipt.md) |
| 12 | phpMyAdmin | Cix build and login-page probe pass | ✅ package selection + direct service contract | S | [receipt](../corpus/migrate/phpmyadmin/receipt.md) |
| 13 | Redis | Cix build, direct `/data` state projection, and PING pass | ✅ package selection + arbitrary-path state | S | [receipt](../corpus/migrate/redis/receipt.md) |
| 14 | Renovate | Cix timer activation, execution, and indexed logs pass; token/config delivery is unconverted | 🔶 schedule/logging pass; credentials await CIP-81 implementation | S | [receipt](../corpus/migrate/renovate/receipt.md) |
| 15 | Tomcat | Cix build and bounded HTTP reachability probe pass | ✅ complete package tree + idempotent setup | S | [receipt](../corpus/migrate/tomcat/receipt.md) |
| 16 | Traefik | Cix build and ping endpoint pass | ✅ package selection + direct service contract | S | [receipt](../corpus/migrate/traefik/receipt.md) |
| 17 | Verdaccio | Cix Corepack/pnpm build sequence fails before producing an item | 🔶 package-manager build remains non-green | M | [receipt](../corpus/migrate/verdaccio/receipt.md) |
| 18 | Wallos | Docker and Cix health probes pass through the Cixfile overlay universe; declarative readiness remains untranslated | 🔶 runtime passes; READINESS remains undeclared | M | [receipt](../corpus/migrate/wallos/receipt.md) |
| 19 | Watchtower | Go build succeeds, but faithful runtime requires Docker’s socket/API | ❌ competing Docker control plane | M | [receipt](../corpus/migrate/watchtower/receipt.md) |
| 20 | Whoami | Cix build and bounded HTTP probe pass | ✅ named Go builder + direct service contract | M | [receipt](../corpus/migrate/whoami/receipt.md) |

## 1. Compose files in the wild (18)

| # | Stack (source) | What it actually needs | Ribbon | Effort | Evidence |
|---|---|---|---|---|---|
| 1 | [wordpress-mysql](https://raw.githubusercontent.com/docker/awesome-compose/master/wordpress-mysql/compose.yaml) | 2 svcs, named volume, ports, env | 🔶 MySQL maps to private state + ports; WordPress's mutable `/var/www/html` volume still needs the immutable-app/state restructure from Dockerfile row 16 | S | desk |
| 2 | [prometheus-grafana](https://raw.githubusercontent.com/docker/awesome-compose/master/prometheus-grafana/compose.yaml) | named vol + relative config binds | 🔶 private roles plus compose `host:`/`.env` containment cover the storage shape; relative source/config conversion remains a migration exercise | S | desk |
| 3 | [flask-redis](https://raw.githubusercontent.com/docker/awesome-compose/master/flask-redis/compose.yaml) | build: + live source bind (dev loop) | 🔶 D47 build + `cix watch` warm rebuild/restart are built; source sync is deliberately refused, so framework hot reload stays in `nix develop` | M | desk |
| 4 | [react-express-mongodb](https://raw.githubusercontent.com/docker/awesome-compose/master/react-express-mongodb/compose.yaml) | 2 networks (frontend can't see db), anon-volume masking node_modules | segmentation ⏳ D26/D27; volume-masking ❌ idiom (restructure honestly) | M | desk |
| 5 | [Gitea](https://docs.gitea.com/installation/install-with-docker) | 1 svc, data dir, ports incl. SSH 222:22 | ✅ private state dir + declared ports | S | desk |
| 6 | [Umami](https://raw.githubusercontent.com/umami-software/umami/master/docker-compose.yml) | healthcheck-gated startup, init: | 🔁 HTTP/TCP `READINESS`/`LIVENESS` and structural readiness ordering are built; the separate condition graph is deliberately ❌; init ceremony dissolves under systemd | S | desk |
| 7 | [Immich](https://raw.githubusercontent.com/immich-app/immich/main/docker/docker-compose.yml) | env-interpolated bind paths, image healthchecks, shm_size, GPU overlays | 🔶 own-directory `.env`, static-identity `host:`, probes, and `SHM` are built; `CLAIM gpu` still proves unit properties only—not Immich/NVIDIA integration | M | desk |
| 8 | [Paperless-ngx](https://raw.githubusercontent.com/paperless-ngx/paperless-ngx/main/docker/compose/docker-compose.postgres.yml) | consume watch-dir (host-shared rw bind), variant compose files | 🔶 `DIR /consume` plus static-identity read-write `host:` materialization cover the watch-dir; variants ≈ D46 parametric | M | desk |
| 9 | [Mastodon](https://raw.githubusercontent.com/mastodon/mastodon/main/docker-compose.yml) | `internal: true` no-egress net; rw dir **shared between web+sidekiq**; 127.0.0.1 binds; 5-svc health DAG | 🔶 hermetic `shared:` covers the rw surface; `CLAIM egress` and readiness/liveness are built, but the health DAG and D26/D27 segmentation remain | M | desk |
| 10 | [Penpot](https://raw.githubusercontent.com/penpot/penpot/main/docker/images/docker-compose.yaml) | YAML anchors (preprocessing), shared-rw assets volume | 🔶 anchors are moot (JSON canonical, D28) and `shared:` now covers the asset surface; full migration remains a desk claim | M | desk |
| 11 | [Plausible CE](https://raw.githubusercontent.com/plausible/community-edition/master/compose.yml) | ulimits, migrate-then-run chains | 🔶 `START_PRE` maps the setup chain; `LimitNOFILE` is native systemd operator policy but has no compose field | S | desk |
| 12 | [Authentik](https://goauthentik.io/docker-compose.yml) | normal stack + worker mounting **docker.sock** to manage outposts | stack ✅; socket-worker ❌ (imperatively orchestrates siblings — competing model; our answer is cix's own surface) | M | desk |
| 13 | [Pi-hole](https://raw.githubusercontent.com/pi-hole/docker-pi-hole/master/README.md) | cap NET_ADMIN/SYS_TIME/SYS_NICE, port 53, DHCP/NTP | 🔶 port declaration works, but those raw capabilities and host mutations require explicit operator policy; no native semantic claim yet | M | desk |
| 14 | [Supabase](https://raw.githubusercontent.com/supabase/supabase/master/docker/docker-compose.yml) | 12-svc health-conditioned DAG, 20+ binds, :z flags | 🔶 native probes, structural readiness ordering, and static-identity `host:` binds are built; the condition graph is deliberately ❌ and `:z` is SELinux/operator policy | L | desk |
| 15 | [Sentry self-hosted](https://raw.githubusercontent.com/getsentry/self-hosted/master/docker-compose.yml) | ~60 svcs, profiles, external volumes, installer-driven | profiles ≈ D46 parametric ⏳; compose-as-installer-backend ❌-leaning honest | XL | desk |
| 16 | [Nextcloud AIO](https://raw.githubusercontent.com/nextcloud/all-in-one/main/compose.yaml) | mastercontainer spawns ~10 siblings via docker.sock | ❌ — the file describes 10% of the deployment; competing orchestration model | — | desk |
| 17 | [Frigate](https://docs.frigate.video/frigate/installation) | privileged, 5 device passthroughs, sized tmpfs, shm | devices 🔶 `CLAIM device` + closed `DevicePolicy=` dogfooded on a VM node, no privileged widening; `SHM` ✅; not a full Frigate app verification | L | desk |
| 18 | [Home Assistant](https://www.home-assistant.io/installation/linux) | host network, privileged, dbus, USB | 🔶 host networking is possible, but broad privilege, D-Bus, and USB access remain operator/device-policy work | M | desk |

**Frequency signals (18 files):** restart policy 17, named volumes 13, bind mounts 14,
healthchecks 10 + condition-gated depends_on 6, .env/interpolation ~6, named networks 6
(multi-net 2, internal:true 1), shm_size 5, docker.sock 3, privileged 2, compose
`secrets:` **0**, `deploy/replicas` **0**. The wild ignores the features docker-compose
has on paper (secrets, replicas — validating D30's deferrals) and leans hard on the ones
whose design now has implementation behind it (CIP-79 health), plus the still-unfinished
CIP-82 operator materialization.

## 2. Kubernetes shapes in the wild (15)

| # | Shape (source) | Essential semantics | Ribbon | Effort | Evidence |
|---|---|---|---|---|---|
| 1 | [minimal nginx Deployment](https://raw.githubusercontent.com/kubernetes/website/main/content/en/examples/application/deployment.yaml) | run N copies on port 80 | ✅ one copy; replicas remain ⏳ CIP-85 implementation | S | desk |
| 2 | [Guestbook](https://raw.githubusercontent.com/kubernetes/examples/master/web/guestbook/frontend-deployment.yaml) | stateless tier finds redis via DNS | ⏳ the services are packageable, but TCP discovery/DNS remains networking work; probes await CIP-79 and limits await resource policy | S | desk |
| 3 | [Bitnami WordPress chart](https://raw.githubusercontent.com/bitnami/charts/main/bitnami/wordpress/templates/deployment.yaml) | PHP + writable dir + DB password + hostname | 🔶 arbitrary-path role dirs dissolve init-chown ceremony; DB discovery and credential delivery remain queued | M | desk |
| 4 | [Bitnami PostgreSQL chart](https://raw.githubusercontent.com/bitnami/charts/main/bitnami/postgresql/templates/primary/statefulset.yaml) | one postgres, stable identity, durable volume, password | 🔶 most permission ceremony dissolves into DynamicUser + `STATEDIR`; password delivery remains queued after CIP-81 | S | desk |
| 5 | [Bitnami Redis chart](https://raw.githubusercontent.com/bitnami/charts/main/bitnami/redis/templates/master/application.yaml) | redis + volume + password + "only my clients connect" | NetworkPolicy → ⏳ D27 talks-to; privileged init-sysctl = host prerequisite, honestly operator-side | M | desk |
| 6 | [Grafana chart](https://raw.githubusercontent.com/grafana-community/helm-charts/main/charts/grafana/templates/_pod.tpl) | grafana + config file + admin secret | 🔶 config-in-item + repin dissolves the reload sidecars; admin credential delivery remains queued | M | desk |
| 7 | [Prometheus operator CR](https://raw.githubusercontent.com/prometheus-community/helm-charts/main/charts/kube-prometheus-stack/templates/prometheus/prometheus.yaml) | scrape-what-selectors-say, N days on disk | ⏳ far — operator model ≈ reconciler + D46 parametric; the CR *interface style* is the lesson | L | desk |
| 8 | [node-exporter DaemonSet](https://raw.githubusercontent.com/prometheus-community/helm-charts/main/charts/prometheus-node-exporter/templates/daemonset.yaml) | one observer per node reading /proc,/sys | ⏳ operator `DIR :ro` declarations exist, but host materialization is queued; per-node = per-host root | S | desk |
| 9 | [ingress-nginx](https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/charts/ingress-nginx/templates/controller-deployment.yaml) | control loop watching cluster API | ❌ different world — our host edge is binds/publishes, not an API-watching proxy; HPA/PDB n/a | — | desk |
| 10 | [Istio Bookinfo](https://raw.githubusercontent.com/istio/istio/master/samples/bookinfo/platform/kube/bookinfo.yaml) | 4 microservices with identities; mesh by injection | services ✅; pod/tree colocation ⏳ CIP-85/86 implementation; mesh policy ⏳ D27; injection-as-mechanism ❌ (we declare) | M | desk |
| 11 | [Renovate CronJob](https://raw.githubusercontent.com/renovatebot/helm-charts/main/charts/renovate/templates/cronjob.yaml) | run batch on schedule with token + config | 🔶 APP `schedule`/persistent timer and indexed logs pass; token/config delivery was not converted | S | [receipt (timer/log only)](../corpus/migrate/renovate/receipt.md) |
| 12 | [Airflow scheduler](https://raw.githubusercontent.com/apache/airflow/main/chart/templates/scheduler/scheduler-deployment.yaml) | don't start before DB schema; synced DAG dir | wait-init → built `START_PRE`/ordering 🔶; git-sync sidecar vs repin remains a content-model mismatch | M | desk |
| 13 | [Airflow migrate Job](https://raw.githubusercontent.com/apache/airflow/main/chart/templates/jobs/migrate-database-job.yaml) | run migration once per upgrade | 🔶 built `START_PRE` covers cold start; migrate-on-upgrade hook remains a design question | M | desk |
| 14 | [cert-manager](https://raw.githubusercontent.com/cert-manager/cert-manager/master/deploy/charts/cert-manager/templates/deployment.yaml) | three control loops + CRDs, no "app" at all | ❌ different world; the *need* (cert provisioning) returns later via credentials story | — | desk |
| 15 | [CloudNativePG Cluster CR](https://raw.githubusercontent.com/cloudnative-pg/cloudnative-pg/main/docs/src/samples/cluster-example-full.yaml) | "HA postgres, size X, these params, backup to S3" | the operator absorbs 100% of row-4's ceremony — same app at the opposite mechanism extreme; composix's pitch is that interface *without* a cluster | — | desk |

## 3. Dockerfiles in the wild (18)

| # | Image (source) | What the build actually does | Ribbon | Effort | Evidence |
|---|---|---|---|---|---|
| 1 | [postgres official](https://github.com/docker-library/postgres/blob/master/17/bookworm/Dockerfile) | apt + arch-conditional source compile + gosu/uid-999/locale surgery | ✅ dissolves — nixpkgs ships postgres; the file becomes a package selection + service contract | S | desk |
| 2 | [redis official](https://github.com/docker-library/redis/blob/master/7.4/debian/Dockerfile) | source compile + sha256 tarball + gosu | ✅ nixpkgs Redis + direct `STATEDIR /data`; PING passes | S | [receipt](../corpus/migrate/redis/receipt.md) |
| 3 | [nginx official](https://github.com/nginx/docker-nginx/blob/master/mainline/debian/Dockerfile) | pinned vendor apt + GPG + stdout log symlinks | ✅ dissolves; stdout/stderr go to indexed journald, so log symlinks are unnecessary | S | [receipt](../corpus/migrate/nginx/receipt.md) |
| 4 | [node official](https://github.com/nodejs/docker-node/blob/main/24/bookworm-slim/Dockerfile) | verified binary download + unpack | ✅ dissolves (nixpkgs nodejs) | S | desk |
| 5 | [python official](https://github.com/docker-library/python/blob/master/3.13/slim-bookworm/Dockerfile) | CPython from source + GPG | ✅ dissolves | S | desk |
| 6 | [Next.js app](https://github.com/vercel/next.js/blob/canary/examples/with-docker/Dockerfile) | 3-stage, npm cache mounts, COPY --from standalone | ✅ D47 named builders + persistent underlay + binder COPY | M | desk |
| 7 | [gitea](https://github.com/go-gitea/gitea/blob/main/Dockerfile) | go+pnpm compile, cache mounts, .git bind for version stamp | ✅ D47; version-stamp bind 🔶 (COPY it, or a build-arg story ❓) | M | desk |
| 8 | [vaultwarden](https://github.com/dani-garcia/vaultwarden/blob/main/docker/Dockerfile.debian) | rust + digest-pinned FROMs + xx cross-compile | build ✅; digest-pinned FROM = locks by default; cross-compile 🔶 (nix cross exists, no cix surface) | M | desk |
| 9 | [uv app (astral)](https://github.com/astral-sh/uv-docker-example/blob/main/Dockerfile) | pure uv fetch with cache+lockfile bind mounts | ✅ the D38 spike proved this ecosystem shape | S | desk |
| 10 | [jenkins](https://github.com/jenkinsci/docker/blob/master/debian/Dockerfile) | war download + GPG + tini + uid surgery | ✅ FETCH + item; tini/uid ceremony dissolves | S | desk |
| 11 | [playwright](https://github.com/microsoft/playwright/blob/main/utils/docker/Dockerfile.noble) | huge apt + browser binaries + `--mount=type=secret` npmrc + chmod 777 | 🔶 nixpkgs browsers replace the install path; build secrets remain a gap | L | desk |
| 12 | [pytorch](https://github.com/pytorch/pytorch/blob/main/Dockerfile) | 6-stage, CUDA gated by ARG, unverified NVIDIA key | 🔶 nixpkgs-cuda works but unfree/impure pain is real; FETCH would force the missing pin | L | desk |
| 13 | [airflow](https://github.com/apache/airflow/blob/main/Dockerfile) | 50+ ARGs, interpreter compiles, uid-50000-gid-0 OpenShift trick | 🔶 by volume; most dissolves into nixpkgs + D47, the ARG matrix ≈ D46 parametric | L | desk |
| 14 | [php official](https://github.com/docker-library/php/blob/master/8.4/bookworm/cli/Dockerfile) | pages of ./configure + apt-mark dance | ✅ dissolves (nixpkgs php + extensions) | S | desk |
| 15 | [grafana](https://github.com/grafana/grafana/blob/main/Dockerfile) | 17 FROM lines, go+yarn compile, unverified glibc curl, chmod 777 | ✅ D47 for the build; the variant matrix ≈ D46; 777s dissolve | M | desk |
| 16 | [wordpress official](https://github.com/docker-library/wordpress/blob/master/latest/php8.4/apache/Dockerfile) | ext compile + **entrypoint copies app into volume + chowns at runtime** | runtime fs surgery ❌ as-is; restructure: immutable app + arbitrary-path role state 🔶 | M | desk |
| 17 | [keycloak](https://github.com/keycloak/keycloak/blob/main/quarkus/container/Dockerfile) | unpack prebuilt dist (ADD, no checksum) + g+rwX | ✅ FETCH (pin forced — an upgrade over the original) + item | S | desk |
| 18 | [traefik scratch](https://github.com/traefik/traefik-library-image/blob/master/v3.5/scratch/Dockerfile) | pure assembly of static binary + certs + tzdata | ✅ dissolves — this Dockerfile is yearning to be a nix closure | S | desk |

**Build-side signals:** the *official-image class dissolves* — its work (compile the
interpreter, pin the tarball, create the uid, wire gosu/tini, symlink logs) is exactly
what nixpkgs + DynamicUser + systemd + journald already do; migrating these means
porting runtime config, not the build. The *modern app-build class maps 1:1 onto
D47/D71/CIP-87*: multi-stage → named builders, `COPY --from` → binder-rooted COPY,
`--mount=type=cache` (in every modern build, in zero official images) → the persistent
builder underlay plus traced early cutoff (with `--cold` as the clean read/output audit),
checksummed curl → FETCH (and the three unverified downloads in the corpus — pytorch,
grafana, keycloak — would be *forced* honest by FETCH's TOFU pin). Universal
gosu/su-exec/tini/uid-999 boilerplate: ceremony our substrate deletes.

## 4. Open gaps at a glance (ranked cross-corpus demands)

This is the short answer to “what is still open?” Rows are named as
`Compose n`, `Kubernetes n`, and `Dockerfile n` from the three tables above.
“Designed—unbuilt” means the model is settled in the cited CIP but the named
corpus rows still lack the implementation.

| Rank | Demand | Status | Rows blocked or proving it |
|---:|---|---|---|
| 1 | Health wiring | **Met ([CIP-79](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0079-health.md))** | Compose 6, 7, 9, 14; Kubernetes 2, 10 — the health VM scenario proves rollout gating, structural readiness ordering, and watchdog restart/recovery |
| 2 | Operator host-binds | **Met ([CIP-82](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0082-dirs.md))** | Compose 2, 7, 8, 14; dirs2 VM proves static-identity pre-existing host data survives purge |
| 3 | Shared-rw directories | **Met ([CIP-82](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0082-dirs.md))** | Compose 9, 10; dirs2 VM proves setgid group sharing |
| 4 | Timers / CronJob | **Met ([CIP-75](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0075-timers.md))** | Kubernetes 11; [Renovate receipt](../corpus/migrate/renovate/receipt.md) |
| 5 | Operational logs | **Met ([CIP-83](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0083-observability.md))** | Dockerfile 3; Kubernetes 11 |
| 6 | Artifact dev loop | **Met; source sync refused ([CIP-76](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0076-devloop.md))** | Compose 3 |
| 7 | Migrate-on-upgrade hook | **Designed—unbuilt ([CIP-75](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0075-timers.md), event-driven D48f leg)** | Compose 11; Kubernetes 12, 13 |
| 8 | Network segmentation and talks-to | **Designed—unbuilt ([CIP-86](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0086-netns.md))** | Compose 4, 9; Kubernetes 2, 5, 10 |
| 9 | Profiles and variants | **Designed—unbuilt ([CIP-85](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0085-compose-tree.md), D46 family surface)** | Compose 8, 15; Dockerfile 7, 12, 13, 15 |
| 10 | Build and runtime secrets | **Built (CIP-81; desk re-grade)** — host-consented FETCH credential files and compose `LoadCredential=` sources | Dockerfile 11; Kubernetes 3, 4, 6, 11 |
| 11 | Competing control planes and mutable overlay roots | **Refused** | Compose 12, 16; Kubernetes 9, 14; Dockerfile 16 |

The met rows have deliberate boundaries: timers accept native `OnCalendar`
rather than translating cron; logs stay in journald; `cix watch` always runs a
real rebuilt artifact; health probes are native notify/watchdog wiring and the
condition graph stays deliberately refused. The refused class is equally deliberate: Docker-socket
orchestration, API-watching controllers, injection meshes, and runtime-mutating
webroots are other control planes or filesystem models, not missing emulation.

## 5. Example candidates (borderline cases worth adopting into examples/)

- **Mastodon-shaped stack** remains the top integration candidate after CIP-79/82:
  readiness is now built, so it would validate queued `shared:` materialization plus
  network segmentation instead of forcing an unsettled health design.
- **Paperless-shaped ingest** is the smallest clean CIP-82 leg-2 gate: one writable
  operator watch-dir, one private state dir, and an observable import result.
- **Immich-shaped** is deferred to the in-flight CIP-78 devices track; it remains the
  combined host-dir/health/GPU stress case, not a regrade-track example.
- **Pi-hole** remains useful, but it is a gap-finder rather than a pure capability win:
  NET_ADMIN/SYS_TIME/SYS_NICE have no native semantic claims today.
- **Renovate-shaped cron batch** is retired as an example candidate: the focused
  [corpus receipt](../corpus/migrate/renovate/receipt.md) now covers timer generation,
  execution, and logs. A future full example would be about CIP-81 credentials, not
  timers.
- **Gitea-shaped build** (proj2 candidate): go+pnpm dual-ecosystem compile with cache
  mounts and a version-stamp — the heaviest realistic D47 exercise short of nasty.
- **Bitnami-postgres vs our postgres pack**: not a new example — a docs page putting
  the chart's 400 lines next to our Cixfile, per k8s rows 4/15. Pure showcase.

Adopt one candidate per track with its forcing function as the gate.
