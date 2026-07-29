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
- 2026-07-29 01:22 UTC — Generation goldens (service, edge owner, listener socket, target), byte
  determinism, and identical-input Nix store-path determinism are green. Implemented the root
  runtime: per-composite nix-env profiles, sysusers application, guarded atomic links into
  /etc/systemd/system, daemon reload, target start, changed-unit restarts, disappeared-unit
  stop/unlink, down with profile retention, and profile rollback followed by reactivation. Unit
  links are only replaced/removed when they point into the expected prior generation, preventing
  accidental overwrite of administrator units. Diff dry-builds and reports sorted unit
  add/remove/change plus service storePath old→new. Next: compile and test the runtime/CLI, wire the
  root cix binary and composite-aware ps, then exercise activation against the real example.
- 2026-07-29 01:28 UTC — Runtime and CLI unit tests pass. Wired `cix compose check/diff`,
  `cix up/down/rollback`, and replaced the root `cix ps` dispatch with a compatible all-cix unit
  view that adds COMPOSITE and SERVICE columns based on each service's slice. Edge owners now also
  live in the composite slice, making the grouping and resource boundary complete. Existing
  one-shot `cix run` units remain visible with no composite. Next: verify workspace CLI/tests,
  commit this increment, then build the three-item real stack and root-gated demo.
- 2026-07-29 01:31 UTC — The first cix tour run exposed two environmental/compatibility facts:
  stale failed cix-run listener probes remained in the user manager, and unconditional ps columns
  would drift the established non-compose tour. Reset only the stale `cix-*` failures and stopped
  the empty cix-run slice, leaving both managers clean. `cix ps` now preserves its exact legacy
  view when no composite is loaded and switches to grouped COMPOSITE/SERVICE columns when one is
  present. Targeted cix-compose/cix tests, tour drift/determinism, fmt, and denied-warning clippy
  now pass. Next: commit activation/CLI, then implement and empirically drive the example stack.
- 2026-07-29 01:39 UTC — Example implementation hit a real API wall: Cixfile cannot express the
  spec-v3 listener already required by D30, so an nginx item cannot simultaneously have its own
  Cixfile and accept only inherited fds. Added the minimal `LISTENER <name>` directive in
  cix-cixfile: it emits only the existing stream-listener object, conflicts with same-named PORT,
  and selects cixSpec 3 when present. Existing Cixfiles remain byte-for-byte spec v2. This is a
  prerequisite extension rather than new compose scope. Next: verify and commit it separately,
  then finish the stack items.
- 2026-07-29 01:52 UTC — Added the compose stack sources: PostgreSQL is Unix-only with the proven
  DynamicUser NSS wrapper and `/run/postgresql`; the Python backend queries it through that edge
  and serves HTTP on `/run/backend`; nginx maps systemd fd 3 into its inherited-socket mechanism,
  retains AF_UNIX-only/private-network service policy, and proxies to the backend edge. Each item
  has its own Cixfile. Added hand-written compose.json plus a 37-line stdlib-only generator and a
  byte-equivalence test, along with the root-gated integration-test entry point. Backend content
  and a compose env override are deliberately separate so the demo can prove both item updates and
  env overlay. Next: implement demo.sh, build all three items, and begin the live activation loop.
- 2026-07-29 02:02 UTC — All three Cixfiles build and emit valid v2/v3 specs. Added the sudo demo:
  isolated root tag state, config-generator byte check, v1 activation and fd-only web assertions,
  grouped ps, v2 retag/diff, unchanged web/db timestamps, backend-only restart, rollback to v1,
  down, listener/path/link audits, and both-manager cleanliness. Down now also removes empty direct
  `/run/<name>` edge mount destinations (the dstyle proposal's post-gate amendment); arbitrary
  consumer destinations are deliberately not removed. Next: run the live demo, diagnose each
  systemd/application wall, and keep cleanup asserted after every attempt.
