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
- 2026-07-29 02:06 UTC — Live pass 1 stopped before compose activation because the system manager
  retained an empty `cix-run.slice` from preceding tests; stopped/reset the scoped cix units in
  both managers. Pass 2 stopped during root tagging: sudo's secure PATH omits Nix, and cix-index's
  tag path-info helper does not have cix-run's Nix fallback. No stack units or links were created.
  The demo now gives root the explicit default Nix profile plus standard system paths. This is
  demo plumbing, not a library/API expansion. Next: retry from clean managers.
- 2026-07-29 02:11 UTC — First activation reached nginx on its inherited TCP fd, but db and
  backend failed at NAMESPACE with `File exists`: the service compiler's own RuntimeDirectory
  alias and compose's edge BindPaths both targeted the same `/run/<name>` path. Fixed generation
  by cloning each checked service and suppressing only run paths owned by one of its edge grants
  before calling cix-run; the original item spec still drives semantic validation, and unrelated
  run dirs remain managed normally. Updated the golden to assert the edge-owned service has no
  competing RuntimeDirectory. The demo trap removed all stack units, links, listener, and edge
  paths. Next: rebuild cix and retry live.
- 2026-07-29 02:16 UTC — Live pass after the namespace fix proved both edge projections mount,
  backend starts, and nginx reaches `/run/backend/backend.sock`. PostgreSQL initdb then failed
  loading `dict_snowball`: because its executable is symlinked into the assembled item, PostgreSQL
  derives `$libdir` from the item prefix. Moved the NSS helper/runtime file aside and linked the
  package's complete `lib` at the expected item path. The first nginx readiness request also
  reached backend before db was ready and caused `check=True` to terminate it; backend now returns
  503 and stays alive until PostgreSQL is queryable. Cleanup again removed all stack state. Next:
  rebuild the changed items and retry.
- 2026-07-29 02:19 UTC — PostgreSQL now initializes and listens only on the database edge; backend
  stays healthy through readiness retries. The remaining example error was explicit in the server
  log: psql defaulted its database name from `--username=cix`, but initdb creates `postgres`, not a
  `cix` database. Added `--dbname=postgres`. The timed-out pass was then downed by the trap with no
  stack residue. Next: rebuild backend and continue the full update/rollback flow.
- 2026-07-29 02:22 UTC — First full live demo is green. v1 returned
  `hello from backend v1 via compose: database-ok`; system properties confirmed nginx retained
  PrivateNetwork, AF_UNIX-only, SocketBindDeny=any and the named inherited fd. Grouped ps showed
  `stack/backend`. After retag, diff reported exactly the backend service unit and store path;
  up left web/db ActiveEnterTimestampMonotonic unchanged and restarted backend; v2 was served.
  Rollback reactivated the v1 generation and response, then down removed all stack units, socket,
  links, `/run` edge paths, and port 8080. Root ps warned that it cannot reach the invoking user's
  bus under sudo, as the pre-existing ps implementation does; the system composite view is correct.
  Next: focused fmt/clippy/tests, commit live fixes, then run every required gate twice.
- 2026-07-29 02:25 UTC — Focused compose/Cixfile fmt, tests, and denied-warning clippy pass. The
  combined cix test initially failed only because a concurrent listenfds loop populated the user
  manager between the tour's two live ps captures. Reset those failed transient units and the
  empty slices in both managers; the cix tour drift and determinism tests then pass. The tour
  itself leaves its known empty user cix-run slice, which is stopped during every final cleanup
  audit. No compose code failure was involved. Next: commit the empirically required generator and
  example fixes.
- 2026-07-29 02:31 UTC — Exact verification gate passed twice consecutively:
  `cargo fmt --all --check`, workspace all-target clippy with `-D warnings`, and
  `cargo test --workspace`. The complete sudo demo then passed twice consecutively, including both
  rollback cycles and cleanup audits. `nix build .#checks.x86_64-linux.vm-dogfood --no-link` is
  green with the updated cix package. Tightened the root-gated integration test: only non-root
  self-skips; root always invokes demo.sh, which resolves the normal target/debug cix fallback or
  an explicit CIX_BIN. Next: final D30 IN/OUT and residue audit, then final log/commit.
- 2026-07-29 02:36 UTC — Final D30 audit is complete. IN is fully represented: strict
  path-diagnostic compose/lock models and schema; local/qualified/store resolution with pin,
  track, and targeted update; item/service/env/listener/edge/collision validation; deterministic
  store generations and manifests; composite profile activation, restart-changed, disappearance
  cleanup, down, rollback, diff, slice-grouped ps; sysusers-backed Unix-edge capabilities; and the
  three-Cixfile config-as-code stack. An explicit source scan confirms no composite netns,
  replicas/scale, socket proxy, health wiring, LoadCredential secrets, resource limits, or
  reconciler daemon were added.
- 2026-07-29 02:36 UTC — Final boring choices/deviations: cix.lock is a strict top-level
  `{"services": ...}` map with no invented lock version; check/diff resolve in memory while only up
  persists the adjacent lock; wildcard listener addresses conservatively conflict across IP
  families; down removes only empty direct `/run/<name>` edge destinations and will not delete
  arbitrary consumer paths; ps preserves the old layout until a composite is present. Two minimal
  prerequisite extensions are isolated in history: cix-index's non-tagging resolver and Cixfile's
  stream-only LISTENER directive. Generated edge groups intentionally persist as sysusers metadata,
  while authority exists only on running member units.
- 2026-07-29 02:36 UTC — Compose-v1 design walls: an edge-owned run path must replace, not overlay,
  the service compiler's RuntimeDirectory alias; compose must own direct runtime bind-destination
  cleanup; sudo callers need Nix available for root-side tag resolution; root ps cannot inspect an
  unrelated user bus without its runtime environment. Example-only walls were PostgreSQL's
  symlink-relative `$libdir`, explicit database selection, and readiness errors needing 503 rather
  than process exit. Final cleanup found both managers unloaded and no system links, stack paths,
  or port 8080 listener. It also removed only the many unloaded `cix-run-listenfds-*` files left in
  the user runtime directory by earlier probes, then daemon-reloaded it; those files were not
  compose artifacts and are not recoverable or needed.

- 2026-07-30 19:55 UTC — Diagnosed `scenario-update-repin` on `track/repinfix`.
  Exact failing repro: `nix build .#checks.x86_64-linux.scenario-update-repin
  --no-link -L`; after the second `cix up`, the VM entered the 900-second v2 curl
  retry with no API restart. Temporary VM diagnostics proved both profile resolutions
  named the same generation, both manifests retained the v1 API store path, both API
  units were byte-identical and carried the v1 bind source/environment, the unit link
  still targeted that generation, and the journal contained only the initial v1 start.
  This is correct product behavior: compose declarations default to `update: pin`, the
  adjacent lock therefore replays v1, and naming a mutable index tag `:track` does not
  opt into tracking. D44 requires an explicit root-side `update: track`; D47 did not
  change compose resolution or restart comparison. Fixed the scenario fixture to emit
  that policy only for update-repin and moved the generation/store-path assertions
  before the HTTP retry so a future lock-resolution regression fails immediately.
  Focused verification is green with the same exact command: the generation and API
  store path move to v2, systemd stops/starts only the API, v2 responds, the DB
  timestamp stays fixed, and rollback restarts v1. Next: commit the scenario correction,
  then run the full requested Rust/tour and lifecycle gates.
- 2026-07-30 20:07 UTC — Committed the correction as `e0bee7d`. The complete
  requested gate is green: `cargo fmt --all --check`;
  `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo test --workspace`;
  `cargo test -p cix --test tour -- --ignored generate_tour` followed by
  `git diff --exit-code -- docs/tour`;
  `nix build .#checks.x86_64-linux.scenario-update-repin --no-link -L`; and
  `nix build .#checks.x86_64-linux.scenario-lifecycle --no-link -L`. The committed
  update-repin check observed the API stop/start, v2 response, unchanged DB activation
  timestamp, v1 rollback, and teardown. The lifecycle guard independently observed
  selective API restart, unchanged DB activation, rollback, and complete unit removal.
  Its existing DB process required systemd's bounded stop timeout and SIGKILL during
  `down`, after which the scenario completed successfully.
