# track/dirs — CIP-82 leg 1: overlay backing + arbitrary paths + DIR

Read AGENTS.md first. Authoritative: docs/cips/0082-dirs.md (§3 + §5
Decision). SCOPE = leg 1 only: manifest/parser + unit generation.
Compose materialization (`host:`/`shared:`/`as:`), `cix clean`/purge
verbs, and `.env` are leg 2+ — do NOT build them. Work in
`.worktrees/dirs` on branch `track/dirs`. Keep `crates/cix-run/LOG.md`
current (create if absent, tracked).

1. **Arbitrary in-namespace paths for role dirs**: `STATEDIR`,
   `CACHEDIR`, `LOGDIR`, `RUNDIR` (Cixfile) and `dirs.{state,cache,
   logs,run}` (manifest) accept any absolute path (`LOGDIR /app/logs`);
   the D11 conventional-root validation is removed. Paths must be
   absolute, no `..`, no duplicates across all roles of one service
   (check error).
2. **Overlay backing in unit generation** (crates/cix-run/src/unit.rs
   `add_directories`): replace the alias branch and `state-N` index
   names with the full-mirror subpath form —
   `LogsDirectory=<unit-base>/<full declared path>` (systemd creates
   parents) + `BindPaths=` (`BindReadOnlyPaths=` never applies in leg 1)
   from the host location to the declared in-namespace path. Keep the
   `TemporaryFileSystem=<class-root>:ro` masking so undeclared class
   paths stay invisible. Multiple dirs per role = multiple subpath
   entries, no index names. User-manager mode keeps the equivalent
   `%S`-rooted bind fallback.
3. **Env override**: set `$STATE_DIRECTORY`/`$CACHE_DIRECTORY`/
   `$LOGS_DIRECTORY`/`$RUNTIME_DIRECTORY` explicitly to the declared
   in-namespace paths (colon-joined per systemd convention), overriding
   systemd's host-side values.
4. **`DIR` directive** (undecorated): `DIR /media:ro` and
   `DIR /consume:rw`; a bare path means rw. Manifest field
   `dirs.data: [{path, ro}]`. Leg-1 semantics: parse,
   validate (absolute, no dups), store in manifest, and have `cix run`/
   generation FAIL with the CIP's teaching error when a DIR has no
   materialization — since leg 1 has no compose materialization, any
   DIR declaration errors at unit-generation time with: "DIR declares
   operator-supplied data; materialization arrives with compose
   (docs anchor); for a cix-managed dir pick a role:
   STATEDIR/CACHEDIR/LOGDIR/RUNDIR". Parser suggestion fixtures: the
   naive `DIR` misreading is exercised in a torture fixture asserting
   that exact guidance.
5. **Migration**: existing declarations (e.g. postgres
   `STATEDIR /var/lib/postgresql`) stay legal — in-namespace paths are
   unchanged; only host-side layout moves to the full mirror
   (`/var/lib/<unit>/var/lib/postgresql`). Update unit-generation
   snapshot tests accordingly. VM checks must stay green WITHOUT
   loosening any assertion about in-namespace behavior (postgres/redis
   state survives restart, side-by-side isolation).
6. Docs: docs/cixfile.md dirs section rewritten per CIP-82 (roles =
   claims cix satisfies; DIR = operator-supplied; lifecycle table);
   docs/docker.md `VOLUME` row splits STATEDIR/DIR; tour regen.
7. Tests: unit-gen snapshots (subpath + bind + env override; multi-dir
   per role; arbitrary path), parser (DIR forms, dup/relative errors,
   teaching error), VM: one service declaring `LOGDIR /app/logs` writes
   a file, restarts, reads it back; host path
   `/var/log/<unit>/app/logs` contains it.

Gate: fmt / `cix fmt --check examples` / warning-denied clippy /
workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. Exact repros in the LOG. Commit
on this branch when green.
