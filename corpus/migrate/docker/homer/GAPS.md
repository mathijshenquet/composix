Generated: migrate.md@current · independently rechecked · 2026-08-05
Status: current

- The staged registry/CA symptom was not an irreducible pnpm wall: importing CA certificates, pinning pnpm's data tree inside FETCH, and installing offline lets the Vite build and HTTP check pass. → evidence
- Docker's fixed lighttpd uid/gid, Alpine package layout, labels, and build-platform selector dissolve into the systemd identity and locked nixpkgs closure. → case
- Native health checks use the declared default port 8080; changing Docker's interpolated `PORT` at runtime is not equivalent. → language
