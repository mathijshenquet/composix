
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
