# The wild corpus — real Dockerfiles, compose files, and k8s shapes, triaged

Status: survey 2026-07-30 (three research sweeps; sources linked per row). Ledger-style
sibling of docs/docker.md: docker.md maps *features*, this maps *real artifacts*.

> **Honesty caveat (Mathijs's review, same day): every ribbon below is a DESK grade —
> assigned from reading, not from converting and running. "✅ dissolves" is optimism
> until a Cixfile builds and passes a health check. The migrate-prompt track
> (.dev/specs/track-migrate.md) is the empirical re-analysis: its receipts confirm or
> refute these grades, and its loss numbers supersede this table's cheer. Those dual
> receipts prove functional faithfulness, not version parity: a Dockerfile-pinned release
> and the package selected by a pinned nixpkgs revision may differ.**

Ribbons:

- ✅ expressible today (with which mechanism)
- 🔶 expressible with a known workaround / honest loss (stated)
- ⏳ blocked on a recorded-but-unbuilt decision (D-number)
- ❌ outside the thesis (with the honest why)
- Effort = S/M/L/XL: Cixfile+compose lines plus thinking, for a competent adopter.

## 1. Compose files in the wild (18)

| # | Stack (source) | What it actually needs | Ribbon | Effort |
|---|---|---|---|---|
| 1 | [wordpress-mysql](https://raw.githubusercontent.com/docker/awesome-compose/master/wordpress-mysql/compose.yaml) | 2 svcs, named volume, ports, env | ✅ state dirs + ports | S |
| 2 | [prometheus-grafana](https://raw.githubusercontent.com/docker/awesome-compose/master/prometheus-grafana/compose.yaml) | named vol + relative config binds | ✅ / config binds 🔶 (bake into item; operator host-binds ⏳ compose) | S |
| 3 | [flask-redis](https://raw.githubusercontent.com/docker/awesome-compose/master/flask-redis/compose.yaml) | build: + live source bind (dev loop) | build ✅ (D47); live-reload dev bind ❌ deploy-side (docker.md `watch` ❓) | M |
| 4 | [react-express-mongodb](https://raw.githubusercontent.com/docker/awesome-compose/master/react-express-mongodb/compose.yaml) | 2 networks (frontend can't see db), anon-volume masking node_modules | segmentation ⏳ D26/D27; volume-masking ❌ idiom (restructure honestly) | M |
| 5 | [Gitea](https://docs.gitea.com/installation/install-with-docker) | 1 svc, data dir, ports incl. SSH 222:22 | ✅ | S |
| 6 | [Umami](https://raw.githubusercontent.com/umami-software/umami/master/docker-compose.yml) | healthcheck-gated startup, init: | ordering ✅; health wiring ⏳ D30-deferral; init free under systemd | S |
| 7 | [Immich](https://raw.githubusercontent.com/immich-app/immich/main/docker/docker-compose.yml) | env-interpolated bind paths, image healthchecks, shm_size, GPU overlays | binds ⏳ compose operator-binds; shm ✅ easier; GPU 🔶 devices | M |
| 8 | [Paperless-ngx](https://raw.githubusercontent.com/paperless-ngx/paperless-ngx/main/docker/compose/docker-compose.postgres.yml) | consume watch-dir (host-shared rw bind), variant compose files | watch-dir ⏳ operator-binds; variants ≈ D46 parametric | M |
| 9 | [Mastodon](https://raw.githubusercontent.com/mastodon/mastodon/main/docker-compose.yml) | `internal: true` no-egress net; rw dir **shared between web+sidekiq**; 127.0.0.1 binds; 5-svc health DAG | egress polarity ✅ designed (D43/D48b, ⏳ built); **shared-rw edge = real gap** (below); binds ✅ | M |
| 10 | [Penpot](https://raw.githubusercontent.com/penpot/penpot/main/docker/images/docker-compose.yaml) | YAML anchors (preprocessing), shared-rw assets volume | anchors ✅ moot (JSON canonical, D28); shared-rw same gap as #9 | M |
| 11 | [Plausible CE](https://raw.githubusercontent.com/plausible/community-edition/master/compose.yml) | ulimits, migrate-then-run chains | ✅ (`LimitNOFILE` easier; SETUP/ExecStartPre) | S |
| 12 | [Authentik](https://goauthentik.io/docker-compose.yml) | normal stack + worker mounting **docker.sock** to manage outposts | stack ✅; socket-worker ❌ (imperatively orchestrates siblings — competing model; our answer is cix's own surface) | M |
| 13 | [Pi-hole](https://raw.githubusercontent.com/pi-hole/docker-pi-hole/master/README.md) | cap NET_ADMIN/SYS_TIME/SYS_NICE, port 53, DHCP/NTP | caps ✅ AmbientCapabilities; port-53-vs-resolved = operator; DHCP/NTP host mutation 🔶 honest | M |
| 14 | [Supabase](https://raw.githubusercontent.com/supabase/supabase/master/docker/docker-compose.yml) | 12-svc health-conditioned DAG, 20+ binds, :z flags | ✅ by mechanism, L by volume; :z = SELinux, n/a-noted | L |
| 15 | [Sentry self-hosted](https://raw.githubusercontent.com/getsentry/self-hosted/master/docker-compose.yml) | ~60 svcs, profiles, external volumes, installer-driven | profiles ≈ D46 parametric ⏳; compose-as-installer-backend ❌-leaning honest | XL |
| 16 | [Nextcloud AIO](https://raw.githubusercontent.com/nextcloud/all-in-one/main/compose.yaml) | mastercontainer spawns ~10 siblings via docker.sock | ❌ — the file describes 10% of the deployment; competing orchestration model | — |
| 17 | [Frigate](https://docs.frigate.video/frigate/installation) | privileged, 5 device passthroughs, sized tmpfs, shm | devices 🔶 `DeviceAllow=` sans privileged; tmpfs/shm ✅ easier; hardening story fights the default | L |
| 18 | [Home Assistant](https://www.home-assistant.io/installation/linux) | host network, privileged, dbus, USB | 🔶 ironically easy: it *wants* to be a host service (rawdog = our default), near-zero isolation stated loudly | M |

**Frequency signals (18 files):** restart policy 17, named volumes 13, bind mounts 14,
healthchecks 10 + condition-gated depends_on 6, .env/interpolation ~6, named networks 6
(multi-net 2, internal:true 1), shm_size 5, docker.sock 3, privileged 2, compose
`secrets:` **0**, `deploy/replicas` **0**. The wild ignores the features docker-compose
has on paper (secrets, replicas — validating D30's deferrals) and leans hard on the ones
we deferred *with intent to build* (health wiring, operator binds).

## 2. Kubernetes shapes in the wild (15)

| # | Shape (source) | Essential semantics | Ribbon | Effort |
|---|---|---|---|---|
| 1 | [minimal nginx Deployment](https://raw.githubusercontent.com/kubernetes/website/main/content/en/examples/application/deployment.yaml) | run N copies on port 80 | ✅ minus replicas (⏳ D30-deferral; `replicas` = tree-node property per compose-tree) | S |
| 2 | [Guestbook](https://raw.githubusercontent.com/kubernetes/examples/master/web/guestbook/frontend-deployment.yaml) | stateless tier finds redis via DNS | ✅ edges/localhost instead of DNS; probes ⏳ D30-health; limits ⏳ resource-limits | S |
| 3 | [Bitnami WordPress chart](https://raw.githubusercontent.com/bitnami/charts/main/bitnami/wordpress/templates/deployment.yaml) | PHP + writable dir + DB password + hostname | ✅ core; init-chown ceremony **dissolves** (DynamicUser+state dirs own ownership); secrets ⏳ LoadCredential | M |
| 4 | [Bitnami PostgreSQL chart](https://raw.githubusercontent.com/bitnami/charts/main/bitnami/postgresql/templates/primary/statefulset.yaml) | one postgres, stable identity, durable volume, password | ✅ — ~90% of the template is permission/TLS/configurability ceremony our model doesn't need; showcase row | S |
| 5 | [Bitnami Redis chart](https://raw.githubusercontent.com/bitnami/charts/main/bitnami/redis/templates/master/application.yaml) | redis + volume + password + "only my clients connect" | NetworkPolicy → ⏳ D27 talks-to; privileged init-sysctl = host prerequisite, honestly operator-side (ocimport finding repeats) | M |
| 6 | [Grafana chart](https://raw.githubusercontent.com/grafana-community/helm-charts/main/charts/grafana/templates/_pod.tpl) | grafana + config file + admin secret | ✅ — the 5-sidecar hot-reload fleet emulates "config from objects"; ours: config in item, update = repin. Ceremony dissolves | M |
| 7 | [Prometheus operator CR](https://raw.githubusercontent.com/prometheus-community/helm-charts/main/charts/kube-prometheus-stack/templates/prometheus/prometheus.yaml) | scrape-what-selectors-say, N days on disk | ⏳ far — operator model ≈ reconciler + D46 parametric; the CR *interface style* is the lesson | L |
| 8 | [node-exporter DaemonSet](https://raw.githubusercontent.com/prometheus-community/helm-charts/main/charts/prometheus-node-exporter/templates/daemonset.yaml) | one observer per node reading /proc,/sys | 🔶 host mounts = `mounts` + rawdog; per-node = per-host root; DaemonSet-as-concept n/a (single host) | S |
| 9 | [ingress-nginx](https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/charts/ingress-nginx/templates/controller-deployment.yaml) | control loop watching cluster API | ❌ different world — our host edge is binds/publishes, not an API-watching proxy; HPA/PDB n/a | — |
| 10 | [Istio Bookinfo](https://raw.githubusercontent.com/istio/istio/master/samples/bookinfo/platform/kube/bookinfo.yaml) | 4 microservices with identities; mesh by injection | services ✅; explicit sidecars = pod members ✅ (tree); mesh policy ⏳ D27; injection-as-mechanism ❌ (we declare) | M |
| 11 | [Renovate CronJob](https://raw.githubusercontent.com/renovatebot/helm-charts/main/charts/renovate/templates/cronjob.yaml) | run batch on schedule with token + config | ✅ APP compose `schedule` maps raw `OnCalendar` to a paired systemd timer; use explicit persistent catch-up where wanted | S |
| 12 | [Airflow scheduler](https://raw.githubusercontent.com/apache/airflow/main/chart/templates/scheduler/scheduler-deployment.yaml) | don't start before DB schema; synced DAG dir | wait-init → SETUP/ordering 🔶; git-sync sidecar vs our repin model = interesting mismatch (content should be an item) | M |
| 13 | [Airflow migrate Job](https://raw.githubusercontent.com/apache/airflow/main/chart/templates/jobs/migrate-database-job.yaml) | run migration once per upgrade | 🔶 SETUP covers cold start; **migrate-on-upgrade hook** is a real design question ❓ (below) | M |
| 14 | [cert-manager](https://raw.githubusercontent.com/cert-manager/cert-manager/master/deploy/charts/cert-manager/templates/deployment.yaml) | three control loops + CRDs, no "app" at all | ❌ different world; the *need* (cert provisioning) returns later via credentials story | — |
| 15 | [CloudNativePG Cluster CR](https://raw.githubusercontent.com/cloudnative-pg/cloudnative-pg/main/docs/src/samples/cluster-example-full.yaml) | "HA postgres, size X, these params, backup to S3" | the operator absorbs 100% of row-4's ceremony — rows 4 vs 15 are the same app at opposite mechanism extremes; composix's pitch is that interface *without* a cluster | — |

## 3. Dockerfiles in the wild (18)

| # | Image (source) | What the build actually does | Ribbon | Effort |
|---|---|---|---|---|
| 1 | [postgres official](https://github.com/docker-library/postgres/blob/master/17/bookworm/Dockerfile) | apt + arch-conditional source compile + gosu/uid-999/locale surgery | ✅ **dissolves** — nixpkgs ships postgres; the whole file becomes `FROM … AS pkgs` + our pack (which exists) | S |
| 2 | [redis official](https://github.com/docker-library/redis/blob/master/7.4/debian/Dockerfile) | source compile + sha256 tarball + gosu | ✅ dissolves (nixpkgs redis; our pack exists) | S |
| 3 | [nginx official](https://github.com/nginx/docker-nginx/blob/master/mainline/debian/Dockerfile) | pinned vendor apt + GPG + stdout log symlinks | ✅ dissolves; log symlinks moot (journald native) | S |
| 4 | [node official](https://github.com/nodejs/docker-node/blob/main/24/bookworm-slim/Dockerfile) | verified binary download + unpack | ✅ dissolves (nixpkgs nodejs) | S |
| 5 | [python official](https://github.com/docker-library/python/blob/master/3.13/slim-bookworm/Dockerfile) | CPython from source + GPG | ✅ dissolves | S |
| 6 | [Next.js app](https://github.com/vercel/next.js/blob/canary/examples/with-docker/Dockerfile) | 3-stage, npm cache mounts, COPY --from standalone | ✅ **maps onto D47**: named builders + builder-local CACHE + binder COPY into an artifact | M |
| 7 | [gitea](https://github.com/go-gitea/gitea/blob/main/Dockerfile) | go+pnpm compile, cache mounts, .git bind for version stamp | ✅ D47; version-stamp bind 🔶 (COPY it, or a build-arg story ❓) | M |
| 8 | [vaultwarden](https://github.com/dani-garcia/vaultwarden/blob/main/docker/Dockerfile.debian) | rust + digest-pinned FROMs + xx cross-compile | build ✅; digest-pinned FROM = our locks *by default*; cross-compile 🔶 (nix cross exists, no cix surface; D14 per-system entries) | M |
| 9 | [uv app (astral)](https://github.com/astral-sh/uv-docker-example/blob/main/Dockerfile) | pure uv fetch with cache+lockfile bind mounts | ✅ the D38 spike literally proved this ecosystem | S |
| 10 | [jenkins](https://github.com/jenkinsci/docker/blob/master/debian/Dockerfile) | war download + GPG + tini + uid surgery | ✅ FETCH + item; tini/uid ceremony dissolves (systemd + DynamicUser) | S |
| 11 | [playwright](https://github.com/microsoft/playwright/blob/main/utils/docker/Dockerfile.noble) | huge apt + browser binaries + `--mount=type=secret` npmrc + chmod 777 | 🔶 nixpkgs browsers replace the install path; **build secrets ❓ gap** (below) | L |
| 12 | [pytorch](https://github.com/pytorch/pytorch/blob/main/Dockerfile) | 6-stage, CUDA gated by ARG, unverified NVIDIA key | 🔶 nixpkgs-cuda works but unfree/impure pain is real; FETCH would *force* the pin the Dockerfile skips | L |
| 13 | [airflow](https://github.com/apache/airflow/blob/main/Dockerfile) | 50+ ARGs, interpreter compiles, uid-50000-gid-0 OpenShift trick | 🔶 by volume; most of it dissolves into nixpkgs + D47, the ARG matrix ≈ D46 parametric | L |
| 14 | [php official](https://github.com/docker-library/php/blob/master/8.4/bookworm/cli/Dockerfile) | pages of ./configure + apt-mark dance | ✅ dissolves (nixpkgs php + extensions) | S |
| 15 | [grafana](https://github.com/grafana/grafana/blob/main/Dockerfile) | 17 FROM lines, go+yarn compile, unverified glibc curl, chmod 777 | ✅ D47 for the build; the variant matrix ≈ D46; 777s dissolve | M |
| 16 | [wordpress official](https://github.com/docker-library/wordpress/blob/master/latest/php8.4/apache/Dockerfile) | ext compile + **entrypoint copies app into volume + chowns at runtime** | runtime fs surgery ❌ as-is (the ocimport mutating-entrypoint class); restructure: app = item, wp-content = state dir 🔶 | M |
| 17 | [keycloak](https://github.com/keycloak/keycloak/blob/main/quarkus/container/Dockerfile) | unpack prebuilt dist (ADD, no checksum) + g+rwX | ✅ FETCH (pin forced — an *upgrade* over the original) + item | S |
| 18 | [traefik scratch](https://github.com/traefik/traefik-library-image/blob/master/v3.5/scratch/Dockerfile) | pure assembly of static binary + certs + tzdata | ✅ dissolves — this Dockerfile is yearning to be a nix closure | S |

**Build-side signals:** the *official-image class dissolves* — its work (compile the
interpreter, pin the tarball, create the uid, wire gosu/tini, symlink logs) is exactly
what nixpkgs + DynamicUser + systemd + journald already do; migrating these means
porting runtime config, not the build. The *modern app-build class maps 1:1 onto
D47*: multi-stage → named builders, `COPY --from` → binder-rooted COPY,
`--mount=type=cache` (in every modern build, in zero official images) → builder-local CACHE,
checksummed curl → FETCH (and the three unverified downloads in the corpus — pytorch,
grafana, keycloak — would be *forced* honest by FETCH's TOFU pin). Universal
gosu/su-exec/tini/uid-999 boilerplate: ceremony our substrate deletes.

## 4. What the wild demands (cross-corpus triage)

Ranked by frequency × how squarely it hits us:

1. **Health wiring** — 10/18 compose files + probes in essentially every k8s chart.
   D30 deferred it; the wild says it's the most-demanded deferral. → schedule early in
   the compose-tree wave (`health` exists in the manifest since v1; the missing part is
   compose *ordering/readiness* semantics).
2. **Operator host-binds** (env-interpolated paths, watch-dirs) — Immich, Paperless,
   Mastodon. Already ⏳ in the ledger; second-most demanded.
3. **Shared-rw directory between services** (Mastodon web+sidekiq, Penpot assets) —
   a genuine edge-model gap: today's edges are producer→consumers sockets/paths, not a
   shared writable surface. Design candidate: an edge variant exposing a common
   writable dir (per-edge group + idmapped ownership, the dstyle mechanism extended).
4. **Timers/CronJob** — ✅ CIP-75: an APP's compose `schedule` is raw systemd `OnCalendar`,
   backed by a paired timer. No cron translation layer or timer service kind.
5. **Migrate-on-upgrade hook** — helm hook-Jobs, Airflow. Our generation switch has a
   natural slot (between build and restart-changed). ❓ candidate; SETUP covers cold
   start today.
6. **Network segmentation & talks-to** — 6/18 named networks, internal:true, k8s
   NetworkPolicy. All lands on D26/D27 — already designed, sequenced after D43 pods.
7. **Profiles/variants** — Sentry profiles, Paperless variant files. D46 parametric
   composes cover the legitimate use; no new mechanism needed.
8. **Build secrets** — playwright's `--mount=type=secret` npmrc; private registries
   generally. FETCH has no secret story. ❓ candidate (a secret must reach the fetch
   without entering the memo key or the store).
9. **The ❌ class is coherent**: docker-socket orchestration (Authentik worker,
   Nextcloud AIO), API-watching controllers (ingress-nginx, cert-manager), injection
   meshes, runtime-mutating entrypoints (wordpress). All are *competing control planes
   or overlay-root assumptions*, not features — the honest answer is composix's own
   surface (reconciler, publishes, D27, items+state-dirs), never emulation.

## 5. Example candidates (borderline cases worth adopting into examples/)

- **Mastodon-shaped stack** (top pick): health DAG + internal-net/egress + the
  shared-rw gap in one 5-service, manageable package. Forces gaps #1 and #3.
- **Immich-shaped**: operator binds + image healthchecks + optional GPU — forces #2
  honestly, GPU as loud 🔶.
- **Pi-hole**: the capability showcase (NET_ADMIN + port 53) — pure win for the
  AmbientCapabilities story; small.
- **Renovate-shaped cron batch**: forces the timer design (#4); tiny surface.
- **Gitea-shaped build** (proj2 candidate): go+pnpm dual-ecosystem compile with cache
  mounts and a version-stamp — the heaviest realistic D47 exercise short of nasty.
- **Bitnami-postgres vs our postgres pack**: not a new example — a docs page putting
  the chart's 400 lines next to our Cixfile, per k8s rows 4/15. Pure showcase.

Adopt one candidate per track with its forcing function as the gate.
