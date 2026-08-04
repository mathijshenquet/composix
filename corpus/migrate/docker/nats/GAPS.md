Generated: migrate.md@00078d9 · gpt-5.6-luna · 2026-08-04
Status: current

- The `SOURCE` record cannot reconstruct the upstream context, so `nats-server.conf` and `docker-entrypoint.sh` remain unreadable. The conversion starts with `--http_port 8222` to make the supplied health probe observable, but this does not preserve the missing config-file or entrypoint contract. → evidence + case
- The faithful source twin packages only the upstream amd64 release even though the Dockerfile dispatches across eight architectures. → case
- The monitoring `/healthz` receipt does not exercise a client publish/subscribe round trip; the dissolved twin also follows nixpkgs' version rather than upstream 2.12.14 by design. → evidence
