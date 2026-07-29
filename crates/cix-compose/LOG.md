# compose track log

- 2026-07-29 00:38 UTC — Started `.dev/specs/track-compose.md`. Read D30/D28/D9, the
  live-proven dstyle Unix-edge proposal #1, and the cix-run spec-v3 generator API. Confirmed a
  clean `track/compose` worktree and that the new crate does not exist. The shell is not directly
  direnv-activated, so all project commands will run through `direnv exec .`. Scope is held to
  D30 IN: strict compose JSON/check, lock resolution, deterministic generations, per-composite
  profiles and activation/rollback/down, env/listeners/Unix edges, pin/track, diff, and grouped
  ps. D30 OUT remains unimplemented. First increment: wire the new crate, then add the one missing
  boring cix-index library resolver before building the data-model and lock lifecycle.
