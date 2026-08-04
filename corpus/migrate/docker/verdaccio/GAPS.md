Generated: migrate.md@e1978b6 · gpt-5.6-luna · 2026-08-04
Status: current

- The pnpm monorepo wall stands: the sandboxed `pnpm --filter verdaccio build` path exits non-zero before producing an item, so the runtime graph remains unproved. → evidence
- The upstream UI CSS generator performs network access during `pnpm run build`; keeping that entire build in `FETCH` is an over-broad networked transformation whose reproducibility has not been established. → case
- Docker's fixed `USER 10001`, root group, uid-entrypoint passwd injection, and ownership/mode ceremony dissolve into systemd-managed identity rather than preserving numeric ownership. → case
- The external `/-/ping` probe is not represented as native readiness and cannot run until the build wall is cleared. → case
- Cold replay diverges at Corepack's pnpm cache (`warm Directory`, `cold Absent`) before reaching the already-red monorepo build. → case (cold stability)
