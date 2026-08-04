Generated: migrate.md@dd2f39a · terra · 2026-07-30
Status: stale — regenerate with CIP-82

- The sed rewrite from `/verdaccio/storage` to `/var/lib/verdaccio` is unnecessary: `STATEDIR /verdaccio/storage` can mirror the upstream volume directly under the migration guide's role-path rule. → case
- The deploy tree moves from `/opt/verdaccio` to `/app`, and configuration moves from `/verdaccio/conf` to `/etc/verdaccio`, without a stated reason. → prompt
- Upstream `VERDACCIO_APPDIR`, user/uid, port/protocol/address, `PATH`, and `HOME` environment declarations disappear without translated/dissolved/gap dispositions. → case
- Node is linked into `/bin` and then found through implicit self-import instead of an explicit artifact tool declaration. → language ([artifact-import draft](../../../cips/draft/artifact-import.md))
- “Package-manager build remains non-green” means the Corepack/pnpm sequence exits non-zero before producing any item; record the precise failing command and diagnostic on a fresh attempt. → evidence
- Until that build receipt exists, the service block is untested aspiration rather than a runnable approximation. → browser
