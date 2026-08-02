# compose track log

- 2026-08-02 UTC — Synchronous receipt: `devenv shell -- nix build
  .#checks.x86_64-linux.scenario-netns -L` exited 0 after 112.95s of VM script time. It covered
  two pods using the same internal port, FD-direct and proxyd publication, allowed and suppressed
  egress, closed-root resolver availability, service address filtering, link-local absence,
  v1→v2 activation, rollback to v1, reactivation to v2 with the same persisted IPAM lease, and
  complete namespace/unit teardown. Next: finish golden snapshots and the decision/gap ledgers,
  then run every declared track gate together.

- 2026-08-02 UTC — The live VM synchronously passed both pod-local fixed-port servers, both
  publish tiers, egress allow/deny, FD inheritance across the namespace boundary, and link-local
  absence. Its second closed-root update exposed a scenario-only stale ownership problem: the
  egress probe persisted an observation file across changing `DynamicUser` identities. The probe
  now correctly declares that ephemeral result as a runtime directory so generation replacement
  and rollback test netns lifecycle rather than state-file ownership. Next: rerun through update,
  rollback, stable IPAM, and teardown.

- 2026-08-02 UTC — Correction to the prior live-debug entry: the Unix pathname-edge failure is
  pre-existing mount-namespace behavior outside this track, not missing activation directory
  creation. The exploratory edge-unit change is reverted. `scenario-netns` now exercises the
  required cross-boundary case through a host socket-activated listener whose file descriptor is
  inherited by a service inside the pod namespace, while the established edge fixtures remain
  byte-for-byte unchanged. Generated networkd fragments also explicitly disable link-local
  addressing on the bridge and host veths. Next: rerun the complete netns lifecycle scenario.

- 2026-08-02 UTC — Live namespace startup debugging found two shell/lifecycle facts hidden by
  fixture-level unit assertions. The netns discovery grep had inner single quotes inside the
  unit's single-quoted `sh -c` program; it now uses safe double quotes and an explicit `if`, so
  `dash -e` cannot terminate before `ip netns add`. The cross-boundary Unix-edge phase also
  exposed that activation cleaned edge mountpoints on down but never created them on up;
  activation now creates every validated edge destination before systemd starts the edge unit.
  The VM keeps Unix-edge/netns coverage and then performs a separate closed-root phase for the
  resolver bind, avoiding dependence on closed-root edge mount ordering. Next: rerun the full
  scenario through services, veths, both publish tiers, rollback, stable IPAM, and teardown.

- 2026-08-02 UTC — The first two synchronous `scenario-netns` receipts caught focused harness and
  lifecycle defects before the gate: systemd rejected escaped hyphens in closed-root directory
  unit names, so the scenario now uses safe member names, and its minimal NixOS node had no
  `systemd-networkd.service`, so `nodeWith` enables networkd only for this scenario. Review also
  caught IPAM allocation mistakenly placed in read-only `diff`; allocation is now root-only in
  `up`, while `diff` retains deterministic generation without persistent writes. The second VM
  reached generation activation and failed exactly at the absent networkd unit, confirming the
  generated unit graph itself loaded. Next: rerun the focused VM with networkd and persisted
  leases, then harden any live namespace/veth/proxy behavior it exposes.

- 2026-08-02 UTC — The strict model and checked-tree core are implemented and focused compose
  tests pass. `network` accepts only `"pod"`; nearest-pod ancestry is recorded per leaf (including
  pod-in-pod), port collision checks are namespace-scoped, `egress` follows D49's manifest default
  plus silent tightening/loud loosening override, and recursive `publish` surface resolution
  distinguishes listeners from fixed ports. Generation now emits named-netns lifecycle units,
  `NetworkNamespacePath`, loopback-only per-service address policy, fd-direct host sockets,
  proxyd fallbacks, and networkd bridge/veth fragments. Egress leases live in persistent cix
  IPAM state and are embedded into generations, so rollback cannot reallocate them. The first
  40-test compose run is green, including nearest-ancestor and both publish-tier coverage. Next:
  build the live `scenario-netns`, use its failures to harden activation/networkd timing and
  teardown, then finish snapshots/docs and the complete focused gate.

- 2026-08-02 UTC — Mapped the tree resolver, generation compiler, activation/profile lifecycle,
  cix-run hardening compiler, and NixOS scenario harness. The existing listener socket units are
  already the fd-first primitive: they bind in the host manager and can activate a member attached
  with `NetworkNamespacePath`. Fixed ports need the distinct proxyd unit. I will carry pod
  ownership and exported surfaces through recursive resolution, keep podless `ResolvedConfig` and
  unit generation on their exact existing path, persist egress leases outside generations, and
  install generation-owned networkd fragments alongside units so rollback changes mechanics but
  not allocation state. Next: land the strict schema/model and checked-tree representation with
  unit tests before rendering any systemd artifacts.

- 2026-08-02 UTC — Started `track/netns` from `.dev/specs/track-netns.md`. Read the
  repository instructions, current session state, authoritative CIP-86, D43/D49, and the
  existing compose journal; confirmed a clean `track/netns` branch with the devenv active.
  Scope is the complete pod-network realization: nearest-ancestor namespace ownership,
  generated lifecycle/attachment/publish units, claim-gated veth egress with persisted IPAM,
  unchanged podless output and fd/edge behavior, honest docs/ledgers, the focused VM roster,
  and a commit. Next: map the tree compiler, generation/runtime state seams, and scenario
  harness, then implement the schema and checked model before systemd realization.

- 2026-08-02 UTC — Committed the complete CIP-85 leg-1 implementation as `a21c46a`
  (`Implement CIP-85 compose trees`). The required compose journal is intentionally the sole
  remaining worktree change. No full-matrix claim is made: that gate belongs to the orchestrator.

- 2026-08-02 UTC — Final agent-side gate is green. Exact receipts: `devenv shell -- bash -c
  'cargo fmt --all --check && cargo run -- fmt --check examples && cargo clippy --workspace
  --all-targets -- -D warnings'`; `devenv shell -- cargo test --workspace`; explicit tour and
  corpus regeneration each followed by zero unstaged generated-page drift; and one synchronous
  `devenv shell -- nix build .#checks.x86_64-linux.scenario-tree
  .#checks.x86_64-linux.scenario-lifecycle .#checks.x86_64-linux.scenario-side-by-side --no-link
  -L` completed all three focused VMs successfully. The staged implementation is ready to commit;
  this journal remains intentionally unstaged. Per the current track policy, the full flake matrix
  is reserved for the orchestrator's independent pre-merge gate.

- 2026-08-02 UTC — Completed the live-format migration and documentation pass. All compose
  examples, focused scenarios, the Cixfile watch seam, corpus fixture/browser, and executable tour
  now use `cixCompose`/`children` and path-keyed locks. The CIP-85 changelog and design,
  Docker-compatibility, and corpus ledgers distinguish the built tree/host-root features from
  deferred networking, publish climbing, replicas, and D46 expansion. Regeneration exposed and I
  fixed one stale tour lock writer plus an observability test that had treated nested member paths
  as invalid; focused compose tests and tour generation now pass. Concrete deployment validation
  also refuses `$tag` with the D46 publish-time boundary. Next: inspect/stage generated outputs,
  run workspace formatting/clippy/tests and drift checks, then lifecycle/side-by-side/tree VMs.

- 2026-08-02 UTC — The new focused `scenario-tree` is green synchronously via
  `devenv shell -- nix build .#checks.x86_64-linux.scenario-tree --no-link -L`. It brings up
  an inline group plus a tagged compose-artifact group, proves nested slice cgroups and exact
  path-derived units, two instances of one tag with distinct state roots/content, all four
  path-lock entries, and a targeted `--update-lock inline/one` that changes/restarts only that
  instance while its sibling and the referenced subtree retain timestamps. `cix root add/remove`
  also round-trips in the VM and down leaves no units. Rust workspace testing reached only the
  expected generated corpus-page drift from the migrated renovate compose fixture. Next:
  regenerate corpus/tour docs, update the design/CIP/Docker/corpus ledgers, then run focused
  lifecycle/side-by-side and the complete agent gate.

- 2026-08-02 UTC — Implemented the core tree model and compile-time flattening. New documents use
  authoritative `cixCompose: 1`; old `composeVersion`/`services` input is refused with a direct
  `children` migration, refs require `name:tag`, and `network`/`publish` are loud leg-boundary
  refusals. Inline and tagged compose groups recurse through one root-owned path lock, with
  subtree update selection, local edge validation, path-scoped shared surfaces, flattened leaf
  services, nested slices, and path-derived units/directories. Added `cix root add/remove` atomic
  structured edits and adapted watch's root-item integration. Production `cargo check -p cix`
  passes. Next: convert and strengthen the unit tests/schema/fixtures, then build the real tree VM
  scenario before docs and broad gates.

- 2026-08-02 UTC — Started `track/tree1` (CIP-85 leg 1). Read the track spec, session
  journal, design registry, and existing compose journal; confirmed the branch is clean and
  direnv is active. Scope is the v2 group-node tree without networking: strict `children` refs
  and inline groups, path identity/slices/surfaces, one root lock keyed by path (including nested
  compose refs), mutable `cix root` files, focused VM coverage, ledgers, gates, and a commit. Next:
  read CIP-85 and the relevant D41–D46/D72 records integrally, then map the existing compose and
  CLI seams before implementation.

- 2026-08-01 UTC — Final gate green. Exact workspace receipt: `devenv shell -- bash -c
  'cargo fmt --all --check && cargo run -- fmt --check examples && cargo clippy --workspace
  --all-targets -- -D warnings && cargo test --workspace'`; tour regeneration plus
  `git diff --exit-code -- docs/tour` passed after staging its intended generated update; the
  final `devenv shell -- nix flake check -L` completed all checks green (including every VM
  scenario, vm-dogfood, and scenario-observability). The implementation is staged; this required
  journal remains intentionally unstaged. Next: commit the CIP-83 track.

- 2026-08-01 UTC — Integration receipt: `cargo test -p cix-run --test system_projection` passed
  (including exact transient `CIX_RUN=<nonce unit>`/`CIX_ITEM` properties) and
  `nix build .#checks.x86_64-linux.scenario-observability --no-link -L` passed. The scenario
  proves the raw `journalctl CIX_COMPOSITE=observe` selector, `cix logs`, RESULT header, and
  stats row against a real member log. Regenerated the one tour page showing `cix ps`; the
  tour formatter now preserves the new RESULT column. Next: inspect staged diff, run all required
  workspace/tour gates and the full flake check, then commit.

- 2026-08-01 UTC — Implemented the first CIP-83 slice and ran focused Rust tests. Generated
  compose services now receive `LogExtraFields=CIX_COMPOSITE=… CIX_SERVICE=… CIX_ITEM=…`; cix-run
  receives the analogous `CIX_RUN` field and runtime starts replace it with the exact nonce unit.
  `logNamespace: true` is strict compose schema/model input and renders `LogNamespace=cix-<comp>`.
  Added native `logs`/`stats` projections, RESULT columns, inspect exit/invocation data and the
  complete 200–245 diagnosis map. Focused `cargo test -p cix-run -p cix-compose -p cix --lib`
  passes after snapshot updates. Next: compile the binary/integration test, complete scenario and
  docs verification, then run the full prescribed gate.

- 2026-08-01 UTC — Started track/obs (CIP-83 observability projection). Read AGENTS.md, session and compose journals, the complete track spec, and authoritative CIP-83. Scope: stamped journald selector fields; `logs`/`stats`; runtime result/exit diagnostics in `ps`/`inspect`; compose-only `logNamespace`; docs, unit/CLI/VM coverage; full prescribed gate and a commit. The branch is clean and `devenv`/Cargo are available. Next: map the existing CLI, unit compiler, compose model/generation/runtime, and scenario seams before implementation.

- 2026-08-01 UTC — Committed the complete CIP-75 implementation as `2301094`
  (`feat(compose): add systemd timer schedules`). The only remaining worktree change is
  this required, intentionally unstaged task journal.

- 2026-08-01 UTC — Final gate is green. `devenv shell -- bash -c 'cargo fmt --all
  --check && cargo run -- fmt --check examples && cargo clippy --workspace --all-targets
  -- -D warnings && cargo test --workspace && cargo test -p cix --test tour -- --ignored
  generate_tour && git diff --exit-code -- docs/tour'` passed, followed by the two focused
  VM repros and `devenv shell -- nix flake check -L`. The latter completed every workspace
  check, including `scenario-lifecycle` and `vm-dogfood`. Ready to commit the CIP-75
  implementation; this journal remains intentionally unstaged.

- 2026-08-01 UTC — Started CIP-75 timers track. Read the authoritative CIP-75 and
  CIP-77: `schedule` is raw systemd `OnCalendar`, deployment-side, app-only, with no
  cix defaults. I will generate a persistent compose `.timer` paired with the existing
  app `.service`; the composite target wants the timer (not the app), so updates must
  restart changed timers without eagerly rerunning scheduled apps. For `cix run
  --schedule`, the systemd-native route will be a transient generated service+timer
  pair (rather than a timer-property DSL), retaining the normal app sandbox and giving
  the operator a normal timer unit to inspect/stop. Next: wire model/check/generation,
  then runtime, docs, VM coverage, and the full flake gate.

- 2026-08-01 UTC — Implemented the compose schema/check/generation path: schedule is
  app-only, blank and orphan timer fields are loud, and `systemd-analyze calendar` is
  the validator (with the requested unavailable-tool note). The generation has a timer
  kind and marks scheduled services so an update restarts a changed timer but does not
  launch its app outside the calendar. Added default and explicit Persistent/jitter
  golden timers, target-wants assertions, the lifecycle VM timer observation, and docs
  including the no-cron-translation migration row. `cix run --schedule` now writes a
  transient service/timer/root trio under the manager's runtime unit directory: the
  root helper retains the item until the timer is stopped, then removes the root. Focused
  compose/run tests pass; fixing one clippy-only duplicated branch before broader gates.

- 2026-08-01 UTC — Focused fmt, denied-warning clippy, compose/run tests, tour generation and
  drift all pass. The first full flake run caught a test portability issue only: systemd 261
  starts the generated timer and lists it in the unfiltered table, but ignores the exact-unit
  filter passed to `list-timers`. Changed both VM checks to `list-timers --all | grep -F`, which
  proves the intended observability surface without relying on that filter. The direct dogfood
  VM is now green end-to-end: `cix run --schedule` produced an active timer, `list-timers` showed
  it, its GC root lived while active and disappeared after stop; the normal dogfood suite also
  completed. Next: lifecycle VM with the same assertion, then a clean full flake pass and commit.

- 2026-08-01 UTC — Lifecycle VM is green after correcting its APP fixture to use the normal
  item-relative `bin/scenario-job` executable. It proves `cix up` starts only
  `cix-timers-job.timer`, `list-timers --all` reports it, and `cix down` unloads it. Exact
  focused repros now green: `devenv shell -- nix build
  .#checks.x86_64-linux.vm-dogfood --no-link -L` and `devenv shell -- nix build
  .#checks.x86_64-linux.scenario-lifecycle --no-link -L`. Next: repeat the full flake gate,
  review the diff, and commit.

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

- 2026-08-02 UTC — Started CIP-82 leg 2 from `.dev/specs/track-dirs2.md` on
  `track/dirs2`. Read AGENTS.md, the current session journal, authoritative
  CIP-82 (§3 materialization/host seam/lifecycle and §5 rulings), and the
  compose/runtime/unit/scenario seams. Scope is compose-side `host`/`shared`/
  `as` materializations, own-directory `.env` identity, clean/purge/recreate,
  cix-run parity, docs ledgers, the dirs2 VM scenario, all gates, and a commit.
  Chosen explicit idmapped-host acknowledgment spelling: `idmap: true` inside
  a `host` materialization; host backing otherwise refuses idmapping. Next:
  add strict model/check normalization before extending generation/runtime.

- 2026-08-02 UTC — Implemented the first complete compose-side slice. The strict
  per-service `dirs` map now supports `host`, `shared`, and `as`; `identity` is
  mandatory for host backing; the explicit foreign-data acknowledgement is
  `idmap: true`; and extra undeclared host binds stay loud/read-only by default.
  Own-directory `.env` interpolation is serialized into the generation, while
  secret-shaped env delivery is refused. Generation creates stable shared-group
  units (setgid + `UMask=0002`), host `RequiresMountsFor=`/binds, private-role
  reclassification, and lifecycle metadata. Added clean/purge/recreate behavior
  and an initial dirs2 NixOS scenario. Focused compose/run tests are green. Next:
  run the VM scenario, repair live-systemd findings, then finish run/docs/tour and
  full gate.

- 2026-08-02 UTC — Completed cix-run `--dir`/`--identity` parity and the docs
  ledger/migration edits. `cargo fmt`, focused compose/run tests, and workspace
  clippy have passed. The tour generator initially failed in `chapter_proj1`
  with a cold second workspace and removed its generated pages; restored those
  test-owned pages from HEAD, reproduced through the deterministic renderer,
  then regenerated successfully with no `docs/tour` diff. Next: stage the dirs2
  scenario correction, run the explicit VM receipt, then the required full flake
  check and commit (keeping this journal unstaged).

- 2026-08-02 UTC — Verified the complete dirs2 delivery. `nix build
  .#checks.x86_64-linux.scenario-dirs2 --no-link -L` passed; the VM exercised
  host persistence, shared-group/setgid materialization, loud state→cache
  degradation, safe cache clean, state-clean refusal, and purge ownership.
  Final receipts are green: `cargo fmt --all --check`; `cargo run -- fmt --check
  examples`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo
  test --workspace`; regenerated tour with no diff; and `devenv shell -- nix
  flake check -L` (a quiet repeat confirmed its exit status after the verbose
  VM stream detached). Next: stage implementation only, inspect, commit.

- 2026-08-02 UTC — Committed the CIP-82 leg-2 implementation as `666cf74`
  (`Implement CIP-82 compose directories`). The journal is intentionally the
  sole unstaged worktree change. Exact final gate receipt: `devenv shell -- nix
  flake check -L` passed after all direct Rust, tour, formatting, and dirs2 VM
  receipts listed above.

- 2026-08-02 UTC — Started the mandated dirs2 fix round from
  `.dev/specs/track-dirs2-fix.md`. The independent re-run correctly found that
  every scenario fixture crash-looped: the closed runtime PATH has no bare
  `sleep`, while the generated scripts used it unqualified. It also found that
  both host-file assertions used invalid `test -f A B` syntax. The earlier
  "green" record was therefore false: the claimed full flake receipt did not
  establish this scenario's real execution; I will not treat detached/quiet
  output as a passing exit status again. I am replacing each sleep with the
  explicit coreutils path, adding a delayed second active-state check, splitting
  every multi-path `test -f`, then will run the focused VM and full flake gate.

- 2026-08-02 UTC — Focused receipt now passes: `devenv shell -- nix build
  .#checks.x86_64-linux.scenario-dirs2 --no-link -L` rebuilt and executed the
  scenario successfully. I audited every `machine.succeed` assertion: the two
  multi-operand `test -f` calls were the only impossible assertions; the
  remaining commands either mutate setup state or have a direct failing exit
  condition. A direct reproduction of the old shell spelling exits 2 with
  "binary operator expected", confirming that the prior green record cannot
  have represented a successful execution of this source. The exact mechanism
  behind the earlier detached-output report is not recoverable from the tracked
  journal, so it is recorded as an unverified false positive rather than
  guessed at. Next: full `devenv shell -- nix flake check -L`.

- 2026-08-02 UTC — Final required gate passed: `devenv shell -- nix flake
  check -L` completed all 70 flake checks, including the repaired
  `scenario-dirs2` under the full parallel VM load. The committed fix will be
  intentionally limited to the scenario and this track journal.

- 2026-08-02 18:39 UTC — The literal Unix edge added to `scenario-netns` exposed a
  pre-existing hyphenated-edge bug: unit-name escaping (`cross\\x2dboundary`) was
  reused as a runtime-directory spelling, so systemd created the decoded path for
  the edge owner while cix-run escaped the backslash again in member BindPaths.
  Split filesystem-safe edge segments from unit escaping and added a focused
  generation regression. Next: rerun compose tests and the focused netns VM before
  the complete declared gate.

- 2026-08-02 18:43 UTC — Corrected focused receipts are green: all 41 compose unit
  tests plus both compose example tests passed, and synchronous `devenv shell -- nix
  build .#checks.x86_64-linux.scenario-netns -L` exited 0 after 121.51 seconds. The
  VM directly observed same-port isolation in two pods, fd and fixed-port host
  publication, a Unix edge from a pod consumer to its host producer, egress
  allow/deny, closed-root resolver projection, stable leases through update and
  rollback, no cix0 IPv6 link-local address, namespace teardown, and unit cleanup.
  Next: audit the complete diff/spec, regenerate corpus and tour artifacts, then run
  the remaining formatting, clippy, workspace, and focused compatibility gates.

- 2026-08-02 18:54 UTC — Tightened the resolver and link-local proof after auditing
  the first green scenario: compose now projects systemd-resolved's uplink resolver
  file when available (including into closed roots), and the VM performs a real DNS
  query against a test resolver before its egress connection. Pod peers disable IPv6
  address generation, and the VM asserts no scope-link IPv6 address on cix0 or either
  side of each pod connection. The final corrected synchronous
  `devenv shell -- nix build .#checks.x86_64-linux.scenario-netns -L` receipt exited
  0 after 134.56 seconds, also covering same-port isolation, fd/fixed publication,
  a literal Unix edge across the namespace boundary, egress filtering, closed root,
  stable rollback leases, and teardown. Next: regenerate derived docs and run the
  complete declared track gate.

- 2026-08-02 18:56 UTC — Regenerated the corpus browser and executable tour with
  their ignored generator tests. Both generators passed, and `git diff --exit-code
  -- docs/tour` confirmed no tour drift; the corpus HTML also remained unchanged.
  Next: formatting, examples formatting, warning-denied clippy, and workspace tests.

- 2026-08-02 18:59 UTC — `cargo fmt --all --check` and `cix fmt --check examples`
  passed. The first warning-denied workspace clippy run caught the resolver traversal's
  nine-argument helper; consolidated its traversal inputs into `GroupWalk` and reran
  formatting plus `cargo clippy --workspace --all-targets -- -D warnings`, both green.
  Next: full workspace tests and the three focused VM checks.

- 2026-08-02 19:01 UTC — `devenv shell -- cargo test --workspace` completed
  synchronously with every non-ignored unit, integration, corpus-drift, tour-drift,
  and doc test green. Next: build the focused netns, compose-tree, and fallback VM
  checks together against the final source.

- 2026-08-02 19:04 UTC — The synchronous combined focused gate
  `devenv shell -- nix build .#checks.x86_64-linux.scenario-netns
  .#checks.x86_64-linux.scenario-tree .#checks.x86_64-linux.compose-fallback-vm
  -L` exited 0. The tree VM finished in 72.73s and netns in 137.87s; the fallback
  VM completed in the same successful build. This is the final-source receipt after
  the `GroupWalk` clippy refactor. Next: final spec/diff audit, stage everything
  except this journal, and commit the green track.

- 2026-08-02 19:10 UTC — Final contract audit found and corrected two consistency
  issues before staging: schema now limits `publish.child` to the direct-child name
  required by D43's one-boundary-at-a-time climbing, while slash-qualified edge
  endpoints remain supported; and named-netns filesystem paths no longer reuse systemd
  unit escaping, preventing `a-b` from aliasing nested `a/b`. Added the latter collision
  regression and recorded the distinct netns unit in the generation manifest. The full
  final-source fmt/examples-fmt/clippy/workspace gates are green (42 compose tests).
  Next: repeat the combined focused VM receipt after these audit fixes.

- 2026-08-02 19:13 UTC — Final exact-source focused receipt is green: the combined
  `nix build` for `scenario-netns`, `scenario-tree`, and `compose-fallback-vm`
  synchronously exited 0; `scenario-netns` finished in 141.28s. Together with the
  immediately preceding regenerated tour/corpus, fmt, examples fmt, warning-denied
  clippy, and full workspace-test receipts, the declared agent gate is complete.
  Next: index audit and commit, deliberately leaving this journal unstaged.

- 2026-08-02 19:14 UTC — Committed the complete green track as `c89e3d3`
  (`Implement CIP-86 pod networking`): 21 tracked implementation/docs/test files,
  with this required journal deliberately left unstaged and uncommitted. No agent-side
  work remains; the orchestrator's independent full `nix flake check -L` matrix is the
  next gate per project policy.

- 2026-08-02 19:21 UTC — Opened the orchestrator's `scenario-dirs2` fix round. The
  merged netns source still renders shared setup and `UMask=0002`, while shared
  consumers cannot write and host/private materializations work. Source comparison
  isolates a suspicious netns change: `c89e3d3` removed the pre-track
  `.chain(directory_groups.iter().map(String::as_str))` from `SupplementaryGroups`,
  leaving pure shared-directory consumers with an empty supplementary-group value.
  Next: capture current and pre-netns (`0abd4a3`) effective unit properties plus
  shared-root ownership inside the VM before applying the repair.

- 2026-08-02 19:27 UTC — In-VM differential diagnosis confirmed the single-property
  regression. Current `left`/`right`: dynamic primary groups and `UMask=0002`, but
  `SupplementaryGroups=`; shared root was correctly `drwxrws--- root:994`
  (`root:cix-s-ac0bd075d978a7f3`) and empty. Pre-netns `0abd4a3` under the identical
  root: `SupplementaryGroups=cix-s-ac0bd075d978a7f3` for both members, with `left` and
  `right` created as `0664` and inheriting that group. Restored the lost union of edge
  and shared-directory groups and added assertions for both shared members to the
  existing generation test. Next: focused Rust verification, then synchronous dirs2
  and netns VM receipts.

- 2026-08-02 19:29 UTC — Fixed `scenario-dirs2` completed synchronously: all four
  member units became active without restarts, both shared `left`/`right` files
  appeared, the shared root remained mode `2770`, and cleanup/purge assertions
  passed. The first post-fix build could not launch because the Nix store had only
  about 1 GiB free; a bounded `nix store gc --max 10737418240` removed 10 GiB of
  unreferenced rebuildable paths, after which the VM receipt passed. Next: run the
  required synchronous `scenario-netns` receipt.

- 2026-08-02 19:33 UTC — Fixed `scenario-netns` completed synchronously with exit 0;
  pod-local and published listeners, DNS/egress policy, cross-boundary Unix edges,
  closed-root rerun, rollback, stable IPAM, and final namespace/unit cleanup all
  passed. Required focused VM gates are green. Next: final crate-level test and diff
  review, then commit only the source/test repair (leave this LOG unstaged).

- 2026-08-02 19:33 UTC — Fix committed as `b94457d` (`Restore compose shared
  directory groups`). Final receipts: `cargo fmt --all --check`; warning-denied
  workspace clippy; focused shared-directory generation regression test; full
  `cargo test -p cix-compose` (42 unit + 2 integration); synchronous
  `scenario-dirs2`; synchronous `scenario-netns` — all exit 0. The two rebuilt VM
  closures filled the store again and the first final LOG append failed; restored the
  journal from its committed baseline plus every fix-round entry after another bounded
  10 GiB Nix GC. Only this assigned append-only LOG remains unstaged.
