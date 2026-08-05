Generated: migrate.md@f474d3f · gpt-5.6-luna · 2026-08-05
Status: current

- Imported glibc and traced ENOTDIR both clear their former walls, but the pinned root `pnpm-lock.yaml` is stale for the copied `package.json`: offline frozen install refuses its 18 missing development specifiers before an item exists. → case
- Fixed Docker numeric ownership and PM2 supervision dissolve into cix/systemd identity and service supervision. → case
