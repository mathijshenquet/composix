
2026-07-30 — migrate round N=1, target whoami
verdict: docker build=pass; docker run=pass; docker check=pass; cix build=pass; cix run=pass; cix check=pass.
prompt-gaps verbatim:
- "The prompt did not provide the Docker build context or a pinned source revision; the repository URL was inferred from the Dockerfile URL and fetched as an opaque build input."
- "The prompt says FETCH is the only network access, but does not teach how FETCH commands obtain build tools, bash, or CA certificates; this conversion needed explicit nixpkgs git, bash, go, and cacert paths."
- "The prompt does not explain how to preserve language dependency caches fetched in FETCH for a network-isolated BUILDER; this conversion placed GOMODCACHE under the fetched source tree and reused it in RUN."
- "The prompt requires cix run in check.sh but does not state that the supported system-manager run requires root, nor how a transient unit is torn down; the check uses passwordless sudo and systemctl stop."
- "The prompt does not explain how to accept a changed FETCH output lock; cix required cix build --update-lock."
cix-capability-gaps: none observed in the supported system-manager path. User-manager degraded mode could not provide a reachable listener in this environment, but it is explicitly warned as degraded by cix.
artifacts: corpus/migrate/whoami/{Dockerfile,SOURCE,Cixfile,Cixfile.lock,check.sh,receipt.md}; final item=/nix/store/z7ad7rdyi4wcpbcx37rnynf4k00zvsfi-cix-item-whoami; docker digest=sha256:bf3c544f03d387bd30e9b8bc2e08bc6b6f4aae80d884822fe43e472844ab5d44.

2026-07-30 — Round N=2 (traefik, nats)

traefik: docker build=pass; docker run=pass; docker check=pass; cix build=pass; cix run=pass; cix check=pass. Docker digest=sha256:c864999938e1dfa9b7dfc5ad644d0e9c5f413612cc501dc69d261684417815a3; Cix item=/nix/store/wnxvm6b76y05ay18ixa2vcsvkk1f578h-cix-item-traefik.

nats: docker build=pass; docker run=pass; docker check=pass; cix build=pass; cix run=pass; cix check=pass. Docker digest=sha256:f0f977e50ad69c0b9a041f145cce27df06166295792391f98f4ac415a067756c; Cix item=/nix/store/x0q9whg4ff6khpr23lkmlz5bzlpqjiz6-cix-item-nats.

Prompt gaps (verbatim): traefik Dockerfile: `COPY entrypoint.sh /`; nats Dockerfile: `COPY nats-server.conf /etc/nats/nats-server.conf` and `COPY docker-entrypoint.sh /usr/local/bin`. The task permits only the target Dockerfiles, while docs/migrate.md says: “ENTRYPOINT shell scripts → read them; port the essential env/flag setup into ENV/EXEC lines.” The referenced scripts/configuration are therefore unavailable. Best guesses: Traefik runs with `--ping=true` and probes its candidate-listed `:8080/ping`; NATS runs with `-m 8222` and probes its candidate-listed `:8222/healthz`.

Cix capability gaps: none observed. Docker environmental note: direct Git-context builds failed because Docker required `/usr/bin/git`; final checks fetch the source archive into a temporary context before building the original Dockerfile.
2026-07-30 — migrate round N=4, targets caddy, echo-server, adminer, memcached
verdicts: caddy=check-fail (Docker pass; Cix build/run returned a store item/unit but the bounded HTTP probe failed); echo-server=build-fail (Docker pass; Cix `FETCH` reached the pinned checkout but `cix build .` timed out while npm materialization was still in progress); adminer=pass; memcached=pass.

prompt-gaps verbatim:
- "When the Dockerfile builds from a repo context, FETCH the repo (git clone) and record the resolved revision in your SOURCE notes." The supplied candidate URLs are moving `master` URLs and do not give a revision or a repository-context URL; resolved revisions had to be inferred by cloning the URL's repository.
- "ENTRYPOINT shell scripts → read them; port the essential env/flag setup into ENV/EXEC lines." Adminer's entrypoint dynamically creates design links and enabled-plugin files from arbitrary `ADMINER_DESIGN` and `ADMINER_PLUGINS` values; the prompt gives no declarative mapping for that runtime filesystem mutation. The central login-page probe intentionally covers the base service only.
- "DIR state /var/lib/<name> for persistent data (docker VOLUME maps here), plus cache/logs/run variants." The built cix parser rejects the documented `DIR` directive (`unknown directive \"DIR\"`), so Caddy's Dockerfile-mandated writable XDG config/data directories could not be declared.
- "If a re-fetch legitimately changes the output, accept it with `cix build --update-lock`." The built CLI rejected that documented command without a binder name and required `cix build --update-lock src .`.

cix-capability-gaps:
- The documented `DIR` service directive is not accepted by this cix binary.
- The documented no-argument `cix build --update-lock` interface is not accepted by this cix binary; it requires the lock-bearing binder name.
- Caddy's packaged service did not answer the declared listener after `cix run`; this is a check failure, not an attributed root cause because the conversion protocol permits only the cix binary as a black box.
- Echo Server's npm-cache `FETCH` did not complete within its bounded check timeout; no Cix item was produced.

artifacts: corpus/migrate/{caddy,echo-server,adminer,memcached}/{Dockerfile,SOURCE,context files,Cixfile,Cixfile.lock,check.sh,receipt.md}. Passing Cix items: adminer=/nix/store/6wqrprc1lqkb7g116812x0d4wvkfx17p-cix-item-adminer; memcached=/nix/store/kg6afp6d8dvkwjl8r3qip4yyy3y5lpww-cix-item-memcached.

2026-07-30 — living-receipt refresh after D50–D53
layout: moved Adminer, Memcached, and the complete Echo Server upstream tree into
their pair `context/` directories. Docker checks for those pairs now use
`docker build --file "$root/Dockerfile" "$root/context"`; pair roots contain only
the six declared artifacts. Caddy and Echo Server were layout-only: their known Cix
check failure/timeout remains recorded rather than being represented as a pass.

language: whoami now uses two builder-local FETCH steps (clone, then Go module
download) with a continuation-form PATH and a comment explaining the pin boundary.
The new pins were accepted with `../../../target/debug/cix build --update-lock build .`.

verdicts: whoami=pass, traefik=pass, nats=pass, adminer=pass, memcached=pass.
Refreshed Cix item paths: whoami=/nix/store/y696s2gxr34bvcqzndm8gz2hkkhf9fci-cix-item-whoami;
traefik=/nix/store/wnxvm6b76y05ay18ixa2vcsvkk1f578h-cix-item-traefik;
nats=/nix/store/x0q9whg4ff6khpr23lkmlz5bzlpqjiz6-cix-item-nats;
adminer=/nix/store/6wqrprc1lqkb7g116812x0d4wvkfx17p-cix-item-adminer;
memcached=/nix/store/kg6afp6d8dvkwjl8r3qip4yyy3y5lpww-cix-item-memcached.

exact repro: `cargo build -p cix`; `cd corpus/migrate/whoami && ../../../target/debug/cix build --update-lock build . && ./check.sh cix`; `cd corpus/migrate/traefik && ./check.sh cix`; `cd corpus/migrate/nats && ./check.sh cix`; `cd corpus/migrate/adminer && ./check.sh cix`; `cd corpus/migrate/memcached && ./check.sh cix`.

candidate audit: added the no-escape column using `nix search nixpkgs '^(whoami|traefik|nats|caddy|echo-server|adminer|memcached|ntfy|filebrowser|homer|it-tools|mailpit|nginx|redis|valkey|haproxy|httpd|mosquitto|mysql|mariadb|mongo|postgresql|tomcat|phpmyadmin|registry|vault|minio|syncthing|miniflux|gotify|uptime-kuma|healthchecks|changedetection|freshrss|kanboard|verdaccio|homepage|ghost|vaultwarden|rabbitmq|wordpress|nextcloud|gitea|pihole|dozzle|watchtower)$' --json`; alternate-name search for nats-server/distribution/gotify-server/homepage-dashboard/rabbitmq-server/etc.; and `nix eval --raw nixpkgs#<attr>.pname` checks. Ghost and LinuxServer/nginx remain explicitly ambiguous.

## 2026-07-30 — round N=8

Verdicts and resulting Cixfile class: caddy pass (dissolves); phpmyadmin check-fail (build); verdaccio build-fail (build); dozzle capability-gap (build); watchtower capability-gap (build); nginx run-fail (dissolves); redis check-fail (dissolves); tomcat run-fail (dissolves).

Prompt gaps, verbatim:

- "ENV PORT default=8080"
- "FETCH <name> <cmd> (top-level, empty workdir, binds `${name}`)"
- "EGRESS if the service initiates outbound connections"
- "FILE"
- "A Dockerfile's `COPY` of sibling files ... fetch those files from the same repository directory as the Dockerfile itself"

Black-box observations: the documented ENV form is rejected; `ENV NAME = value` parses. RUN/FETCH requires bash in a declared PATH, so the prompt's top-level FETCH form has no documented way to satisfy the tool. EGRESS accepts no argument. FROM rejects `context` although the prescribed corpus layout places build context there. EXEC does not preserve quoted/escaped multiword argv values, blocking nginx's `-g 'daemon off;'`. Node-generated launchers requiring `/bin/sh` fail in builder sandboxes.

Cix capability gaps: no declared host Unix-socket bind/mount or Docker API capability, so Dozzle and Watchtower cannot faithfully receive `/var/run/docker.sock`; Watchtower's upstream Dockerfile also expects a CI-provided binary absent from the resolved repository context. Docker checks not explicitly marked pass were not promoted without a completed dual-mode transcript.

## 2026-07-31 — corpusfetch start

Scope: replace the ten tracked upstream `context/` trees with one pinned fetch
script. Before any removal, reproduce every recorded repository/revision into an
isolated temporary directory and compare it recursively with the currently
vendored tree, excluding `.git`.

## 2026-07-31 — corpusfetch verification

Pre-removal reproduction command (run once for each candidate):
`git clone --no-checkout <SOURCE repository> <scratch>/<name>; git -C
<scratch>/<name> checkout --detach <SOURCE revision>; diff -r --exclude=.git
corpus/migrate/<name>/context <selected scratch tree>`. All selected trees were
byte-identical:

- `adminer` `7088bf0e2003398930eed5ea00032b61aa3b55cb`
- `dozzle` `c159db5f0b5cccb80e7432d51823eb2702b617bc`
- `echo-server` `2b735482f942cbd889f1d49f3ff892364d0519ac`
- `memcached` `53ac0ecb0bf88b471a0110f8996ce791baf1a667`
- `nginx` `e0f008fab4e1ce252c9451590c6a2aff305dd03c`
- `phpmyadmin` `452a995fe6c90b96473fc17c3d704786c33d42bc`
- `redis` `2ac6f46c6ba6f3ece54183a518a2bfd865390368`
- `tomcat` `f3407586eb54489354943d0d1be0a595a11825d7`
- `verdaccio` `15b3f0c66aa60d1fbd3d7229f730fe827911b5f9`
- `watchtower` `ca0e86e824ec05389ab972ea97d04d4bf0476e90`

Initial raw-checkout diffs found five historic contexts to be deliberately
projected Docker build contexts, not bad revisions. `SOURCE` now makes their
selectors explicit: Adminer `5/` without its Dockerfile and `fastcgi/`;
Memcached `1/alpine/` without its Dockerfile; Echo Server without its separately
tracked Dockerfile; Dozzle without `.claude/`; and Verdaccio without six
development/example paths. The per-candidate receipts record those honest
findings. All other candidates are the checkout root.

Post-removal repro: `cd corpus/migrate && ./fetch.sh --all` completed twice
without error; each run fetched the ten revisions above and installed no `.git`
directory. `./fetch.sh nats` exits 1 with the clear diagnostic `nats SOURCE lacks
a parseable repository URL`. `git check-ignore
corpus/migrate/echo-server/context/README.md` reports
`.gitignore:14:corpus/migrate/*/context/`; `git ls-files
'corpus/migrate/*/context/**'` reports zero files.

Smoke: `./corpus/migrate/fetch.sh echo-server`; `devenv shell -- cargo build`
passed. `cd corpus/migrate/echo-server && ./check.sh cix` reached the current cix
parser but failed before a build: `line 7: PATH was replaced by IMPORT (D58)`.
This historical corpus Cixfile is pre-D58 and remains outside the corpusfetch
scope; no crate source was changed.

Static verification: `bash -n corpus/migrate/fetch.sh` and all ten guarded
context-backed `check.sh` scripts passed; `git diff --check HEAD` passed;
`git diff --name-only HEAD -- crates` is empty. Final diff removes 2,707 tracked
context files and adds the fetch/docs/guard/source-metadata changes only.
## 2026-07-31 — corpus-polish start

- `cd corpus/migrate && ./fetch.sh --all` — pass; freshly fetched pinned contexts for adminer (`7088bf0e2003398930eed5ea00032b61aa3b55cb`), dozzle (`c159db5f0b5cccb80e7432d51823eb2702b617bc`), echo-server (`2b735482f942cbd889f1d49f3ff892364d0519ac`), memcached (`53ac0ecb0bf88b471a0110f8996ce791baf1a667`), nginx (`e0f008fab4e1ce252c9451590c6a2aff305dd03c`), phpmyadmin (`452a995fe6c90b96473fc17c3d704786c33d42bc`), redis (`2ac6f46c6ba6f3ece54183a518a2bfd865390368`), and tomcat (`f3407586eb54489354943d0d1be0a595a11825d7`).
- `devenv shell -- cargo build -p cix` — pass; produced `target/debug/cix`. `bash -n corpus/migrate/*/check.sh` and `git diff --check` also pass.
- Fresh Cix receipts (all after the fetch above, using the D62 member selector in each check script):
  - `cd corpus/migrate/adminer && ./check.sh cix` — pass; `/nix/store/g0yd5br9x691yawy8mjsfx6dd0ssp8mp-cix-item-adminer` served the login page.
  - `cd corpus/migrate/caddy && ./check.sh cix` — pass; `/nix/store/d8aiy4fv1wb3zm6b69h410kfp28q82pi-cix-item-caddy` served its response (after the existing D36 PrivatePIDs fallback).
  - `cd corpus/migrate/dozzle && ./check.sh cix` — non-zero; after the refreshed UI build, the Go-module `FETCH` pin changes between runs. A forced update then reaches the separate missing `shared_cert.pem` source failure. The runtime Docker-socket boundary remains ❌.
  - `cd corpus/migrate/echo-server && ./check.sh cix` — non-zero; network npm materialization completed inside the check window, then the offline webpack launcher failed because its `/usr/bin/env` interpreter is unavailable in the builder union. This remains a non-passing npm-build row.
  - `cd corpus/migrate/memcached && ./check.sh cix` — pass; `/nix/store/i9zz4b30wg3lnjmshj5bkjz51999hpmc-cix-item-memcached` answered `VERSION`.
  - `cd corpus/migrate/nats && ./check.sh cix` — pass; `/nix/store/6nj4ggg4wmfpy6hw6hlp3wwnrn66w6ic-cix-item-nats` answered `{"status":"ok"}`.
  - `cd corpus/migrate/nginx && ./check.sh cix` — pass; `/nix/store/xlnmhf6gwsl8c41q7f8iq241vgs5r102-cix-item-nginx` served HTTP with quote-aware `-g 'daemon off;'` argv.
  - `cd corpus/migrate/phpmyadmin && ./check.sh cix` — pass; `/nix/store/gbfjxsjkc07w22y99jgglmsxf3s0yydb-cix-item-phpmyadmin` served the phpMyAdmin page.
  - `cd corpus/migrate/redis && ./check.sh cix` — pass; `/nix/store/zsjh7kxzh2y2hc3hz4c69hkzxzcc2l63-cix-item-redis` answered `PONG` after the D36 fallback.
  - `cd corpus/migrate/tomcat && ./check.sh cix` — non-zero; `/nix/store/4qpc7nnkf21jkd7bg2wddiyqzisyrdd4-cix-item-tomcat` did not become reachable.
  - `cd corpus/migrate/traefik && ./check.sh cix` — pass; `/nix/store/nnil2w7r861fw33aarp97szdgkzxl33v-cix-item-traefik` answered its ping probe.
  - `cd corpus/migrate/verdaccio && ./check.sh cix` — non-zero during the Corepack/pnpm build sequence; no item produced.
  - `cd corpus/migrate/watchtower && ./check.sh cix` — non-zero after building `/nix/store/49xrwxb2760x3g7zg47n61s6jvg4l3b0-cix-item-watchtower`; the runtime Docker-socket boundary remains ❌.
  - `cd corpus/migrate/whoami && ./check.sh cix` — pass; `/nix/store/5raa2baz0ixyj7lrhqzrcdbpvf8rlj0i-cix-item-whoami` served the HTTP probe.
- Lock refresh required by the changed dozzle builder graph: `cd corpus/migrate/dozzle && ../../../target/debug/cix build --update-lock ui .#dozzle` and `../../../target/debug/cix build --update-lock build .#dozzle`; the final ordinary check still correctly reports the unstable Go-module FETCH pin above.
- Gate smoke: `devenv shell -- cargo test --workspace` — pass. Final static repros: `for file in corpus/migrate/*/check.sh; do bash -n "$file"; done`; `git diff --check`; `git ls-files 'corpus/migrate/*/context/**'`; and `git diff --name-only | rg -v '^corpus/migrate/' && exit 1 || true` — all pass (the context listing is empty and the diff is corpus-only).
- Cleanup repro: `systemctl --user reset-failed 'cix-*'; systemctl --user stop cix-run.slice; sudo -n systemctl reset-failed 'cix-*'; sudo -n systemctl stop cix-run.slice` — completed; test-created units are stopped/reset.

## 2026-07-31 — D66 absolute artifact destinations

- `cd corpus/migrate && ./fetch.sh --all` — pass; all eight fetched contexts
  were refreshed from their pins before the Cixfile sweep.
- After `devenv shell -- cargo build -p cix`, the previously green receipts
  were re-run with their exact commands: `cd corpus/migrate/{adminer,caddy,
  memcached,nats,nginx,phpmyadmin,redis,traefik,whoami} && ./check.sh cix`.
  All nine passed using D66 absolute COPY/LINK destinations; caddy, nginx,
  phpmyadmin, and redis used the existing D36 PrivatePIDs fallback on this
  host. The known non-green dozzle, echo-server, tomcat, verdaccio, and
  watchtower rows were not promoted.

## 2026-07-31 — D58 `/usr/bin/env` addendum

- `cd corpus/migrate && ./fetch.sh echo-server` refreshed Echo Server's pinned
  context. Its first ordinary `./check.sh cix` stopped at the historical git
  FETCH pin, so `cd corpus/migrate/echo-server && ../../../target/debug/cix
  build --update-lock build .#echo-server` refreshed only that builder's pins
  and reached the intended launcher proof: webpack's wrapper failed at
  `/usr/bin/env` with the new explicit coreutils IMPORT hint.
- Adding `${pkgs.coreutils}` to Echo Server's builder IMPORT is the explicit
  provisioning required by D58; no ambient software was added. Repro:
  `cd corpus/migrate/echo-server && ../../../target/debug/cix build
  --update-lock build .#echo-server && ./check.sh cix`. It passed: webpack
  compiled with its eight upstream warnings and the service passed its bounded
  HTTP probe. Final item:
  `/nix/store/mjvw61rg51b8zv3qvmz81n2rhphnn6is-cix-item-echo-server`.

## 2026-07-31 — migrate r5 start

- Scope loaded from `docs/migrate.md` and `.dev/specs/track-migrate-r5.md`; worktree
  began clean on `track/migrate-r5`, with direnv active.
- `php.withExtensions` is evaluator-level package customization and therefore outside
  Cixfile by D32. The Wallos pair will exercise D4's first-class `.nix` escape hatch,
  as required by the track spec; this is part of the PHP-class result, not silently
  treated as native Cixfile support.
- Exact compiler command started: `devenv shell -- cargo build`.
- `devenv shell -- cargo build` — pass in 11.06s; produced this worktree's
  `target/debug/cix` without modifying `crates/**`.
- Candidate pin discovery commands: `git ls-remote <repository> refs/heads/<branch>`
  resolved Excalidraw `786ab266ff3a9cfffaed16804cf9132b44bc08ae`, Parse Server
  `315e157637d902d85f465563b2863a9e19bf1ff4`, Wallos
  `3a7f965d0412b40ca29a678c90f0c830bc7e3faa`, Directus
  `b1d7a45a77661fd13928a53448c06649f36b56f5`, and Filestash
  `cdcb9566d4d24c065e461b1c8e3220ff68ef98ac`.
- Context materialization: `cd corpus/migrate && ./fetch.sh excalidraw &&
  ./fetch.sh parse-server && ./fetch.sh wallos && ./fetch.sh directus &&
  ./fetch.sh filestash` — pass; all five ignored trees checked out exactly at
  their SOURCE revisions with `.git` removed.
- Excalidraw contract read before conversion: the pinned Dockerfile copies the full
  monorepo, runs Yarn 1.22.22 with `yarn.lock`, invokes root
  `build:app:docker` → `excalidraw-app`'s `cross-env ... vite build`, copies only
  `excalidraw-app/build` into nginx, and probes `/` with wget. The Cix service
  retains that static output and probe; Cixfile cannot encode the Docker
  `HEALTHCHECK`, classified as a product gap even if the paired probe passes.
- Excalidraw Docker receipt: `cd corpus/migrate/excalidraw && ./check.sh docker`
  — pass on 2026-07-31; image
  `sha256:6b5d15281cc14f4a9f8a1f3b4323171c899863b1a6c45173135c68232b7cddd3`
  served an HTML document containing `Excalidraw Whiteboard` at the natural `/`
  probe. Build and probe completed well below the 20-minute slow-tier cutoff.
- Excalidraw Cix receipt: `cd corpus/migrate/excalidraw && ./check.sh cix` —
  pass; Yarn dependency FETCH completed in 14.849s, offline Vite build in 10.919s,
  item `/nix/store/7y17li69l72zz85lkp7bpka7bgnb7dqy-cix-item-excalidraw`
  served the identical title probe under the D36 degraded PrivatePIDs fallback.
  Classification: node attempted 1/passed 1; residual Cix health-edge omission is
  a product gap, not a behavioral probe failure.
- Parse Server contract read before conversion: production starts
  `node ./bin/parse-server`, requires application ID, master key, server URL, and
  an external MongoDB/PostgreSQL database; `/parse/health` reports the server
  lifecycle state. Its two Docker volumes are writable cloud/config inputs and
  its default file logging needs an explicit log role directory. The pair uses
  one bounded MongoDB 8.0.4 companion in each mode, marks the master key secret,
  and declares outbound DB access with `GRANT egress`.
- Parse Server Docker receipt: `cd corpus/migrate/parse-server && ./check.sh docker`
  — pass on 2026-07-31; MongoDB companion image digest
  `sha256:aaad67f2dca93148e5343c03210bcfc89a0107516a4756bfa018acd6579e5b18`,
  Parse Server image
  `sha256:395ee46833dd658437dcaedcba0d0ed3bea2e2b4cf03e17bd41540344bbb7289`,
  and `/parse/health` returned `{"status":"ok"}`.
- Parse Server first Cix attempt: `cd corpus/migrate/parse-server && ./check.sh cix`
  — build-fail before execution: D22 correctly rejects artifact destination `/lib`.
  This was a conversion error, not a product/language loss; the application tree was
  moved intact under `/parse-server` so the entry script's `../lib` resolution remains
  faithful without projecting into a denied system path.
- Parse Server second Cix attempt: the same exact command — build-fail during
  parsing because `CONFIGDIR /parse-server/cloud` is outside the directive's enforced
  `/etc/<one-component>` role namespace. Corrected by managing
  `/etc/parse-server-{cloud,config}` and linking the Docker-visible volume paths to
  those writable directories; this preserves the application paths while using
  systemd's configuration-directory contract.
- Parse Server third Cix attempt: the same exact command — build-fail while
  resolving `${pkgs.nodejs_20}` because current locked nixpkgs removed upstream-EOL
  Node 20 on 2026-04-30. Upstream Parse Server's own `engines` explicitly supports
  Node `>=22.13.0 <23.0.0`, so the conversion selected `${pkgs.nodejs_22}` and
  records this Docker-image/package version mismatch rather than pretending the
  literal 20.19 runtime survived.
- Parse Server fourth Cix attempt: the same exact command — build passed and
  produced `/nix/store/f8kikjhwvgx23yhac62cmildy57by6gd-cix-item-parse-server`,
  but `cix run` rejected the broken-at-build-time links from Docker's
  `/parse-server/{cloud,config}` paths to runtime-created `CONFIGDIR`s. There is
  no faithful current role-dir spelling for writable configuration outside
  `/etc/<one-component>`; classified as a product gap. The unused approximation
  was removed. The central server/DB health contract remains testable, while the
  receipt explicitly does not claim volume-path fidelity.
- Parse Server fifth Cix attempt: the same exact command — `npm ci` completed,
  then its builder `FETCH` output differed from the first pin
  (`sha256-tAgfKPquYLEt9DBcLDmulhTCr8JjHLCXek/UZIMh33o=` →
  `sha256-7bmIJlypytNVOcxkptjFPw8HVu0KVxQLW6WN9ZpgbV0=`). Before classifying
  this node row as a pin-stability loss, the new output will be accepted once with
  the compiler-prescribed `--update-lock build` and immediately retested normally.
- Parse Server lock acceptance: `cd corpus/migrate/parse-server &&
  ../../../target/debug/cix build --update-lock build .#parse-server` — pass;
  produced `/nix/store/kf4d06323935ym7y4bllbprjidzn367b-cix-item-parse-server`.
  Immediate ordinary retry `cd corpus/migrate/parse-server && ./check.sh cix` —
  build-fail again: accepted
  `sha256-Z0u3uJu6NH9y7FYCFfmdM+ecjvLvxgg2rnAHcCfR7lY=` changed to
  `sha256-kHA3Y9J6woNYANJAP5X5kT7OLuNFdXAAY59GXT1auvo=`. The required stable
  pin cannot be obtained from repeated `npm ci` outputs, so the Cix runtime was
  not promoted from an unrepeatable build. Classification: product gap in
  FETCH/package-manager output stability. Node class now attempted 2/passed 1.
- Wallos contract read before conversion: upstream starts nginx, PHP-FPM, and cron
  under one signal-forwarding shell; initializes and migrates SQLite; persists
  `/var/www/html/db` and `/var/www/html/images/uploads/logos`; runs scheduled PHP
  jobs; and probes `/health.php` for exact `OK`. Its `gd`, `intl`, `zip`,
  `pdo_sqlite`, and `calendar` extensions require `php.withExtensions`, so the
  implementation uses D4's pinned `default.nix` escape rather than inventing
  Cixfile syntax. The dynamic-user item maps the two writable app paths through
  symlinks into `STATEDIR`-equivalent `/var/lib/wallos`, and replaces root-only
  cron with unprivileged supercronic while retaining the upstream schedule.
- Wallos Docker receipt: `cd corpus/migrate/wallos && ./check.sh docker` — pass
  on 2026-07-31; image
  `sha256:103ffa469b3455ebb5609802c124b6d7202bdd8606690c6e09ae18bc605eae46`
  returned exact `OK` from `/health.php`.
- Wallos first Cix receipt: `cd corpus/migrate/wallos && ./check.sh cix` — Nix
  build passed, producing
  `/nix/store/chdrq4if7wwvxkxzrmk9aw2dsmrfvq57-cix-item-wallos`; run-fail.
  Setup successfully created and migrated SQLite through migration 53, then nginx
  exited because the hardened unit has no usable `/dev/stdout` device. The setup
  also exposed PCRE's executable-memory expectation. Corrected the access log to
  declared `/var/log/wallos` and added the exact `jit` grant; this was runtime
  contract discovery, not a pass claim.
- Wallos second Cix receipt: the same exact command — Nix build passed, producing
  `/nix/store/fhfbvd1q90hahxnf8p5gmjg8klwdzfvj-cix-item-wallos`; run-fail.
  nginx and supercronic started, but PHP-FPM rejected `/proc/self/fd/2` under the
  hardened device policy just as nginx had rejected `/dev/stdout`. Its error log
  was moved to the already declared `/var/log/wallos` role directory.
- Wallos third Cix receipt: the same exact command — Nix build passed, producing
  `/nix/store/vlnfj18gqm80c4s1lywri6jr2ksai5zc-cix-item-wallos`; run-fail.
  PHP-FPM's internal `127.0.0.1:9000` bind was correctly denied because the
  manifest grants only nginx's public port. Switched the private nginx↔FPM edge
  to `/run/wallos/php-fpm.sock`, matching upstream's alternate supplied nginx
  config and the declared `RUNDIR`, without exposing a second TCP port.
- Wallos final Cix receipt: `cd corpus/migrate/wallos && ./check.sh cix` — pass;
  `/nix/store/gm8yx7vsm3izqvfn94ji8jmqndggrm0r-cix-item-wallos` completed
  idempotent DB setup/migrations, started PHP-FPM, nginx, and supercronic under
  DynamicUser, and returned exact `OK` from `/health.php` after the D36 fallback.
  PHP class attempted 1/passed 1. Escape-hatch finding: expressing
  `php.withExtensions` was direct and reviewable, but required hand-authoring the
  whole item manifest, service scripts, mounts, and pin; D4 is capable but a much
  steeper migration than Cixfile. D48 health-edge omission remains a product gap.
- Directus contract read before conversion: Node 22/pnpm 10 monorepo performs a
  pinned dependency fetch, offline recursive install/build, production deploy,
  then `cli.js bootstrap` before PM2 starts `cli.js start`. PM2 dissolves into
  systemd; SQLite, extensions, and uploads are symlinked from the immutable app
  tree to `/var/lib/directus`; the natural probe is exact `pong` from
  `/server/ping`.
- Directus Docker receipt: `cd corpus/migrate/directus && ./check.sh docker` —
  pass on 2026-07-31; image
  `sha256:1f0b93fb3d7cbb737e1b13e164c01266a85b824fe9510195492bd768435b1498`
  returned exact `pong`.
- Directus first Cix receipt: `cd corpus/migrate/directus && ./check.sh cix` —
  build-fail in pnpm's nixpkgs wrapper because it invokes bare `sed`. Added the
  missing whole-package `${pkgs.gnused}` IMPORT, exactly following the migration
  prompt's builder provisioning rule; conversion error, not a class loss.
- Directus second Cix receipt: the same exact command — build-fail during pnpm's
  sqlite3 native fallback because current `${pkgs.python3}` is Python 3.14 and
  node-gyp 8 imports removed stdlib `distutils`; upstream Docker explicitly adds
  Python plus setuptools. Selected still-packaged `${pkgs.python311}`, whose
  stdlib retains the interface required by this pinned node-gyp. This is a
  package/toolchain version mismatch correction, not yet a class verdict.
- Directus third Cix receipt: the same exact command — Python 3.11 allowed the
  sqlite3 native fallback to compile, but the FETCH output disagreed with its
  prior pin (`sha256-gcvNcwjgcSI1zemo9XMLCvB5bz9fLFeTqxLikykWEJk=` →
  `sha256-S9eT81DZxrGwAvv4v4wjT7SzbcZRdqDpsSxFUTBcLI0=`). As with Parse
  Server, accept once then immediately ordinary-retest before assigning the
  final pin-stability classification.
- Directus lock acceptance/build continuation: `cd corpus/migrate/directus &&
  ../../../target/debug/cix build --update-lock build .#directus` — FETCH and
  native sqlite3 compilation passed, but offline `pnpm run build` failed when
  pnpm's downloaded `sass-embedded-linux-x64` tried to spawn its FHS-linked
  `dart` executable (`ENOENT`). With `lib` deliberately absent from IMPORT (D58),
  the builder has no declared way to supply that prebuilt ELF interpreter/loader.
  Classification: language gap. No runtime item or Cix pass claimed. Node class:
  attempted 3/passed 1.
- Filestash contract read before conversion: upstream clones its branch inside
  Docker rather than consuming the supplied context, runs `go get ./...` plus five
  `go generate` directives, and links both system cgo libraries and bundled static
  archives before copying `dist` into a Debian runtime. The Cix pair instead uses
  the SOURCE-pinned context, provisions every named build package, maps its mutable
  `data/state` tree to `/var/lib/filestash` through `FILESTASH_PATH`, and keeps
  ffmpeg/poppler runtime helpers explicit. The natural `/` probe validates the
  upstream `X-Powered-By: Filestash` response even when first-run setup redirects.
- Filestash initial Docker command: `cd corpus/migrate/filestash && ./check.sh docker`
  built image `sha256:903502d3fc0f4ccaa5dc88d4ac037a88960647f81ccc4149706f37f77cfd8ff8`,
  but the first check implementation wrongly searched the empty body of the valid
  `/` 307 response. Classified as a probe error, not an application failure; the
  probe was corrected to assert the product header on the natural endpoint.
- Filestash Docker receipt: `cd corpus/migrate/filestash && ./check.sh docker` —
  pass on 2026-07-31; image
  `sha256:c5bfe487160aa1eb52d3b1afb84ebea8c403e782119ff1594329de43cb54ab51`
  returned `X-Powered-By: Filestash` at `/`. Upstream's Dockerfile clones moving
  `master` despite the recorded SOURCE revision, so its independent build is an
  honest upstream reproducibility weakness; the Cix input itself remains pinned.
- Filestash first Cix attempt: `cd corpus/migrate/filestash && ./check.sh cix` —
  build-fail because three upstream `go generate` programs fetch cdnjs and GitHub,
  while the conversion initially placed generation in an offline `RUN`. Classified
  as a conversion error: generation was moved into the networked `FETCH`, matching
  upstream `make init`, while compilation remains offline.
- Filestash second Cix attempt: the same exact command — development headers and
  ffmpeg pkg-config metadata were not added to compiler search paths merely by
  whole-package IMPORTs; compilation reported missing Brotli/LibRaw headers and
  all libav `.pc` files. This was a conversion error. The necessary dev outputs
  and explicit `CPATH`, `LIBRARY_PATH`, and `PKG_CONFIG_PATH` flags were added.
- Filestash third Cix attempt: the same exact command — build-fail before compile
  because the combined network generator output changed from
  `sha256-n2k0tbaO2MlzScCJVi0fYVzHd/C5VQPPzAQsKDZ7grk=` to
  `sha256-8MT+jdjGPLslQ6N/V4fWSl1mRJrzDr1HwBaJdkxj8Dk=`. As a bounded diagnostic,
  `cd corpus/migrate/filestash && ../../../target/debug/cix build --update-lock
  build .#filestash` accepted one output and continued to compilation.
- Filestash first lock-acceptance continuation: that exact command — compile-fail
  after reaching the image plugins because jpeg/png/webp/gif development headers
  were still absent. These are split nixpkgs outputs rather than missing packages;
  all available dev outputs were added explicitly, with libwebp/giflib headers
  taken from their single outputs.
- Filestash second lock-acceptance continuation: `cd corpus/migrate/filestash &&
  ../../../target/debug/cix build --update-lock build .#filestash` — compile reached
  the final linker, which could not find the literal static archives demanded by
  upstream cgo (`libwebp.a`, `libjpeg.a`, `libpng.a`, `libz.a`, `libraw.a`, plus
  lcms2). Normal nixpkgs outputs intentionally ship most of these only shared.
  This confirmed the Go+cgo shape rather than a missing header/import mistake.
- Filestash final bounded attempt: the same exact lock-acceptance command after
  selecting explicit `pkgsStatic` outputs — closure-fail in locked nixpkgs itself:
  `giflib-static-x86_64-unknown-linux-musl-6.1.3` tries to link `libgif.so` using
  `crtbeginT.o` and fails with relocation `R_X86_64_32` against `__TMC_END__`;
  both static libwebp derivations and the remaining closure are cancelled.
  Cixfile cannot express the package override needed to build a tailored glibc
  static dependency set (D32). Classification: language/package-selection gap;
  the changing generated FETCH is separately a product pin-stability gap. Go+cgo
  class attempted 1/passed 0.
- Legacy context materialization: `cd corpus/migrate && ./fetch.sh tomcat &&
  ./fetch.sh dozzle` — pass; both ignored trees match their recorded SOURCE pins.
- Tomcat root-cause reproduction: `cd corpus/migrate/tomcat && item=$(../../../target/debug/cix
  build .#tomcat); unit=$(sudo -n ../../../target/debug/cix run --detach "$item" |
  tail -n1); ... journalctl ...` — the original item exited immediately because
  the sole linked `catalina.sh` lacked `uname`, `ls`, `expr`, and `dirname`, then
  resolved its symlink under `/bin` and reported `Cannot find //bin/setclasspath.sh`.
  Root cause: incomplete artifact/runtime closure, not a network reachability bug.
- Tomcat repair attempt 1: `cd corpus/migrate/tomcat && ./check.sh cix` — parse-fail,
  because `IMPORT` is builder-only. The complete coreutils package was instead
  copied into the service artifact and added to runtime PATH.
- Tomcat repair attempt 2: the same exact command — run validation rejected direct
  EXEC `/tomcat/bin/catalina.sh`; retained the complete package tree but added the
  required executable projection at `/bin/catalina.sh`.
- Tomcat repair attempt 3: the same exact command — setup-fail because nixpkgs
  Tomcat has no `webapps/` directory. Removed the invalid seed copy; an empty
  writable deployment directory correctly represents the no-application image.
- Tomcat repair attempt 4: the same exact command — Tomcat reached JVM startup,
  then failed `Failed to mark memory page as executable`; added the exact `jit`
  grant required by the JVM.
- Tomcat repair attempt 5: the same exact command — Tomcat bound port 8080, then
  its undeclared private shutdown socket on 8005 was denied and it exited. Setup
  now idempotently changes only that listener to `port="-1"`; it also creates the
  writable Catalina deployment directory.
- Tomcat repair attempt 6: the same exact command — setup-fail because `cp -a`
  preserved the read-only Nix source modes in `CATALINA_BASE`. Switched to GNU
  `cp --no-preserve=mode,ownership`; before retesting, exact disposable test state
  was audited and reset with `sudo -n rm -rf /var/lib/private/cix-run-tomcat
  /var/log/private/cix-run-tomcat /run/private/cix-run-tomcat`.
- Tomcat final Cix receipt: `cd corpus/migrate/tomcat && ./check.sh cix` — pass;
  item `/nix/store/zb77wa6z6fka2yx3dmz9s3b0wnbb7d3w-cix-item-tomcat` responded at
  natural `/` on 8080 under the D36 fallback. The candidate says “responds,” so
  the probe now correctly accepts Tomcat's expected no-webapp HTTP 404 rather than
  requiring a success body. Legacy language/product verdict: fixed conversion;
  no remaining Cix gap beyond the documented degraded host fallback.
- Dozzle current legacy command: `cd corpus/migrate/dozzle && ./check.sh cix` —
  build-fail even before the recorded Go FETCH: UI `pnpm fetch --ignore-scripts`
  changed from `sha256-Dynp64Cg8LvghhD7NTFFHwjO6OBnlTs5X9G0E+VtRx8=` to
  `sha256-odtJxzY8QgJHbwT85Auvy+3CaBfSRugnCoeuuFvSSs8=`. Not fixed, per spec.
- Dozzle byte-level Go reproduction command: create two clean `mktemp` run dirs,
  copy pinned `context/{go.mod,go.sum}` into each, then run
  `GOMODCACHE=<run>/cache $(nix build --no-link --print-out-paths --quiet
  nixpkgs#go_1_26)/bin/go mod download` once per directory; compare with
  `diff -qr`, sorted `find ... -type f -printf '%P\\n'`, `wc -c`, and `sha256sum`.
  Outcome: all 33,862 common files were byte-identical. Run 1 alone contained
  seven `cache/download/sumdb/sum.golang.org` tiles: `tile/8/0/x227/780`
  (8192 bytes, sha256 `5ce1866f55639c30e34a8a4263205c463eb33139b0ac61c16eb73885aff666a4`),
  `tile/8/0/x227/780.p/82` (2624, `04139b3c45aa7d142dd22b6a8efcd1d30d25cdfa1ccf3f0c49dde689e84e1943`),
  `tile/8/0/x228/452.p/108` (3456, `128cf09fa3fa0cd5e449d7271fb746f6a66685bbc4856196cc081e0dc991216c`),
  `tile/8/1/889` (8192, `9fb9ad926448981cf511d1f6a80447c405219f96b85f22007cab313767af09cf`),
  `tile/8/1/889.p/196` (6272, `7cb74e7014427857c5cf794619570efbb8164a34a9c0c865a30b9503709fa249`),
  `tile/8/1/892.p/100` (3200, `483bc44b9cc5dff03517b57ed07048e7d3460eafcdc15e7abb003174ef1f68ff`),
  and `tile/8/2/003.p/121` (3872, `7372f71815c1761628e9786325a1e42c0bf8d169f7d94fcc926505e89cb2bd68`).
  They total exactly 35,808 extra bytes, equal to the total tree-size difference;
  there are no differing bytes in common files. Classification: product gap—FETCH
  pins incidental concurrent Go sumdb cache population, not just dependency source.
  Independently, Dozzle's required Docker socket remains an explicit product boundary.
- Batch grade: Node attempted 3/passed 1 (Excalidraw pass; Parse Server product
  FETCH-stability gap; Directus language gap around an FHS-linked downloaded native
  tool), PHP attempted 1/passed 1 through D4's `.nix` escape, and Go+cgo attempted
  1/passed 0 (Filestash language/package-selection gap). Overall new build-class
  loss is 3/5 pairs. Every candidate's Docker build and natural probe passed.
- Complete classification ledger: language gaps — Directus downloaded FHS ELF and
  Filestash's required customized static-library set cannot be expressed by a plain
  package selector; product gaps — repeated npm/go-generator/sumdb FETCH snapshots,
  writable Parse volume paths outside current role-dir grammar, Docker HEALTHCHECK
  edges omitted by Cix manifests, and Docker-socket workloads; prompt gap — the
  track suggested source-dependent CGO flags through builder `ENV`, but builder ENV
  is deliberately plain text and rejected `${pkgs...}` interpolation, so the usable
  spelling is an inline environment on `RUN`; upstream flakes — Filestash Docker's
  moving-master clone and locked nixpkgs's broken `pkgsStatic.giflib` build. Proposed
  design-round inputs for the orchestrator: define which package-manager cache bytes
  FETCH should normalize/exclude, consider an expression escape for customized build
  packages (or document mandatory `.nix` fallback), and reconcile health edges and
  writable application-path projections without weakening role ownership.
- Final smoke gate: `devenv shell -- cargo test --workspace` — pass on 2026-07-31;
  all workspace unit/integration/doc tests passed (two intentional ignored tests).
  Devenv's generated untracked root `devenv.lock` was removed after the gate so the
  track remains confined to `corpus/migrate/**`.
- Final cleanup/audit: `sudo -n systemctl list-units 'cix-run-*' --all --no-legend
  --plain`, `docker ps --format '{{.ID}} {{.Names}} {{.Ports}}' | rg
  'migrate-r5|migrate-r4'`, and `git ls-files 'corpus/migrate/*/context/**'` all
  returned empty. `git diff --check` passed. The two-run Dozzle scratch directory
  was removed after its exact hashes were recorded. Next: commit the corpus-only diff.

## 2026-07-31 — D69 FETCH consumed-set re-check

- Follow-up after independent D69 lock-churn verification: automatic FETCH
  replay `storePath` values are now local cache data rather than serialized
  lock fields. The exact Parse Server repro used two fresh
  `CIX_BUILD_WORKSPACE_DIR` values and `TMPDIR=/tmp` for consecutive
  `cix build --update-lock build .#parse-server` commands; its resulting
  locks were byte-identical (sha256
  `1e5a2a6f69f716245fc1434b6b0a064165518951c2511fe21ddc9be1e4ed9bb2`)
  with no fetch `storePath`, then a fresh-workspace `--cold` replay completed
  both offline Parse Server suffix RUNs. ProjB's corresponding two clean
  updates were byte-identical and its ordinary build memo-hit. Dozzle's
  documented whole conversion remains an honest failure: a forced current
  refresh reaches the pre-existing missing `shared_cert.pem` build-source
  boundary, so its lock was not refreshed; the disposable backend-only
  receipt below remains the Go consumed-set proof.

- Fresh contexts: `cd corpus/migrate && bash ./fetch.sh parse-server && bash
  ./fetch.sh dozzle` — pass at their recorded revisions.
- Parse Server: two `cd corpus/migrate/parse-server && ../../../target/debug/cix
  build --update-lock build .#parse-server` runs followed by
  `../../../target/debug/cix build .#parse-server` — pass. The lock now records
  the seven consumed final paths and a FETCH replay snapshot; both double-fetch
  probes name the known `.npm` cache/index/debug-log files and sizes, while all
  consumed hashes retain the former stable values. This closes the false
  whole-workdir pin failure; no runtime health-pass claim is added here.
- ProjB: from the repository root, `target/debug/cix build --update-lock build
  examples/build/projB#projb` twice, then `target/debug/cix build
  examples/build/projB#projb` — pass. The probe records only the 57,344-byte
  `.cargo/.global-cache`; the consumed `target/release/projb` pin is stable and
  the ordinary build memo-hits.
- Dozzle remains split honestly: in disposable `/tmp/pinkeys-dozzle-backend`, a
  backend-only Cixfile using the recorded source plus minimal generated `dist`
  and placeholder absent cert files ran `/home/mathijs/composix/.worktrees/pinkeys/target/debug/cix
  build --update-lock build .` twice, then `.../target/debug/cix build .` — all
  pass, same `/nix/store/l8l5vlf0v4zzhz0vv7xrimqm7la36l3d-cix-item-proof`, and
  the final build memo-hit. Its automatic FETCH pin contains only `dozzle`; the
  sumdb tile paths are probe facts, hence unconsumed. The UI's pnpm/Vite `dist`
  remains a consumed-byte instability and is recorded, not excluded or normalized
  by cix. The required Docker socket remains an independent runtime boundary.

## 2026-08-04 — corpus gap-ledger track started

- Loaded `AGENTS.md`, `.dev/specs/track-corpusgaps.md`, the living-corpus
  maintenance contract, the migration prompt, and the existing migration journal.
  The worktree is clean on `track/corpusgaps` and `DEVENV_ROOT` points at this
  worktree. Scope is fenced to the 21 new `GAPS.md` files, `docs/corpus.md`,
  `docs/migrate.md`, and this journal; no Cixfile, context, lock, check, receipt,
  generated browser output, or Rust source will be changed.
- Provenance audit started from each case's introducing commit and the most recent
  `docs/migrate.md` revision in its ancestry. Next: desk-review every Dockerfile,
  Cixfile/compose file, receipt, and relevant source-side configuration before
  writing the routed ledgers.

## 2026-08-04 — all 21 case ledgers drafted

- Added one `GAPS.md` beside every living migration. Each has the required
  provenance/status header and every prose bullet ends in a routing arrow. Early
  converter batches are attributed to `terra` because
  `.dev/specs/track-migrate.md` fixes that model; later/manual feature-wave cases
  say `unknown` because the repository does not establish their generating model.
- Desk review covered every checked-in Dockerfile or upstream compose/CronJob,
  current Cixfile/member set, SOURCE, and receipt. Findings beyond the seed include
  Caddy's unrepresentable UDP/QUIC port, Adminer/nginx stop-signal loss, dropped
  Excalidraw `NODE_ENV`, Dozzle's `/data` divergence, NATS's missing config/cluster
  listener, phpMyAdmin's large config/extension delta, Renovate's version-only APP,
  Wallos's retained in-unit supervisor, and source/receipt parity weaknesses.
- Stale statuses are limited to conversions that can already consume a landed
  feature: Caddy/Dozzle/Parse Server/Verdaccio (CIP-82), Excalidraw/Wallos
  (CIP-79), and Renovate (CIP-81). Unadopted artifact-import,
  builder-dev-imports, and FILE…FROM findings remain current and link their drafts.
- Static receipt: `find corpus/migrate -mindepth 2 -maxdepth 2 -name GAPS.md |
  wc -l` printed `21`; a local header/bullet routing audit found no malformed
  ledger, and `git diff --check` exited 0. Next: teach every routed prompt lesson
  in `docs/migrate.md`, then replace the living table's single ribbon with separate
  fidelity and evidence axes.

## 2026-08-04 — migration prompt closes routed prompt gaps

- Amended `docs/migrate.md` in place, in its existing teaching flow: the opening
  contract now requires an explicit disposition for every upstream ENV/config/
  tuning knob; graph selection warns that many sibling copies usually hide one
  deploy unit; FETCH guidance preserves upstream version/checksum binders as well
  as the Cix SRI; runtime assembly mirrors upstream application and role paths by
  default; and verification makes the per-case `GAPS.md` contract mandatory.
- `rg -n '→ prompt$' corpus/migrate/*/GAPS.md` returned 12 findings. Each is
  covered by one of those general lessons; no image-specific exception or syntax
  from an unadopted draft was taught. `git diff --check` exited 0. Next: regrade
  the living table on independent fidelity/evidence axes and cross-check every row
  against its ledger.

## 2026-08-04 — living corpus regraded on two axes

- Replaced the single living-corpus ribbon with independent Fidelity and Evidence
  columns. Fidelity now says faithful/declared losses/blocked/refused plus one
  case-specific clause; Evidence says build/runtime probe/closed-root and links
  the relevant receipt. The closed-root labels also link the exhaustive CIP-84
  audit section so the stronger tier is not attributed to an older per-case
  receipt. Sections 1–3 retain their existing survey grades unchanged.
- Caddy is the canary: its sealed-root `caddy respond` receipt remains strong
  evidence, while Fidelity now names it as a toy that omits upstream config,
  ports, state layout, and the faithful twin. The same separation prevents the
  Memcached/NATS/nginx/phpMyAdmin/Redis/Tomcat/Traefik probes and Renovate's
  version-only timer from implying untested parity.
- Follow-up correction to the earlier stale roster: Filestash is also stale with
  D70. Artifact `IMPORT` still needs its draft, but D70 overlays already provide
  the `.nix` package-customization escape for the broken static package set; that
  finding is therefore routed back to the case, not promoted as new language.
- Static receipt: the living-table slice contains exactly 21 numbered rows and
  `git diff --check` exited 0.

## 2026-08-04 — Language-gap candidates for the orchestrator

- Builder support for downloaded FHS-linked ELF interpreters — Directus — bare
  builder `IMPORT` cannot provide/patch the requested dynamic loader; artifact
  imports/runtime-path links and development header/library search paths do not
  change an executable's ELF interpreter.
- Protocol-aware inbound ports, beginning with UDP — Caddy — the manifest/`PORT`
  surface records TCP only, so the upstream 443/UDP QUIC listener cannot be
  declared or sandbox-authorized; neither existing import draft touches network
  protocols.
- Per-service stop signal/timeout semantics — Adminer and nginx — the upstream
  SIGINT/SIGQUIT contracts fall back to systemd's default because manifests expose
  neither `KillSignal=` nor `TimeoutStopSec=`; this is already recorded as an open
  mechanical field, not covered by either import draft.
- Exact named/multi-network membership and internal-network egress policy —
  Mastodon — CIP-86's pod namespace gives isolation and egress machinery but
  cannot express the upstream external+internal memberships or `internal: true`;
  D26/D27 remain the recorded frontier and the import drafts are orthogonal.

## 2026-08-04 — corpus-gaps final agent gate

- `devenv shell -- cargo fmt --all --check` — exit 0.
- `devenv shell -- cargo run -p cix -- fmt --check examples` — exit 0 (the
  synchronous cached rerun is the receipt after the first invocation compiled the
  worktree binary).
- `devenv shell -- cargo clippy --workspace --all-targets -- -D warnings` —
  exit 0.
- `devenv shell -- cargo test --workspace` — exit 101 for exactly
  `corpus_browser_matches_committed_pages`: the old generated
  `docs/corpus/index.html` contains the former ribbons/evidence, while the
  deterministic in-memory render contains this track's new Fidelity/Evidence
  content. `generated_corpus_browser_is_deterministic` passes. This is the
  intentional concurrent-track merge seam: `.dev/specs/track-corpusgaps.md`
  forbids touching `docs/corpus/`, and `.dev/specs/track-browser3.md` assigns that
  output plus the generator to browser3 with an explicit orchestrator-regeneration
  merge note. The forbidden regeneration was not performed.
- `devenv shell -- cargo test --workspace -- --skip
  corpus_browser_matches_committed_pages` — exit 0; every other workspace unit,
  integration, real-Nix, tour, and doc test passed. The skipped assertion is the
  single failure above, not a broader test selection.
- `devenv shell -- cargo test -p cix --test tour -- --ignored generate_tour` —
  exit 0; `git diff --exit-code -- docs/tour` — exit 0; `devenv shell -- cargo
  test -p cix --test tour tour_matches_committed_document -- --exact` — exit 0.
- Final static audit: all 21 ledgers have valid header pairs, routed bullet
  endings, and existing local link targets; the living table has 21 rows;
  `git diff --check` exits 0; changed paths stay inside the track fence. No focused
  VM scenario applies because this track changes no runtime behavior.
