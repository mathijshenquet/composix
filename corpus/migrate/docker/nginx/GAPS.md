Generated: migrate.md@00078d9 · gpt-5.6-luna · 2026-08-04
Status: stale — regenerate with STOPSIGNAL

- The faithful twin preserves the upstream entrypoint scripts, but its Alpine-specific IPv6 hook cannot validate `apk manifest nginx` against the nixpkgs package and therefore does not perform that Docker mutation. → case
- Docker's stdout/stderr log symlinks are not reproduced, and the faithful runtime probe is red because the packaged configuration opens `/var/log/nginx/error.log` and `access.log` while the service declares no `LOGDIR`. systemd captures process output, but that does not satisfy those file paths. → case
- Docker's fixed nginx uid/gid 101 is replaced by Cix/systemd's managed service identity. → language (fixed service identity)
- Upstream requests graceful `STOPSIGNAL SIGQUIT`; Cix now has the directive, so regenerate this case with that declaration. → stale ([STOPSIGNAL disposition](../../../../cips/dispositions.md#batch-2026-08-04-blessed-by-mathijs-dockermd-application-queued))
- The copied worker-autotune hook's Docker cgroup paths and operator-supplied envsubst/stream templates are not covered by the HTTP probe. → evidence
