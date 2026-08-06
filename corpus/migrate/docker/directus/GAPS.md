Generated: migrate.md@f474d3f · gpt-5.6-luna · 2026-08-05
Status: stale — regenerate with CIP-107 FetchPin migration

- Imported glibc and traced ENOTDIR both clear their former walls, but the pinned root `pnpm-lock.yaml` is stale for the copied `package.json`: offline frozen install refuses its 18 missing development specifiers before an item exists. → case
- Fixed Docker numeric ownership and PM2 supervision dissolve into cix/systemd identity and service supervision. → case
- The 2026-08-06 `--update-lock build` retry completed FETCH and installation but the offline production deploy cannot resolve `@directus/tsconfig@4.0.0` because its package metadata is absent from the pinned cache; its whole-tree FETCH pin therefore cannot yet be regenerated. → case
