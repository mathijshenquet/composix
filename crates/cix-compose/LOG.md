# compose track log

- 2026-07-29 00:38 UTC — Started `.dev/specs/track-compose.md`. Read D30/D28/D9, the
  live-proven dstyle Unix-edge proposal #1, and the cix-run spec-v3 generator API. Confirmed a
  clean `track/compose` worktree and that the new crate does not exist. The shell is not directly
  direnv-activated, so all project commands will run through `direnv exec .`. Scope is held to
  D30 IN: strict compose JSON/check, lock resolution, deterministic generations, per-composite
  profiles and activation/rollback/down, env/listeners/Unix edges, pin/track, diff, and grouped
  ps. D30 OUT remains unimplemented. First increment: wire the new crate, then add the one missing
  boring cix-index library resolver before building the data-model and lock lifecycle.
- 2026-07-29 00:42 UTC — Added the narrowly-scoped cix-index resolver needed by compose: one
  library call resolves a store path, bare local tag, or qualified HTTP index ref for the current
  system and returns storePath/narHash. Remote resolution reuses index content negotiation and
  substituter verification without creating a mirror tag; `pull` now shares the same fetch helper.
  This is the only currently identified cross-territory extension. Next: strict compose/lock
  models, JSON-path diagnostics, and semantic validation with injected resolution tests.
- 2026-07-29 00:54 UTC — Implemented the compose-v1 contract and published JSON Schema in the
  crate. Both compose.json and cix.lock reject unknown fields with serde JSON paths. Resolution
  now covers pin reuse, track refresh, targeted/all update, removed services, item spec loading,
  optional multi-service selection, D21 env validation, required env, listener binds, edge
  references/producer run paths, duplicate edge projections, host-port collisions, bind
  collisions (including wildcard addresses), and port-versus-listener collisions. Tests inject a
  resolver, so the full semantic matrix and both collision orders run rootlessly. Deliberate
  boring choice: a bind with an unspecified address conflicts with either IP family on the same
  port; false negatives would otherwise defer failure to activation. Next: compile checked input
  into deterministic service/edge/socket/target generations and golden fixtures.
- 2026-07-29 01:08 UTC — Added deterministic generation rendering around cix-run's public
  compiler. Services receive composite naming/slice, target ownership, explicit socket
  dependencies, and per-edge SupplementaryGroups/UMask/BindPaths. Generated edge owners hold a
  root:edge-group 2770 RuntimeDirectory for target lifetime; the stable per-edge group comes from
  an in-tree FNV-1a hash to avoid another dependency. Listener sockets carry explicit Service and
  FileDescriptorName wiring. The generation contains units, sysusers.d, source compose.json,
  canonical cix.lock, and manifest.json; `nix store add-path` gives the immutable generation.
  Next: fill and verify unit goldens, then implement profile activation/diff/down/rollback.
