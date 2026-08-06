# directus migration receipt

2026-08-05, generated `migrate.md@f474d3f` (gpt-5.6-luna). Docker mode was not rerun.

`./check.sh cix` reached the glibc-backed native sqlite build, clearing the former Sass loader and traced-ENOTDIR walls, then exited non-zero at the offline install:

```text
ERR_PNPM_OUTDATED_LOCKFILE: pnpm-lock.yaml is not up to date with <ROOT>/package.json
```

The report names 18 removed development specifiers. No item or runtime probe is claimed.

## 2026-08-05 CIP-107 FetchPin regeneration attempt

After restoring the pinned ignored source context, the targeted foreground
`cix build corpus/migrate/docker/directus --update-lock build` update probe
reported volatile `node_modules/.modules.yaml` bytes (9563 B on both probes).
The build then exited 1: the offline `pnpm deploy` could not resolve
`tsdown@0.15.11` because its package metadata was absent from the cache. No
lock change is retained, and the legacy whole-tree FetchPin reader remains
until this evidence can be regenerated.

## 2026-08-06 CIP-107 retry

After `corpus/migrate/fetch.sh directus` restored the pinned context, the
synchronous `target/debug/cix build corpus/migrate/docker/directus --update-lock
build` build completed FETCH and installation, then exited 1 during the offline
production deploy: `@directus/tsconfig@4.0.0` is absent from the pinned package
metadata cache. Its generated partial lock was reviewed and restored; the
tracked `Cixfile.lock` SHA-256 remains
`e40ee98df87de1bbf9a65b261c79f56987e0eb4b70ab1a3ece6a106906ea0d66`.

## 2026-08-06 pnpm-wall coherence check

The earlier upstream-incoherence diagnosis is disproved. The fetched
`package.json` and `pnpm-lock.yaml` hashes exactly match git revision
`b1d7a45a77661fd13928a53448c06649f36b56f5`. Under nixpkgs Node 22 and exact
pnpm 10.27.0, `pnpm install --lockfile-only --frozen-lockfile
--ignore-scripts` validated all 41 workspace projects and exited 0. A full
install against an empty store explicitly reported “Lockfile is up to date”
before exiting 1 at `ERR_PNPM_NO_OFFLINE_TARBALL`. Logs and value-checked
statuses are under `/var/tmp/cix-pnpmwall-directus-current.6xD66E`.

Nearby revision `d87981b99d2e7916905ac797fda79f33dc01190b` also passed the
same full-checkout validation, exit 0; its receipt is
`/var/tmp/cix-pnpmwall-directus-nearby.uONmCj`. A coherent revision therefore
exists (including the current pin), and source incoherence does not gate the
CIP-107 narHash regenerations. The separately observed offline-deploy metadata
wall remains; no Directus item or runtime probe is claimed here.

## 2026-08-06 frozenStore store seal

The translation now deliberately upgrades upstream pnpm 10.27.0 to pinned
nixpkgs pnpm 11.18.0, the first eligible major for pnpm's read-only-store
route. Under Node 22.23.2, `pnpm fetch --frozen-lockfile --ignore-scripts`
exited 0, produced 80,763 CAS files plus `v11/index.db`, and the complete
store was sealed at
`/nix/store/2wc8j4rhf7130m1w6vf99w6ws6m0bzj2-fetched-store`. Its pre-seal and
sealed NAR hashes both equal
`sha256-NpISeaQzBL0Mxkq2mFCBOSJmP/OO4C9+Z+J5QN+6EnA=`; `index.db` is
read-only in the sealed path. Foreground evidence is
`/var/tmp/cix-pnpm-frozenstore-directus.JAsoAo`.

An earlier attempted receipt is invalid: putting Node before pnpm in a Nix
shell let Corepack honor the project pin and run 10.27.0. The valid rerun
selected the pinned 11.18.0 executable explicitly and value-checked both
version lines before fetching. This receipt claims fetch plus whole-store
seal only, as required; it does not claim a Directus item or runtime probe.
