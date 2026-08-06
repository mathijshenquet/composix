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

## 2026-08-06 frozenStore route

A Verdaccio-shaped validation upgraded the package manager to pnpm 11.17.0
under Node 24.16.0, fetched the complete store, and sealed all 56,280 CAS files
plus `v11/index.db` at
`/nix/store/4fphf7lk69cdifd47syfm7ndx8ibf9mk-fetched-store` (NAR hash
`sha256-cKx9qd/dS59GaT6iqf8BaFhWAoXHCKwi9JV2ARgwIQo=`). Two fresh same-path
`install --offline --frozen-lockfile --frozen-store --ignore-scripts` commands
both exited 0. Separate strace captures contained no AF_INET/AF_INET6
operation, the sealed store hash remained unchanged, and both ensuing builds
produced byte-identical NAR hashes for all 31 package `build/` directories.
Foreground evidence is `/var/tmp/cix-pnpm-frozenstore.WWK5Uz`.

The raw `node_modules` trees are not byte-identical: pnpm writes a fresh
`.modules.yaml` `prunedAt` and `.pnpm-workspace-state-v1.json`
`lastValidatedTimestamp`. Those bytes were neither removed nor normalized;
the application build outputs are the identity receipt. A legacy production
deploy using the sealed store still exits 1 at `ERR_PNPM_NO_OFFLINE_META` for
`@verdaccio/e2e-cli@2.10.1`.

The translated cix route uses pinned nixpkgs pnpm 11.18.0 and the earlier
builder's immutable whole-store path. Its double-fetch probe completed in
66,874 ms and sealed
`/nix/store/3cplwgxzn3pxf7m660478z2d5hh8hqxg-cix-build-consumed`, then the
traced offline install remained active until the synchronous 1,200-second
bound exited 124. This is a cix trace-cost wall; no item or runtime probe is
claimed. Exact evidence is
`/var/tmp/cix-pnpm-frozenstore-verdaccio-route.HuCHfk`.
