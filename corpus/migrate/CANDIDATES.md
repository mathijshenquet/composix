# Migrate-corpus candidates — 48 wild Dockerfiles, verified 2026-07-30

Sourced per `.dev/specs/track-migrate.md` (popularity × build speed; no build farm).
All URLs returned HTTP 200 on 2026-07-30; mechanism columns extracted from the actual
file contents, not guessed. Spread: 12 easy / 28 middling / 7 nasty (weighted toward
middling — trivial teaches the prompt nothing, nasty teaches it everything at once).
Rounds consume from the top of each difficulty band; slow-tier rows are flagged and
used sparingly.

| name | Dockerfile URL | base image | est. build | difficulty | mechanisms | natural probe | no escape? (nixpkgs) |
|---|---|---|---|---|---|---|---|
| whoami | https://raw.githubusercontent.com/traefik/whoami/master/Dockerfile | golang:1-alpine → scratch | fast | easy | multi-stage, static Go binary, scratch runtime, EXPOSE, ENTRYPOINT | HTTP GET / echoes headers | no — `whoami` |
| traefik | https://raw.githubusercontent.com/traefik/traefik-library-image/master/v3.5/alpine/Dockerfile | alpine:3.22 | fast | easy | wget prebuilt release tarball, ENTRYPOINT, EXPOSE | `traefik version` or GET :8080/ping | no — `traefik` |
| nats | https://raw.githubusercontent.com/nats-io/nats-docker/main/2.12.x/alpine3.22/Dockerfile | alpine:3.22 | fast | easy | wget binary + sha256 verify, apk, config COPY, ENTRYPOINT | GET :8222/healthz | no — `nats-server` |
| caddy | https://raw.githubusercontent.com/caddyserver/caddy-docker/master/2.11/alpine/Dockerfile | alpine:3.23 | fast | easy | curl binary + sha512 verify, XDG env dirs, EXPOSE | GET :80 / `caddy version` | no — `caddy` |
| echo-server | https://raw.githubusercontent.com/Ealenn/Echo-Server/master/Dockerfile | node:lts-alpine (multi-stage) | fast | easy | multi-stage npm install/build, ENTRYPOINT | HTTP GET / echoes request | yes — no matching app found |
| adminer | https://raw.githubusercontent.com/TimWolla/docker-adminer/master/5/Dockerfile | php:8.4-alpine | fast | easy | apk, curl + sha256 download, USER, STOPSIGNAL, php built-in server | HTTP GET / login page | no — `adminer` |
| memcached | https://raw.githubusercontent.com/docker-library/memcached/master/1/alpine/Dockerfile | alpine:3.24 | medium | easy | small source compile (make -j), build-deps apk pattern, USER, ENTRYPOINT | `printf 'version\r\n' \| nc 11211` | no — `memcached` |
| ntfy | https://raw.githubusercontent.com/binwiederhier/ntfy/main/Dockerfile | alpine | fast | easy | COPY prebuilt binary (goreleaser context), apk, ENTRYPOINT — needs artifact fetched first | GET /v1/health | no — `ntfy-sh` |
| filebrowser | https://raw.githubusercontent.com/filebrowser/filebrowser/master/Dockerfile | alpine:3.23 + busybox stage | fast | easy | COPY prebuilt binary from context, USER, VOLUME, HEALTHCHECK, JSON config | GET /health | no — `filebrowser` |
| nginx | https://raw.githubusercontent.com/nginx/docker-nginx/master/mainline/alpine-slim/Dockerfile | alpine:3.24 | fast | middling | apk from vendor repo (source-build fallback path), entrypoint.d templating (envsubst), STOPSIGNAL, symlinked logs | HTTP GET / | no — `nginx` |
| redis | https://raw.githubusercontent.com/redis/docker-library-redis/master/7.4/alpine/Dockerfile | alpine:3.21 | medium | middling | source compile, gpg+sha256, VOLUME /data, entrypoint drops root (su-exec) | `redis-cli PING` → PONG | no — `redis` |
| mysql | https://raw.githubusercontent.com/docker-library/mysql/master/8.4/Dockerfile.oracle | oraclelinux:9-slim | medium | middling | microdnf rpm install from vendor repo, gpg key import, VOLUME, init-entrypoint | `mysqladmin ping` | no — `mysql84` |
| mariadb | https://raw.githubusercontent.com/MariaDB/mariadb-docker/master/11.4/Dockerfile | ubuntu:noble | medium | middling | apt vendor repo, gosu, VOLUME, initdb entrypoint, bundled healthcheck.sh | `healthcheck.sh --connect` | no — `mariadb` |
| mongo | https://raw.githubusercontent.com/docker-library/mongo/master/8.0/Dockerfile | ubuntu:noble | medium | middling | apt vendor repo, gpg+sha256, numactl wrapper, VOLUME /data/db, entrypoint | `mongosh --eval 'db.runCommand({ping:1})'` | no — `mongodb` |
| postgres (debian) | https://raw.githubusercontent.com/docker-library/postgres/master/17/trixie/Dockerfile | debian trixie (PGDG apt) | medium | middling | apt vendor repo, locale gen, gosu, initdb entrypoint, VOLUME — official alpine variant = slow source compile, prefer this one | `pg_isready` | no — `postgresql` |
| tomcat | https://raw.githubusercontent.com/docker-library/tomcat/master/10.1/jre21/temurin-noble/Dockerfile | eclipse-temurin:21-jre-noble | fast | middling | tarball download + gpg verify, writable work/temp dirs, ENTRYPOINT | GET :8080/ responds | no — `tomcat` |
| phpmyadmin | https://raw.githubusercontent.com/phpmyadmin/docker/master/apache/Dockerfile | php:8.3-apache | medium | middling | apt, php ext compile, gpg+sha256 tarball, entrypoint config-gen, session dir | HTTP GET / login page | yes — no matching app found |
| registry | https://raw.githubusercontent.com/distribution/distribution/main/Dockerfile | golang alpine (xx cross) → alpine | medium | middling | multi-stage cross-compile (tonistiigi/xx), config YAML, VOLUME /var/lib/registry | GET /v2/ returns 200 {} | no — `distribution` |
| vault | https://raw.githubusercontent.com/hashicorp/vault/main/Dockerfile | alpine:3 / ubi-minimal | fast | middling | COPY release binary (CI context), multi-target stages, VOLUME, USER, IPC_LOCK expectation | GET /v1/sys/health (dev mode) | no — `vault` |
| minio | https://raw.githubusercontent.com/minio/minio/master/Dockerfile | minio/minio:latest (repack) | fast | middling | binary repackage, sha256, VOLUME /data, entrypoint script | GET /minio/health/live | no — `minio` |
| syncthing | https://raw.githubusercontent.com/syncthing/syncthing/main/Dockerfile | golang → alpine | medium | middling | multi-stage go build from repo context, VOLUME, HEALTHCHECK, capability drop | GET /rest/noauth/health | no — `syncthing` |
| miniflux | https://raw.githubusercontent.com/miniflux/v2/main/packaging/docker/alpine/Dockerfile | golang:alpine3.23 → alpine:3.24 | medium | middling | multi-stage go build, USER nobody, external postgres dependency | GET /healthcheck (needs postgres) | no — `miniflux` |
| gotify | https://raw.githubusercontent.com/gotify/server/master/docker/Dockerfile | node + custom build img → debian | medium | middling | multi-stage yarn UI + go build via vendor build image, HEALTHCHECK, ENTRYPOINT | GET /health | no — `gotify-server` |
| uptime-kuma | https://raw.githubusercontent.com/louislam/uptime-kuma/master/docker/dockerfile | louislam/uptime-kuma:base2 (custom debian base) | medium | middling | custom pullable base images, npm ci, embedded sqlite data dir, HEALTHCHECK binary, USER | GET / or bundled healthcheck | no — `uptime-kuma` |
| healthchecks | https://raw.githubusercontent.com/healthchecks/healthchecks/master/docker/Dockerfile | python:3.14-slim-trixie (multi-stage) | medium | middling | multi-stage pip wheels, apt, USER, HEALTHCHECK, Django migrations at start | HTTP GET / login page | no — `healthchecks` |
| changedetection | https://raw.githubusercontent.com/dgtlmoon/changedetection.io/master/Dockerfile | python:slim-bookworm (multi-stage) | medium | middling | multi-stage pip build-deps, apt, datastore dir, ENTRYPOINT | HTTP GET / | no — `changedetection-io` |
| freshrss | https://raw.githubusercontent.com/FreshRSS/FreshRSS/edge/Docker/Dockerfile | debian:13-slim | medium | middling | apt php+apache, cron-in-container, entrypoint env config | GET /i/ login page | no — `freshrss` |
| kanboard | https://raw.githubusercontent.com/kanboard/kanboard/main/Dockerfile | alpine:3.24 | fast | middling | apk php+nginx+php-fpm multi-process, VOLUME data, HEALTHCHECK, ENTRYPOINT | HTTP GET / login page | no — `kanboard` |
| verdaccio | https://raw.githubusercontent.com/verdaccio/verdaccio/master/Dockerfile | node:24-alpine (multi-stage) | medium | middling | multi-stage pnpm install, USER, VOLUME storage, config YAML | GET /-/ping | yes — no matching app found |
| homepage | https://raw.githubusercontent.com/gethomepage/homepage/main/Dockerfile | node:22-slim → node:22-alpine | medium | middling | multi-stage pnpm Next.js build, USER, HEALTHCHECK (wget), config dir | GET / | no — `homepage-dashboard` |
| ghost | https://raw.githubusercontent.com/TryGhost/docker-library-ghost/master/6/alpine3.23/Dockerfile | node:22-alpine3.23 | slow | middling | npm install ghost-cli+ghost (heavy npm tree — slow tier), gpg, VOLUME content dir | HTTP GET / (after sqlite init) | yes — `ghost` is unrelated; `ghost-cli` is installer tooling |
| vaultwarden | https://raw.githubusercontent.com/dani-garcia/vaultwarden/main/docker/Dockerfile.debian | rust:slim-trixie → debian:trixie-slim | slow | middling | multi-stage cargo build (slow tier), pinned digests, xx cross, web-vault COPY, VOLUME, HEALTHCHECK | GET /alive | no — `vaultwarden` |
| rabbitmq | https://raw.githubusercontent.com/docker-library/rabbitmq/master/4.1/alpine/Dockerfile | alpine:3.23 (multi-stage) | slow | middling | compiles OpenSSL + Erlang/OTP from source (slow tier); gpg+sha256, VOLUME, entrypoint | `rabbitmq-diagnostics ping` | no — `rabbitmq-server` |
| postgres (alpine, official) | https://raw.githubusercontent.com/docker-library/postgres/master/17/alpine3.23/Dockerfile | alpine:3.23 | slow | middling | official = slow source compile; gpg, initdb entrypoint, su-exec, VOLUME, STOPSIGNAL — prefer trixie apt variant | `pg_isready` | no — `postgresql` |
| wordpress | https://raw.githubusercontent.com/docker-library/wordpress/master/latest/php8.4/apache/Dockerfile | php:8.4-apache | medium | nasty | php ext compiles, entrypoint copies WP core into VOLUME at runtime (self-mutating webroot), needs mysql | GET /wp-admin/install.php | no — `wordpress` |
| nextcloud | https://raw.githubusercontent.com/nextcloud/docker/master/34/apache/Dockerfile | php:8.5-apache-trixie | medium | nasty | php ext compiles, entrypoint rsyncs app code into VOLUME, upgrade-on-start logic, cron sidecar expectation | GET /status.php | no — `nextcloud34` |
| gitea | https://raw.githubusercontent.com/go-gitea/gitea/main/Dockerfile | golang:1.26-alpine → alpine:3.24 | medium | nasty | multi-stage go + pnpm frontend build, s6 supervisor running sshd+gitea multi-process, VOLUME /data, templated config | GET /api/healthz | no — `gitea` |
| pihole | https://raw.githubusercontent.com/pi-hole/docker-pi-hole/master/src/Dockerfile | alpine:3.24 (pinned digest, multi-stage) | fast | nasty | prebuilt FTL download, bash entrypoint jungle, multi-process DNS+web, NET_ADMIN caps, HEALTHCHECK | `dig @127.0.0.1` + GET /admin/ | no — `pihole` |
| linuxserver/nginx | https://raw.githubusercontent.com/linuxserver/docker-nginx/master/Dockerfile | ghcr.io/linuxserver/baseimage-alpine-nginx:3.24 | fast | nasty | s6-overlay init, PUID/PGID chown-at-start, /config VOLUME convention, mod-download system | HTTP GET / | ambiguous — `nginx` exists, LinuxServer wrapper does not |
| dozzle | https://raw.githubusercontent.com/amir20/dozzle/master/Dockerfile | node+golang alpine → scratch | medium | nasty | multi-stage pnpm UI + go build, scratch runtime, hard docker-socket dependency | GET /healthcheck (needs docker socket) | yes — no matching app found |
| watchtower | https://raw.githubusercontent.com/containrrr/watchtower/main/dockerfiles/Dockerfile | alpine:3.19 → scratch | fast | nasty | COPY prebuilt binary (context), scratch + tzdata/certs, HEALTHCHECK, docker-socket + docker-API dependency | one-shot `--run-once` exit 0 | yes — no matching app found |

## Curation notes

- A fresh clone must fetch the pinned upstream build contexts before running any
  migration check: `cd corpus/migrate && ./fetch.sh --all`. To fetch one context,
  use `./fetch.sh <name>`; `check.sh` names the exact command when it is missing.

- The `no escape?` column was refreshed against the current nixpkgs on 2026-07-30:
  `nix search nixpkgs '^(…)$' --json` for the candidate and known alternate names,
  followed by `nix eval --raw nixpkgs#<attr>.pname` for aliases. `yes` means no
  matching application package was found, not that a similarly named package is
  absent; the Ghost and LinuxServer rows make those ambiguity boundaries explicit.
- Deliberately excluded after inspection: searxng (`FROM localhost/...:builder`, not
  standalone-buildable), navidrome (multi-GB osxcross image in build),
  grafana/keycloak/metabase/immich/paperless (slow-tier builds), portainer
  (docker-socket class already represented by dozzle/watchtower).
- ntfy, filebrowser, vault, watchtower COPY a prebuilt binary from the build context
  (goreleaser/CI pattern): fast builds, but the artifact must be fetched first — that
  is itself a realistic migration mechanism (maps to D47 top-level FETCH).
- The docker-socket rows (dozzle, watchtower) are expected ❌/corpus-gap material —
  they are in the list to *prove* the boundary, not to pass.

## No-escape additions — 2026-07-31 hunt (Mathijs: grow the build-class share)

The original 48 rows contain only 5 without a nixpkgs escape. These 12 additions all
BUILD from source in their Dockerfile AND have no nixpkgs package: exact `nix search
nixpkgs '^name$'` empty and broad alternate-name search empty, verified 2026-07-31,
each row independently re-verified after agent curation (one agent false-absent,
`stump`, was caught and removed in that pass). Migration rounds wanting build-class
grades should draw from here; the difficulty philosophy (middling teaches most)
carries over.

| name | Dockerfile URL | base image | est. build | difficulty | mechanisms | natural probe | no escape? (nixpkgs) |
|---|---|---|---|---|---|---|---|
| excalidraw | https://raw.githubusercontent.com/excalidraw/excalidraw/master/Dockerfile | node:24 (BUILDPLATFORM-aware) → nginx:stable-alpine-slim | fast | easy | multi-stage yarn build, static output into nginx, HEALTHCHECK (wget) | HTTP GET / (canvas app loads) | yes — only unrelated `excalidraw_export` CLI |
| parse-server | https://raw.githubusercontent.com/parse-community/parse-server/alpha/Dockerfile | node:20.19.0-alpine3.20 (multi-stage) | fast | easy | multi-stage `npm ci` with dev/prod node_modules split trick, `npm run build`, VOLUME cloud/config | GET /parse/health, :1337 | yes — no matching app found |
| wallos | https://raw.githubusercontent.com/ellite/Wallos/main/Dockerfile | php:8.3-fpm-alpine | fast | middling | apk + `docker-php-ext-install` compiling gd/intl/zip/pdo_sqlite, nginx+php-fpm+dcron multi-process via startup.sh, HEALTHCHECK | GET /health.php (per HEALTHCHECK) | yes — no matching app found |
| nodebb | https://raw.githubusercontent.com/NodeBB/NodeBB/master/Dockerfile | node:lts → node:lts-slim (multi-stage) | medium | middling | multi-stage `npm install` w/ corepack, tini entrypoint, VOLUME node_modules/build/uploads/config | HTTP GET / (forum homepage), :4567 | yes — no matching app found |
| habitica | https://raw.githubusercontent.com/HabitRPG/habitica/develop/Dockerfile-Dev | node:20 | medium | middling | single-stage `npm install` + postinstall + `client:build` + `gulp build:prod`; named "-Dev" but is what habitica's own docker-compose.yml builds for both client and server | HTTP GET / (login page), :3000, needs mongo | yes — only client libs `habiticalib`/`habitipy` |
| directus | https://raw.githubusercontent.com/directus/directus/main/Dockerfile | node:22-alpine (multi-stage) | medium | middling | corepack+pnpm workspace build (`pnpm fetch` → offline install → `pnpm deploy --prod`), pm2-runtime process manager, sqlite default DB | GET /server/ping → "pong", :8055 | yes — no matching app found |
| shlink | https://raw.githubusercontent.com/shlinkio/shlink/main/Dockerfile | php:8.5-alpine (multi-stage) | medium | middling | composer-install build stage, optional RoadRunner runtime binary, multi DB-driver support, non-root user | GET /health, :8080 | yes — only unrelated `hashlink` Haxe VM |
| docmost | https://raw.githubusercontent.com/docmost/docmost/main/Dockerfile | node:26-slim (multi-stage) | medium | middling | pnpm monorepo build (server/client/shared packages), prod-only reinstall in installer stage, non-root `node` user, VOLUME /app/data/storage | HTTP GET / (login page), :3000 | yes — no matching app found |
| filestash | https://raw.githubusercontent.com/mickael-kerjean/filestash/master/docker/Dockerfile | alpine/git (clone) → golang:1.26-trixie (build) → debian:stable-slim (runtime) | medium | middling | 3-stage build, `make build` compiling Go backend linked against libvips/ffmpeg/libraw/libheif C libraries, non-root user | HTTP GET / (connection screen), :8334 | yes — no matching app found |
| baserow | https://raw.githubusercontent.com/baserow/baserow/master/backend/Dockerfile | python (uv-based, many builder stages) | slow | nasty | dedicated builder stages compile su-exec, wasmtime, and QuickJS-NG-as-WASI from source, then uv-managed Python deps for the Django backend; aggressive prod-image size trimming | GET /api/_health/, :8000 | yes — no matching app found |
| grist | https://raw.githubusercontent.com/gristlabs/grist-core/main/Dockerfile | node:22-trixie (builder) → python:3.11-slim-trixie (py collector) → node:22-trixie-slim (runtime) | slow | nasty | TS/webpack app build + embedded Python/Pyodide sandbox + gVisor-unprivileged binary fetch (untrusted-formula sandboxing), tini entrypoint | HTTP GET /status, :8484 | yes — checked `grist` and `grist-core` |
| chatwoot | https://raw.githubusercontent.com/chatwoot/chatwoot/develop/docker/Dockerfile | ruby:3.4.4-alpine3.21 (multi-stage) | slow | nasty | bundler gem install + pnpm JS deps + Rails asset precompile, vips/imagemagick runtime libs, git-sha stamping, dev/prod conditional npm install | HTTP GET / (login page), :3000, needs postgres+redis | yes — no matching app found |

Hunt residue (checked, not added): rocketchat and zulip-server are genuinely absent
from nixpkgs but heavy (Meteor / Python+webpack); teable/wekan/taiga/huly/appwrite/
plane absent but heavy-multi-service; leantime and invoice-ninja absent but their
Dockerfiles only `ADD` prebuilt release tarballs (no build to migrate); ryot's
backend binary is CI-prebuilt (mechanism already represented). Checked and found
PACKAGED (don't re-hunt these; notable corrections: `stump` and `mattermost` ARE
packaged despite first-pass claims otherwise): stump, mattermost, beszel, gatus,
stirling-pdf, firefly-iii, karakeep, linkwarden, vikunja, mealie,
speedtest-tracker, documenso, docuseal, listmonk, umami, n8n, bookstack,
trilium-next, silverbullet, readeck, linkding, wiki-js, stalwart-mail, snipe-it,
librenms, komga, kavita, audiobookshelf, photoview, monica, grocy,
tandoor-recipes, kimai, netbox, soft-serve, diun, woodpecker (server+agent+cli),
outline, gotosocial, pixelfed, misskey, homebox, inventree, wallabag,
actual-server, pairdrop, slskd, dashy-ui, memos, gotenberg, glance,
standardnotes (attr present, server-vs-client ambiguity unresolved), and the
conduwuit successors matrix-continuwuity/matrix-tuwunel.
