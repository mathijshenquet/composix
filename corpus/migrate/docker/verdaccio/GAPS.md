Generated: migrate.md@f474d3f · gpt-5.6-luna · 2026-08-05
Status: current

- The translation upgrades pnpm 11.1.2 to pinned nixpkgs pnpm 11.18.0: pnpm upgraded past upstream's `packageManager` pin for frozenStore read-only-store support; pre-11.7 pnpm structurally cannot install from a pinned store. The whole fetched store (`files/` plus `index.db`) is retained; no store bytes are stripped, normalized, or regenerated. → case
- The full frozenStore route reaches the build, but legacy production deploy still requires package-mirror metadata that `pnpm fetch` does not put in the sealed store (`ERR_PNPM_NO_OFFLINE_META` for `@verdaccio/e2e-cli@2.10.1`). → case
- The ordinary monorepo build produced no item in this assembly; its runtime and `/-/ping` remain unproved. Fixed UID/group and init-wrapper behavior dissolve into systemd identity/supervision. → case
