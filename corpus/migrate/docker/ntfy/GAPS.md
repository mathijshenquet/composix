Generated: migrate.md@current · track-expand-ntfy-filebrowser · 2026-08-06
Status: current

- The Docker build context is created by GoReleaser and supplies `ntfy` without recording an artifact identity in the Dockerfile. The faithful Cixfile makes that hidden release-tarball input explicit and pins both its upstream SHA-256 and FETCH snapshot; the v2.27.0 build and `/v1/health` receipt pass. → case
- Docker's Alpine base, fixed image layout, labels, and entrypoint-only command dissolve into the locked nixpkgs closure and the explicit `ntfy serve` service command. → case
- The host drops `PrivatePIDs` while realizing DynamicUser, so Cix reports degraded PID-namespace confinement for the successful system-manager health probe. → evidence
