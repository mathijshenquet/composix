
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
