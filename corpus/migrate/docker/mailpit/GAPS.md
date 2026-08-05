Generated: migrate.md@current · independently rechecked · 2026-08-05
Status: current

- The source build and degraded user-manager `/livez` check pass. The supported system-manager run starts Mailpit but native readiness fails when `ProtectHome` prevents the workspace-local cix probe executor from running. → language
- Docker's Alpine layout, fixed uid/gid, and image labels dissolve into the locked nixpkgs closure and managed identity. → case
