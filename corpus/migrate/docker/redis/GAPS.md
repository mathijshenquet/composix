Generated: migrate.md@00078d9 · gpt-5.6-luna · 2026-08-04
Status: current

- Docker's fixed redis uid/gid, `gosu` verification, root-time `chown`, and privilege transition dissolve into Cix/systemd's managed service identity. The faithful twin retains argument normalization and umask behavior, but its copied entrypoint's root-only branch is not exercised. → language (fixed service identity)
- The PING receipt proves the faithful service responds, not that its retained protected-mode patch, TLS build, jemalloc tuning, or every copied command behaves identically to the Docker image. → evidence
- The dissolved twin deliberately follows nixpkgs' Redis version and defaults rather than the upstream 7.4.8 source build; it preserves only the state, port, startup, and probe-served behavioral contract. → evidence
- The 2026-08-06 cold-replay sweep could not materialize the faithful Redis FETCH snapshot: the declared `REDIS_DOWNLOAD_SHA` for `http://download.redis.io/releases/redis-7.4.8.tar.gz` mismatched the fetched 3.37 MiB response (`sha256-PsELmdlX+Gux9kXvgjb2FuGNuTKFgZ1xxZPEyiGhroo=` declared, `sha256-LijBYlrDlf7uKh6a7XacdBt0oLJbRXtp8BkqgJ126Nc=` observed). → upstream drift wall
