Generated: migrate.md@current · independently rechecked · 2026-08-05
Status: current

- The faithful 3.2.22 source build and `haproxy -v` probe pass, and its cold replay passes. The supplied context contains no `haproxy.cfg`, port, healthcheck, or readiness contract, so startup and stats behavior are not claimed. → evidence
- Docker's fixed uid/gid and `/var/lib/haproxy` workdir dissolve into DynamicUser. Error files are expanded into a declared state role at startup because this host could not use a writable `CONFIGDIR` mount. → case
- Formatting the locked Cixfile changes the FETCH builder identity: the formatted-copy cold build looks for snapshot `3378f6418827b7c769e19aefc1f52f90dce578bebce743ab98056cd4c5e2336d` instead of the existing pin and fails before replay. → language (fmt-key-neutrality)
- The dissolved twin deliberately follows nixpkgs' HAProxy version and feature set rather than the faithful 3.2.22 source build. → evidence
