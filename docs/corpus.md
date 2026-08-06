# The wild corpus — real Dockerfiles, compose files, and k8s shapes, triaged

Status: surveyed 2026-07-30; regraded 2026-08-04 from the per-case gap audit,
after the D47/D74 and CIP-75/76/80/82/83 implementation wave. Grading is
maintained per-track from this sweep onward. Ledger-style sibling of
docs/docker.md: docker.md maps
*features*, this maps *real artifacts*. **[Browse every checked-in migration
side by side](corpus/index.html)**: upstream on the left, Cixfile/Nix and
compose artifacts on the right, with the receipt scope attached.

Grading keys:

- **Living-corpus Fidelity:** ✅ faithful; 🔶 declared losses; ⏳ blocked before a
  faithful item/runtime; ❌ refused as outside the thesis. Every cell states the
  case-specific reason; the icon alone is never the claim.
- **Living-corpus Evidence:** `desk` (reading only), `build` (compiler/item
  result), `runtime probe` (the named behavior ran), or `closed-root` (the named
  behavior ran under the CIP-84 sealed root). A higher tier proves only the
  behavior named in that row; it does not promote fidelity or imply config/version
  parity.
- Sections 1–3 retain their survey ribbons without regrading: ✅ expressible,
  🔶 workaround or declared loss, ⏳ recorded-but-unbuilt, and ❌ outside the
  thesis.
- Effort = S/M/L/XL: Cixfile+compose lines plus thinking, for a competent adopter.

## How this corpus is maintained (the loops)

Adopted 2026-08-04 (Mathijs's corpus review). The corpus is human-consumable
first: today it is the dev-loop instrument, later it is adopter-facing
documentation. Clarity of the rendered page and cleanliness of every checked-in
file are requirements, not niceties.

Each case carries a `GAPS.md` next to its Cixfile: free-form markdown whose
only machine-read lines are the header pair

```
Generated: migrate.md@<commit> · <model> · <date>
Status: current            (or: stale — regenerate with <feature/CIP>)
```

Every gap is one prose bullet ending in an arrow that routes it to where the
fix lives: `→ case` (this conversion), `→ prompt` (docs/migrate.md),
`→ language (<CIP/draft>)`, `→ evidence` (receipt/reproducibility work),
`→ refused` (thesis boundary, stated), or `→ browser` (rendered-page clarity).
The vocabulary is deliberately open — the arrow is routing, not taxonomy;
invent a new target when none fits.

Three loops drain the ledger:

1. **Corpus → CIPs → features → corpus.** `→ language` gaps are promoted to
   `cips/draft/` entries citing their exhibiting cases. A track that lands a
   feature greps `corpus/migrate/docker/{docker,k8s}/*/GAPS.md` for its CIP/draft name and flips
   the exhibiting cases to `Status: stale — …` in the same track (this extends
   the ledger-currency rule in AGENTS.md). Stale cases form the regeneration
   queue.
2. **Prompt → corpus → prompt.** Regeneration is always cold: a fresh agent
   gets `docs/migrate.md` + the Dockerfile + its context, nothing else; the
   `Generated:` header records prompt version and model. A hand-edit to a
   canon Cixfile must carry, in the same commit, the migrate.md addendum that
   would have generated it — the one exception is an edit justified as
   "obsoleted by CIP-N implementation", which the staleness loop covers.
   Periodically, cold-regen a sample with 2–3 models and diff against canon:
   that measures whether the prompt teaches, and feeds both prompt edits and
   the model table.
3. **(Later) automation.** Once cold regeneration reliably matches canon, the
   prompt plus the paired `check.sh` probes are the regression suite for a
   `cix migrate` assistant. Recorded intent, deliberately unbuilt.

Verification tiers: CI parses every corpus Cixfile with the real parser (rot
guard); `check.sh {docker|cix}` receipts are rerun per-track when a case is
touched; the CIP-84 closed-root VM audits the green set exhaustively. Cases
that dissolve entirely into nixpkgs additionally carry a Dockerfile-faithful
twin so the page can show both translations side by side.

## The living migration corpus (30)

These are the checked-in conversions under `corpus/migrate/docker/`, not a second
historical grade set. Fidelity grades the translation; Evidence grades only the
named receipt. Open any case in the [side-by-side browser](corpus/index.html).

Ribbon vocabulary (Mathijs, 2026-08-05): **✅** = the case works; any
remaining deviation from Docker-faithful is deliberate — refused,
dissolved by design, or otherwise arranged — and stated in the cell.
**🔶** = open gaps remain, always qualified where a path exists:
**🔶🔄** the next regeneration improves it (the enabling fix has
landed; the case sits in the regeneration queue), **🔶⌛** an adopted
fix is in the pipeline but not yet implemented. A plain 🔶 has gaps
with no queued remedy. ⏳/❌ are retired from this table; refusals now
live inside the cell prose of the case that records them.

| # | Case | What the receipt establishes | Fidelity | Effort | Evidence |
|---:|---|---|---|:---:|---|
| 1 | Adminer | Faithful 5.5.0 and dissolved twins build cold; the refreshed faithful login probe passes | ✅ Source checksums, PHP tuning, dynamic design/plugin state, and `SIGINT` stopping are live; fixed identity and Alpine extension composition deliberately dissolve | S | [runtime probe + cold replay](../corpus/migrate/docker/adminer/receipt.md) · [closed-root (package contract)](#cip-84-closed-root-audit) |
| 2 | Caddy | Fresh faithful build and HTTP probe pass; distinct pinned raw assets fetched normally | ✅ The four-socket translation remains declared; the former 769-byte asset receipt was environment-tainted | S | [receipt](../corpus/migrate/docker/caddy/receipt.md) · [closed-root (package contract)](#cip-84-closed-root-audit) |
| 3 | Directus | The former loader and ENOTDIR walls are cleared; exact pnpm validates the pinned lock and a nearby revision, while offline deploy lacks package metadata | 🔶 The coherent source still produces no item because the pinned fetch artifact cannot serve every deploy-time package | M | [build receipt](../corpus/migrate/docker/directus/receipt.md) |
| 4 | Dozzle | Daemonless socket-bridge declaration formats/parses; importing cacert makes the formerly hung frontend FETCH complete both stability probes | 🔶 Socket bridge is intentionally desk/unprobed without gate dockerd; the full frontend/item/runtime remain unverified | M | [daemonless receipt](../corpus/migrate/docker/dozzle/receipt.md) |
| 5 | Echo Server | The worker's warm build, HTTP probe, and pinned cold replay pass after the script-driven dependency FETCH moves to a TOFU pin | ✅ Real `/app` contract and HTTP behavior regained; locked Node replacing the moving Alpine image is the deliberate canon substitution | M | [runtime probe + cold replay](../corpus/migrate/docker/echo-server/receipt.md) |
| 6 | Excalidraw | The faithful static item builds warm and cold; an in-store runner serves the title on port 80, while the supplied 18090/runner-path harness remains red | 🔶 Upstream layout and native readiness regained; host-architecture selection and two documented volatile-output normalizations differ | M | [warm+cold build and directed runtime proof](../corpus/migrate/docker/excalidraw/receipt.md) |
| 7 | Filestash | DNS now reaches the first FETCH, which exceeds its 20-minute bound snapshotting the module tree | 🔶 A 2.7 GiB/~69k-file fetched module cache blocks compilation before the static-library loop; no item exists | L | [build receipt](../corpus/migrate/docker/filestash/receipt.md) |
| 8 | Memcached | Faithful 1.6.45 source build answers `VERSION`; the nixpkgs protocol contract is separately sealed-root audited | ✅ Faithful source build at upstream-pinned 1.6.45 with upstream configure flags, twins present; root-only upstream test harness skipped (declared) | S | [runtime probe](../corpus/migrate/docker/memcached/receipt.md) · [closed-root (package contract)](#cip-84-closed-root-audit) |
| 9 | NATS | Faithful 2.12.14 binary answers monitoring health; the nixpkgs monitoring contract is separately sealed-root audited | ✅ Exact upstream release, twins present; the config/entrypoint contract is unrecoverable from SOURCE provenance — a permanent loss, declared (borderline call recorded 2026-08-05) | S | [runtime probe](../corpus/migrate/docker/nats/receipt.md) · [closed-root (package contract)](#cip-84-closed-root-audit) |
| 10 | nginx | Both twins build cold and the faithful HTTP probe passes | ✅ `SIGQUIT`, declared cache/runtime roles, and the pid path are live; Alpine hooks, identity, logs, and welcome page deliberately differ | S | [runtime probe + cold replay](../corpus/migrate/docker/nginx/receipt.md) · [closed-root (synthetic contract)](#cip-84-closed-root-audit) |
| 11 | Parse Server | Mongo-backed health probe and cold replay both pass | ✅ Release layout, state roles, credentials, JIT, and database behavior are live; Node 22 and retained dev dependencies are deliberate substitutions | M | [runtime probe + cold build](../corpus/migrate/docker/parse-server/receipt.md) |
| 12 | phpMyAdmin | The signed 5.2.3 source build and login probe pass after its mirror pipeline moves to a TOFU pin; cold replay exposes an `output` read-set divergence | 🔶 Config helpers, tuning, sessions, and application layout regained; exact PHP extension build, Apache entrypoint behavior, secret-file bridges, and the remaining cold read-set divergence are losses | S | [runtime probe + cold read-set defect](../corpus/migrate/docker/phpmyadmin/receipt.md) · [closed-root (login contract)](#cip-84-closed-root-audit) |
| 13 | Redis | Faithful 7.4.8 source build answers PING; the nixpkgs `/data` contract is separately sealed-root audited | ✅ Full upstream source build retained (protected-mode patch, TLS, jemalloc flags), twins present; identity/`gosu` dissolve into systemd by design | S | [runtime probe](../corpus/migrate/docker/redis/receipt.md) · [closed-root (package contract)](#cip-84-closed-root-audit) |
| 14 | Renovate | Both nixpkgs-authoritative twins build warm/cold and the daily calendar/compose contract validates; the sealed-root audit separately proves timer/version/log activation | ✅ Faithful to the supplied narrow CronJob contract; repository config, credentials, actual renovation, and richer job policy are absent because the supplied contract never carried them (borderline call recorded 2026-08-05) | S | [build+schedule validation](../corpus/migrate/docker/renovate/receipt.md) · [closed-root (timer/version contract)](#cip-84-closed-root-audit) |
| 15 | Tomcat | Both twins build warm/cold and the empty-server HTTP probe passes; the sealed-root audit proves the same reachability class | 🔶 Tomcat 10.1.57/JRE21 runtime, ports, and twins regained; Tomcat Native and writable work/log trees remain losses | S | [runtime probe](../corpus/migrate/docker/tomcat/receipt.md) · [closed-root](#cip-84-closed-root-audit) |
| 16 | Traefik | Faithful 3.5.6 answers ping after its release metadata and asset FETCHes move to TOFU pins; their update probes and pinned cold replay pass | 🔶 The selected release, architecture, and published asset digest are retained; the supplied entrypoint remains unavailable, so arbitrary-argument proxy behavior is unproved | S | [runtime probe + cold replay](../corpus/migrate/docker/traefik/receipt.md) · [closed-root (package contract)](#cip-84-closed-root-audit) |
| 17 | Verdaccio | Its pnpm CAS is deterministic, but bare-CAS offline replay fails because the consumed volatile package index is not reconstructable | 🔶 Runtime layout is translated but unproved; a stable/rebuildable package-to-integrity index mechanism is missing | M | [build receipt](../corpus/migrate/docker/verdaccio/receipt.md) |
| 18 | Wallos | Upstream `/var/www/html` assembly with nested roles builds warm and cold | 🔶 Runtime probe is blocked by the workspace-local cix post-probe under `ProtectHome`; cron and Docker process topology remain deliberate losses | M | [build receipt](../corpus/migrate/docker/wallos/receipt.md) |
| 19 | Watchtower | Warm/cold artifact builds and manifest inspection pass | ✅ The socket bridge is intentionally desk/unprobed; the available CI binary is not asserted byte-identical to Docker's missing payload | M | [daemonless build receipt](../corpus/migrate/docker/watchtower/receipt.md) |
| 20 | Whoami | Both Docker-shaped and dissolved twins build warm/cold and the HTTP probe passes | ✅ Runtime certificate/timezone layout and twin regained; the source build stays unproved because upstream never published its context — a permanent, declared evidence boundary (borderline call recorded 2026-08-05) | M | [runtime probe + cold twin builds](../corpus/migrate/docker/whoami/receipt.md) |
| 21 | Mastodon | Fresh six-member in-place CIP-91/92 receipt: credentials, Unix edges, shared-rw state, health, timer, logs, and purge | 🔶 Declared losses — three application members are stubs and D26/D27 network segmentation is absent | M | [closed-root](#cip-84-closed-root-audit) · [modernized in-place receipt](../corpus/migrate/docker/mastodon/receipt.md) |
| 22 | Homer | Locked pnpm/Vite source build and HTTP app-mount probe pass | ✅ The staged registry symptom was a missing CA/traced-store prerequisite; Alpine identity/layout and runtime-port interpolation deliberately dissolve | M | [runtime probe](../corpus/migrate/docker/homer/receipt.md) |
| 23 | it-tools | Current-cix source build/replay produces a web item; the retained item returns HTTP 200 at `/` under system-manager | 🔶 Desk/verified split: the item/root HTTP 200 and DynamicUser service are verified, but a fresh ordinary replay hit its 240-second build bound; the final 1,536,045-line lock does not show CIP-99 workspace-root aggregation, nginx uses ephemeral `/var/log/nginx` with access-file logging off, and the deep-route probe returns 404 | M | [build/runtime receipt](../corpus/migrate/docker/it-tools/receipt.md) |
| 24 | Mailpit | Clean source build and degraded user-manager `/livez` probe pass | 🔶 Supported system-manager native readiness is blocked by workspace-local probe execution under `ProtectHome` | M | [build and runtime receipt](../corpus/migrate/docker/mailpit/receipt.md) |
| 25 | Valkey | Faithful 8.1.9 build, `PING`, and empty-workspace cold replay pass; the dissolved twin builds cold | 🔶 Build-generated random paths no longer enter the read set; DynamicUser replaces Docker ownership, with a declared `/data` state role | M | [build/runtime and cold-replay receipt](../corpus/migrate/docker/valkey/receipt.md) |
| 26 | HAProxy | Faithful 3.2.22 source build, version probe, and cold replay pass; dissolved twin builds cold | 🔶 No supplied config, listener, or health contract exists to start; formatter whitespace currently changes the locked FETCH identity | M | [build/version and formatter receipt](../corpus/migrate/docker/haproxy/receipt.md) |
| 27 | Apache HTTPD | Faithful 2.4.68 build, `It works!` HTTP probe, and empty-workspace cold replay pass; dissolved twin builds cold | 🔶 File logging through `LOGDIR` replaces sandbox-inaccessible fd symlinks; several Docker-adjacent modules remain unavailable to the configured source build | M | [build/runtime and cold-replay receipt](../corpus/migrate/docker/httpd/receipt.md) |
| 28 | Mosquitto | Faithful 2.0.22 MQTT pub/sub roundtrip and cold replay pass; dissolved twin builds cold | 🔶 TCP behavior is live; WebSockets are omitted because the source build cannot see the required headers, and this host degrades `PrivatePIDs` | M | [build/runtime and cold receipt](../corpus/migrate/docker/mosquitto/receipt.md) |
| 29 | ntfy | Faithful 2.27.0 release artifact build and `/v1/health` probe pass; dissolved twin builds | ✅ The Docker-only GoReleaser context artifact is now an explicit, checksum-verified FETCH; Alpine/image metadata and entrypoint-only invocation deliberately dissolve | S | [build and runtime receipt](../corpus/migrate/docker/ntfy/receipt.md) |
| 30 | Filebrowser | Faithful 2.63.23 release artifact build and dissolved twin build pass | 🔶 The source init contract cannot start because arbitrary-path role realization hides the writable `/config` bind; `/health` is honestly unproved | S | [build receipt and runtime wall](../corpus/migrate/docker/filebrowser/receipt.md) |

### CIP-84 closed-root audit

The phase-1 sealed-root VM reproduces and probes every pack member, plus the
Adminer, Caddy, Memcached, NATS, nginx, phpMyAdmin, Redis, Renovate, Tomcat,
and Traefik unary corpus contracts and the complete six-member Mastodon compose
contract. For the regenerated twins, that VM's Caddy, Memcached, NATS, nginx,
Redis, and Traefik fixtures are explicitly package-level or synthetic contracts;
their faithful-source evidence is the per-case build/probe receipt above. The
sealed-root tier therefore applies only to the behavior named in each row, not
to faithful version, entrypoint, or configuration parity. The check has an
exhaustive directory roster, so a newly added pack or migration cannot silently
escape classification.

Thirteen migrations remain outside the green closed-root set. Directus, Filestash,
and Verdaccio still fail before producing a runnable item; Dozzle's FETCH is now
diagnosed and green but its full item is unverified, while its socket bridge and
Watchtower's remain intentionally desk-only Docker-control-plane cases; Parse
Server now has a fresh
Mongo-backed runtime receipt but is not in the sealed-root roster. Echo Server,
Excalidraw, Wallos, and Whoami now have freshly fetched inputs and regenerated
warm/cold evidence: Wallos and Whoami reproduce cleanly, Excalidraw's item is
cold-stable but its supplied runner/port harness is red, and Echo Server's lost
snapshot exposes an unchanged EXPECT mismatch. Those four are no longer
historical one-off derivations, and the new Homer, it-tools, and Mailpit cases
have per-case independent receipts; none is **closed-root verified** until the
phase-2 roster actually exercises its named behavior.

## 1. Compose files in the wild (18)

| # | Stack (source) | What it actually needs | Ribbon | Effort | Evidence |
|---|---|---|---|---|---|
| 1 | [wordpress-mysql](https://raw.githubusercontent.com/docker/awesome-compose/master/wordpress-mysql/compose.yaml) | 2 svcs, named volume, ports, env | 🔶 MySQL maps to private state + ports; WordPress's mutable `/var/www/html` volume still needs the immutable-app/state restructure from Dockerfile row 16 | S | desk |
| 2 | [prometheus-grafana](https://raw.githubusercontent.com/docker/awesome-compose/master/prometheus-grafana/compose.yaml) | named vol + relative config binds | 🔶 private roles plus compose `host:`/`.env` containment cover the storage shape; relative source/config conversion remains a migration exercise | S | desk |
| 3 | [flask-redis](https://raw.githubusercontent.com/docker/awesome-compose/master/flask-redis/compose.yaml) | build: + live source bind (dev loop) | 🔶 D47 build + `cix watch` warm rebuild/restart are built; source sync is deliberately refused, so framework hot reload stays in `nix develop` | M | desk |
| 4 | [react-express-mongodb](https://raw.githubusercontent.com/docker/awesome-compose/master/react-express-mongodb/compose.yaml) | 2 networks (frontend can't see db), anon-volume masking node_modules | 🔶 CIP-86 pod boundaries and no-egress enforcement are built, but this topology needs D26/D27 multi-network membership or `talks-to`; volume-masking remains a ❌ idiom (restructure honestly) | M | desk |
| 5 | [Gitea](https://docs.gitea.com/installation/install-with-docker) | 1 svc, data dir, ports incl. SSH 222:22 | ✅ private state dir + declared ports | S | desk |
| 6 | [Umami](https://raw.githubusercontent.com/umami-software/umami/master/docker-compose.yml) | healthcheck-gated startup, init: | 🔁 HTTP/TCP `READINESS`/`LIVENESS` and structural readiness ordering are built; the separate condition graph is deliberately ❌; init ceremony dissolves under systemd | S | desk |
| 7 | [Immich](https://raw.githubusercontent.com/immich-app/immich/main/docker/docker-compose.yml) | env-interpolated bind paths, image healthchecks, shm_size, GPU overlays | 🔶 own-directory `.env`, static-identity `host:`, probes, and `SHM` are built; `CLAIM gpu` still proves unit properties only—not Immich/NVIDIA integration | M | desk |
| 8 | [Paperless-ngx](https://raw.githubusercontent.com/paperless-ngx/paperless-ngx/main/docker/compose/docker-compose.postgres.yml) | consume watch-dir (host-shared rw bind), variant compose files | 🔶 `DIR /consume` plus static-identity read-write `host:` materialization cover the watch-dir; tagged/inline subtrees are built, while computed D46 variants remain publish-time work | M | desk |
| 9 | [Mastodon](https://raw.githubusercontent.com/mastodon/mastodon/main/docker-compose.yml) | `internal: true` no-egress net; rw dir **shared between web+sidekiq**; 127.0.0.1 binds; 5-svc health DAG | 🔶 executable six-member stack proves shared-rw, structural readiness, native liveness, a DB credential, Unix edges, maintenance timer, and selected logs; pod-local 127.0.0.1 and per-member egress suppression are built (CIP-86); the health-condition DAG stays refused and exact named-network segmentation still needs D26/D27 | M | [receipt](../corpus/migrate/docker/mastodon/receipt.md) |
| 10 | [Penpot](https://raw.githubusercontent.com/penpot/penpot/main/docker/images/docker-compose.yaml) | YAML anchors (preprocessing), shared-rw assets volume | 🔶 anchors are moot (JSON canonical, D28); the shared-rw mechanism is now empirically closed by the Mastodon stack, while a Penpot application migration remains unrun | M | [shared-rw receipt](../corpus/migrate/docker/mastodon/receipt.md) |
| 11 | [Plausible CE](https://raw.githubusercontent.com/plausible/community-edition/master/compose.yml) | ulimits, migrate-then-run chains | 🔶 `START_PRE` maps the setup chain; `LimitNOFILE` is native systemd operator policy but has no compose field | S | desk |
| 12 | [Authentik](https://goauthentik.io/docker-compose.yml) | normal stack + worker mounting **docker.sock** to manage outposts | stack ✅; socket-worker ❌ (imperatively orchestrates siblings — competing model; our answer is cix's own surface) | M | desk |
| 13 | [Pi-hole](https://raw.githubusercontent.com/pi-hole/docker-pi-hole/master/README.md) | cap NET_ADMIN/SYS_TIME/SYS_NICE, port 53, DHCP/NTP | 🔶 port declaration works, but those raw capabilities and host mutations require explicit operator policy; no native semantic claim yet | M | desk |
| 14 | [Supabase](https://raw.githubusercontent.com/supabase/supabase/master/docker/docker-compose.yml) | 12-svc health-conditioned DAG, 20+ binds, :z flags | 🔶 native probes, structural readiness ordering, and static-identity `host:` binds are built; the condition graph is deliberately ❌ and `:z` is SELinux/operator policy | L | desk |
| 15 | [Sentry self-hosted](https://raw.githubusercontent.com/getsentry/self-hosted/master/docker-compose.yml) | ~60 svcs, profiles, external volumes, installer-driven | profiles ≈ D46 parametric ⏳; compose-as-installer-backend ❌-leaning honest | XL | desk |
| 16 | [Nextcloud AIO](https://raw.githubusercontent.com/nextcloud/all-in-one/main/compose.yaml) | mastercontainer spawns ~10 siblings via docker.sock | ❌ — the file describes 10% of the deployment; competing orchestration model | — | desk |
| 17 | [Frigate](https://docs.frigate.video/frigate/installation) | privileged, 5 device passthroughs, sized tmpfs, shm | devices 🔶 `CLAIM device` + closed `DevicePolicy=` dogfooded on a VM node, no privileged widening; `SHM` ✅; not a full Frigate app verification | L | desk |
| 18 | [Home Assistant](https://www.home-assistant.io/installation/linux) | host network, privileged, dbus, USB | ❌ as a broad host-appliance workload: CIP-84 deliberately has no raw-host filesystem view; individual device and data needs can be declared, but ambient D-Bus/host reach is outside cix until it has narrow claims | M | desk |

**Frequency signals (18 files):** restart policy 17, named volumes 13, bind mounts 14,
healthchecks 10 + condition-gated depends_on 6, .env/interpolation ~6, named networks 6
(multi-net 2, internal:true 1), shm_size 5, docker.sock 3, privileged 2, compose
`secrets:` **0**, `deploy/replicas` **0**. The wild ignores the features docker-compose
has on paper (secrets, replicas — validating D30's deferrals) and leans hard on the ones
whose design now has implementation behind it (CIP-79 health and CIP-82 directory
materialization), including the multi-member Mastodon receipt below.

## 2. Kubernetes shapes in the wild (15)

| # | Shape (source) | Essential semantics | Ribbon | Effort | Evidence |
|---|---|---|---|---|---|
| 1 | [minimal nginx Deployment](https://raw.githubusercontent.com/kubernetes/website/main/content/en/examples/application/deployment.yaml) | run N copies on port 80 | ✅ one path-keyed instance; replicas remain ⏳ and are explicitly not CIP-85 leg 1 | S | desk |
| 2 | [Guestbook](https://raw.githubusercontent.com/kubernetes/examples/master/web/guestbook/frontend-deployment.yaml) | stateless tier finds redis via DNS | ⏳ the services are packageable, but TCP discovery/DNS remains networking work; probes await CIP-79 and limits await resource policy | S | desk |
| 3 | [Bitnami WordPress chart](https://raw.githubusercontent.com/bitnami/charts/main/bitnami/wordpress/templates/deployment.yaml) | PHP + writable dir + DB password + hostname | 🔶 arbitrary-path role dirs dissolve init-chown ceremony; DB discovery and credential delivery remain queued | M | desk |
| 4 | [Bitnami PostgreSQL chart](https://raw.githubusercontent.com/bitnami/charts/main/bitnami/postgresql/templates/primary/statefulset.yaml) | one postgres, stable identity, durable volume, password | 🔶 most permission ceremony dissolves into DynamicUser + `STATEDIR`; password delivery remains queued after CIP-81 | S | desk |
| 5 | [Bitnami Redis chart](https://raw.githubusercontent.com/bitnami/charts/main/bitnami/redis/templates/master/application.yaml) | redis + volume + password + "only my clients connect" | NetworkPolicy → ⏳ D27 talks-to; privileged init-sysctl = host prerequisite, honestly operator-side | M | desk |
| 6 | [Grafana chart](https://raw.githubusercontent.com/grafana-community/helm-charts/main/charts/grafana/templates/_pod.tpl) | grafana + config file + admin secret | 🔶 config-in-item + repin dissolves the reload sidecars; admin credential delivery remains queued | M | desk |
| 7 | [Prometheus operator CR](https://raw.githubusercontent.com/prometheus-community/helm-charts/main/charts/kube-prometheus-stack/templates/prometheus/prometheus.yaml) | scrape-what-selectors-say, N days on disk | ⏳ far — operator model ≈ reconciler + D46 parametric; the CR *interface style* is the lesson | L | desk |
| 8 | [node-exporter DaemonSet](https://raw.githubusercontent.com/prometheus-community/helm-charts/main/charts/prometheus-node-exporter/templates/daemonset.yaml) | one observer per node reading /proc,/sys | 🔶 operator `DIR :ro` and mutable per-host root files are built; automatic one-per-node reconciliation remains absent | S | desk |
| 9 | [ingress-nginx](https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/charts/ingress-nginx/templates/controller-deployment.yaml) | control loop watching cluster API | ❌ different world — our host edge is binds/publishes, not an API-watching proxy; HPA/PDB n/a | — | desk |
| 10 | [Istio Bookinfo](https://raw.githubusercontent.com/istio/istio/master/samples/bookinfo/platform/kube/bookinfo.yaml) | 4 microservices with identities; mesh by injection | path-identity tree + pod colocation ✅; mesh policy ⏳ D27; injection-as-mechanism ❌ (we declare) | M | desk |
| 11 | [Renovate CronJob](https://raw.githubusercontent.com/renovatebot/helm-charts/main/charts/renovate/templates/cronjob.yaml) | run batch on schedule with token + config | 🔶 APP `schedule`/persistent timer and indexed logs pass; token/config delivery was not converted | S | [receipt (timer/log only)](../corpus/migrate/docker/renovate/receipt.md) |
| 12 | [Airflow scheduler](https://raw.githubusercontent.com/apache/airflow/main/chart/templates/scheduler/scheduler-deployment.yaml) | don't start before DB schema; synced DAG dir | wait-init → built `START_PRE`/ordering 🔶; git-sync sidecar vs repin remains a content-model mismatch | M | desk |
| 13 | [Airflow migrate Job](https://raw.githubusercontent.com/apache/airflow/main/chart/templates/jobs/migrate-database-job.yaml) | run migration once per upgrade | 🔶 built `START_PRE` covers cold start; migrate-on-upgrade hook remains a design question | M | desk |
| 14 | [cert-manager](https://raw.githubusercontent.com/cert-manager/cert-manager/master/deploy/charts/cert-manager/templates/deployment.yaml) | three control loops + CRDs, no "app" at all | ❌ different world; the *need* (cert provisioning) returns later via credentials story | — | desk |
| 15 | [CloudNativePG Cluster CR](https://raw.githubusercontent.com/cloudnative-pg/cloudnative-pg/main/docs/src/samples/cluster-example-full.yaml) | "HA postgres, size X, these params, backup to S3" | the operator absorbs 100% of row-4's ceremony — same app at the opposite mechanism extreme; composix's pitch is that interface *without* a cluster | — | desk |

## 3. Dockerfiles in the wild (18)

| # | Image (source) | What the build actually does | Ribbon | Effort | Evidence |
|---|---|---|---|---|---|
| 1 | [postgres official](https://github.com/docker-library/postgres/blob/master/17/bookworm/Dockerfile) | apt + arch-conditional source compile + gosu/uid-999/locale surgery | ✅ dissolves — nixpkgs ships postgres; the file becomes a package selection + service contract | S | desk |
| 2 | [redis official](https://github.com/docker-library/redis/blob/master/7.4/debian/Dockerfile) | source compile + sha256 tarball + gosu | ✅ nixpkgs Redis + direct `STATEDIR /data`; PING passes | S | [receipt](../corpus/migrate/docker/redis/receipt.md) |
| 3 | [nginx official](https://github.com/nginx/docker-nginx/blob/master/mainline/debian/Dockerfile) | pinned vendor apt + GPG + stdout log symlinks | ✅ dissolves; stdout/stderr go to indexed journald, so log symlinks are unnecessary | S | [receipt](../corpus/migrate/docker/nginx/receipt.md) |
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
| 3 | Shared-rw directories | **Met ([CIP-82](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0082-dirs.md))** | Compose 9, 10; dirs2 VM proves setgid group sharing and the [Mastodon receipt](../corpus/migrate/docker/mastodon/receipt.md) proves two application members write one surface |
| 4 | Timers / CronJob | **Met ([CIP-75](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0075-timers.md))** | Kubernetes 11; [Renovate receipt](../corpus/migrate/docker/renovate/receipt.md) |
| 5 | Operational logs | **Met ([CIP-83](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0083-observability.md))** | Dockerfile 3; Kubernetes 11 |
| 6 | Artifact dev loop | **Met; source sync refused ([CIP-76](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0076-devloop.md))** | Compose 3 |
| 7 | Migrate-on-upgrade hook | **Designed—unbuilt ([CIP-75](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0075-timers.md), event-driven D48f leg)** | Compose 11; Kubernetes 12, 13 |
| 8 | Network segmentation and talks-to | **Pod realization built; named segmentation/talks-to designed—unbuilt ([CIP-86](https://github.com/mathijshenquet/composix/blob/main/cips/accepted/0086-netns.md))** | Compose 4, 9; Kubernetes 2, 5, 10 — `scenario-netns` proves pod isolation, egress polarity, publish tiers, and stable IPAM; D26/D27 still block the listed multi-network policies |
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

- **Mastodon-shaped stack** is adopted into the executable corpus: its
  [receipt](../corpus/migrate/docker/mastodon/receipt.md) composes shared state, readiness,
  liveness, secrets, Unix edges, a timer, and selected logs. D26/D27 network
  segmentation remains explicitly unclaimed, so a future netns rerun strengthens the
  same case rather than inventing another example.
- **Paperless-shaped ingest** is the smallest clean CIP-82 leg-2 gate: one writable
  operator watch-dir, one private state dir, and an observable import result.
- **Immich-shaped** is deferred to the in-flight CIP-78 devices track; it remains the
  combined host-dir/health/GPU stress case, not a regrade-track example.
- **Pi-hole** remains useful, but it is a gap-finder rather than a pure capability win:
  NET_ADMIN/SYS_TIME/SYS_NICE have no native semantic claims today.
- **Renovate-shaped cron batch** is retired as an example candidate: the focused
  [corpus receipt](../corpus/migrate/docker/renovate/receipt.md) now covers timer generation,
  execution, and logs. A future full example would be about CIP-81 credentials, not
  timers.
- **Gitea-shaped build** (proj2 candidate): go+pnpm dual-ecosystem compile with cache
  mounts and a version-stamp — the heaviest realistic D47 exercise short of nasty.
- **Bitnami-postgres vs our postgres pack**: not a new example — a docs page putting
  the chart's 400 lines next to our Cixfile, per k8s rows 4/15. Pure showcase.

Adopt one candidate per track with its forcing function as the gate.
