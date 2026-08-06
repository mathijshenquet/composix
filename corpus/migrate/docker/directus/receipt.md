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
