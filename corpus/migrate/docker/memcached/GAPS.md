Generated: migrate.md@00078d9 · gpt-5.6-luna · 2026-08-04
Status: current

- Docker's fixed `memcache` uid/gid 11211 is replaced by Cix/systemd's managed unprivileged identity; the protocol probe does not depend on the numeric identity. → language (fixed service identity)
- The upstream `make test` harness expects a root account that the isolated Cix builder does not provide, so the faithful build skips that harness while keeping compilation, `memcached -V`, and the runtime protocol probe fatal. → evidence
- The dissolved twin incorrectly retains upstream version, URL, and checksum environment variables even though nixpkgs selects the packaged version and those values are behavior-free Docker build metadata. → prompt
