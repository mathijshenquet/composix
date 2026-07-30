
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
