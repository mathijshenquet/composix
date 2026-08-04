Generated: migrate.md@d582f41 · unknown · 2026-07-31
Status: stale — regenerate with CIP-79

- Docker's HTTP `HEALTHCHECK` is only mirrored by `check.sh`; regenerate with a native HTTP `READINESS` now that CIP-79 is built. → case
- The Docker build supplies `NODE_ENV=production`, but the Cix builder drops that input without saying whether it is translated or unnecessary. → case
- Static output moved from `/usr/share/nginx/html` to `/srv/www` with a replacement nginx configuration and no stated parity reason. → prompt
- The orange grade is an evidence limit, not a runtime failure: the historical HTTP probe passed, but its consumed build tree has not been reproduced for the closed-root audit. → evidence
