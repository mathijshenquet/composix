# Track: pathdecl — the PATH directive (D31)

Read `docs/design.md` D31 — it is the contract. Where ambiguous, choose boring and note it
in LOG.

Environment: run gates via `devenv shell -- …` or PATH cargo; never wait for confirmation.
Territory: `crates/cix-cixfile/`, `examples/{nginx,postgres,redis,caddy,node-app,listenfds}/`,
`docs/cixfile.md`. Do NOT touch `crates/cix-run` (no schema change exists — PATH is an
ordinary env default), `docs/docker.md` or `docs/tour/` (another track), `examples/compose/`
`examples/dstyle/` `examples/buildshape/`. COMMIT AS YOU GO. Work log:
`crates/cix-cixfile/LOG.md`.

## Deliverables

1. **Parser**: `PATH <dir>…` item-level directive, repeatable; dirs are `${pkg}`-interpolated
   absolute paths; order across occurrences = search order; duplicates error.
2. **Build-time resolution**: in `EXEC`/`SETUP`, a bare argv[0] (no `/`) resolves against the
   declared PATH dirs at compile time — the compiler verifies the executable exists in the
   built package output and writes the REAL absolute store path into the generated spec.
   Unresolvable bare name → line-numbered error listing the searched dirs. No declared PATH +
   bare argv[0] → error suggesting PATH or an absolute `${pkg}` path.
3. **Runtime env**: when PATH is declared, emit `env.PATH` default = the joined dirs into the
   generated spec (ordinary env var; operator-overridable like any env). Error if the Cixfile
   also declares ENV PATH explicitly (one source of truth).
4. **Examples flipped to the idiom**:
   - node-app: `EXEC node /app/server.js` via `PATH ${nodejs}/bin`.
   - postgres: `PATH ${postgresql}/bin ${coreutils}/bin`; scripts call bare `initdb`,
     `postgres`, `id`, `mkdir`, `mv`, `rm`; the `$(dirname "$0")` gymnastics and bin-LINKs
     disappear; verify whether the `share/postgresql` LINK is still needed now that the real
     binary at its real prefix is invoked (expect not — confirm empirically, record).
   - nginx/redis/caddy/listenfds: adopt PATH where it simplifies; direct `${pkg}/bin/x` EXEC
     stays where it's the trivial single line (judgment; note choices).
   All touched examples: rebuild via `cix build` AND `nix-build` where a default.nix exists
   (default.nix variants may keep their current shape — they're the escape hatch, only update
   if their generated spec must stay semantically comparable), sudo demos green.
5. **Docs**: `docs/cixfile.md` — PATH row in the directives table, the LINK-shift prose
   updated (LINK = assets), the worked nginx example updated ONLY if the example itself
   changed, and a short "Scripts and tools" note replacing the dirname-$0 pattern text.
6. **Tests**: parser (order, duplicates, ENV PATH conflict), resolution (found/ambiguous
   across dirs = first wins/not found), golden spec fixture showing resolved argv + PATH env.

## Done gate

fmt/clippy/`cargo test --workspace` green ×2; touched demos green ×2 under sudo; VM check
green; no leftover units; committed; clean status; LOG summary incl. the share/postgresql
finding.
