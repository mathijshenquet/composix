Generated: migrate.md@f474d3f · gpt-5.6-luna · 2026-08-05
Status: current

- The pnpm 11.1.2 `files/` CAS is deterministic across independent FETCHes, but bare-CAS offline installs fail with `ERR_PNPM_NO_OFFLINE_TARBALL`: pnpm consumes a volatile SQLite package index and cannot regenerate it from CAS bytes. Existing FETCH/COPY/RUN cannot form an honest cold-replayable artifact; no hash is invented. → case
- The ordinary monorepo build produced no item in this assembly; its runtime and `/-/ping` remain unproved. Fixed UID/group and init-wrapper behavior dissolve into systemd identity/supervision. → case
