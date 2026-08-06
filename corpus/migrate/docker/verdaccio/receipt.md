# verdaccio migration receipt

2026-08-05, generated `migrate.md@f474d3f` (gpt-5.6-luna). Docker mode was not rerun.

Independent ordinary builds installed the 38-workspace pnpm graph but produced no item. The cold replay exited non-zero before build/deploy:

```text
recorded read set differs between warm and cold at "."
```

The two directory hashes differed at the `FETCH pnpm install` step. This is cold volatility, not a repinning opportunity; `/-/ping` was not run.

## 2026-08-06 bare-store spike and payoff attempt

Two independent exact-pnpm-11.1.2 FETCHes exited 0 and produced identical
`files/` trees: 56,280 files with NAR hash
`sha256-9yhmtdGwQCapcFYJkazRhC+l1XSwkTL4eaFuMg6hU/s=`. Their `index.db` files
differed and contained volatile `checkedAt` data. Copying only `files/` into
two fresh stores and running `pnpm install --offline --frozen-lockfile
--ignore-scripts` made both commands exit 1 with
`ERR_PNPM_NO_OFFLINE_TARBALL`; concurrent lookup selected different first
missing packages, so only the error class is stable. Foreground evidence is
under `/var/tmp/cix-pnpmwall-store-verdaccio.pFFbxD`.

A two-builder Cixfile payoff attempt successfully sealed the CAS-only FETCH
view (`/nix/store/swbd0jgxv3zr1skfjl4gl41a9kwzapq3-cix-build-view`) and copied
it into an offline builder. After explicitly bypassing pnpm 11.17's project
version switch, the full traced monorepo install remained inside the RUN until
the synchronous 360-second timeout (exit 124); see
`/var/tmp/cix-pnpmwall-verdaccio-payoff.OGv38g/build-sixth.log` and
`sixth-exit-status`. No item or runtime probe is claimed. The precise missing
mechanism is a stable or offline-reconstructable package-to-integrity index;
pnpm's CAS alone is insufficient.
