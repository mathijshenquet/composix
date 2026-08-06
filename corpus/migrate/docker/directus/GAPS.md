Generated: migrate.md@f474d3f · gpt-5.6-luna · 2026-08-05
Status: current

- Imported glibc and traced ENOTDIR both clear their former walls. Independent Node 22/pnpm 10.27.0 validation now proves the pinned root lock is coherent for all 41 workspaces; an empty-store install passes frozen validation and then fails only because package content is absent. → case
- The translation upgrades pnpm 10.27.0 to pinned nixpkgs pnpm 11.18.0: pnpm upgraded past upstream's `packageManager` pin for frozenStore read-only-store support; pre-11.7 pnpm structurally cannot install from a pinned store. The whole fetched store (`files/` plus `index.db`) is retained; no store bytes are stripped, normalized, or regenerated. → case
- Fixed Docker numeric ownership and PM2 supervision dissolve into cix/systemd identity and service supervision. → case
- The 2026-08-06 frozenStore probe fetched and sealed 80,763 CAS files plus `index.db` without transforming the store; its read-only NAR hash remained unchanged. The required fetch/seal tier is green, but no offline install, production deploy, item, or runtime is claimed from that probe. → evidence
