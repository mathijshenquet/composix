# verdaccio migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`IMPORT`, builder `ENV`, `STATEDIR`, explicit Node `bin/`, and `CLAIM jit`/`egress`).

Docker side: historical 2026-07-30 receipt, not rerun; no historical Docker digest was captured.

## `./check.sh cix`

```text
BUILDER build step 3 FETCH executed
Adding pnpm@11.1.2 to the cache...
```

Exit status: non-zero during the Corepack/pnpm build sequence; no item was produced.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Worker evidence: no warm item was produced. The staged pnpm graph reached the
project build but failed during output handling (`Not a directory`/later disk
exhaustion), so its supplied probe remained unrun.

After `bash corpus/migrate/fetch.sh verdaccio` exited 0, the assembler's
`target/debug/cix build corpus/migrate/verdaccio` installed the 38-project
workspace, ran the package builds and deploy path, then exited 1 after
`pnpm --filter "./packages/**" build` with bare `Error: Not a directory`.
The supplied probe's ordinary rebuild exited 1 too, additionally reporting
`ERR_PNPM_DEPLOY_DIR_NOT_EMPTY` for the populated warm output path. No item was
produced. `target/debug/cix build --cold corpus/migrate/verdaccio` exited 1
earlier: `FETCH corepack install` observed `.cache/node/corepack/v1/pnpm` as a
warm directory and cold-absent. Docker mode was not rerun.
