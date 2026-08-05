Generated: migrate.md@current · gpt-5.6-luna staging, independently rechecked · 2026-08-05
Status: current

- The app remains at upstream `/var/www/html`; nested durable database/upload roles, cache, runtime socket directory, and native HTTP readiness/liveness are declared. Warm and cold builds pass. → case
- Cron's privileged Alpine process and exact multi-process Docker topology remain outside this one-service translation; scheduled work belongs in separately declared APP/timer artifacts. → case
- The independently rerun runtime check cannot complete because its generated unit tries to execute the workspace-local cix binary from inside `ProtectHome` and receives `203/EXEC`; no HTTP success is claimed from that run. → evidence
