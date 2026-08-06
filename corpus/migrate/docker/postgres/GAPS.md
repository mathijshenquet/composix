Generated: migrate.md@current · track-expand-postgres-registry · 2026-08-06
Status: current

- The upstream Dockerfile installs PostgreSQL from the PGDG APT repository after retrieving a signing key from a keyserver. The faithful case cannot perform that install-time transaction under the no-network RUN contract, so it uses the locked nixpkgs `postgresql_17` package while retaining the upstream initdb entrypoint and `/var/lib/postgresql/data` lifecycle. → language (D47)
- The upstream `gosu` release binary is an explicit checksum-verified FETCH, but its detached-signature/keyserver verification is not reproduced; the FETCH content pin is the stable trust boundary. → evidence
- Docker's Debian base, fixed uid/gid, package-managed locale, and SIGINT image stop signal dissolve into the locked runtime and systemd service lifecycle. → case
- The arbitrary-path state-role realization can make the managed data bind read-only after initialization; this is the known state-role realization defect under repair on `track/staterole-bindfix`, rather than a new PostgreSQL diagnosis. → language (state-role-bindfix)
