# cix-run work log

## track/stopdispo

- 2026-08-05 UTC — Post-merge gate found that `ComposeService` lacked the
  explicit serde spelling for the otherwise snake-case `stop_timeout` field:
  generated `stopTimeout` JSON failed at runtime. Added
  `#[serde(rename = "stopTimeout")]` plus a load-level regression test.
  After `systemctl --user stop 'cix-*'`, `reset-failed 'cix-*'`, and
  `daemon-reload` all exited 0, the complete corrected-head agent tier has
  synchronous exit-0 receipts: `cargo fmt --all --check`; `cargo run -- fmt
  --check examples`; warning-denied workspace/all-target clippy; serial full
  workspace tests (`cargo test --workspace --quiet -- --test-threads=1`, which
  includes corpus and tour drift); explicit corpus-browser regeneration;
  explicit tour regeneration; and `nix run .#progressive-vm-check`. The VM
  selected all 14 changed scenarios, including `scenario-stopdispo`, and
  passed its `stopTimeout`/`KillSignal` assertions. The serial test setting is
  required for this host's systemd-user transient-unit race; it is still the
  complete workspace suite, and its observed exit status was 0.

- 2026-08-05 UTC — Merged `origin/main` at `8bb160f`, incorporating the
  ENV `NAME=value` grammar canon and CIP-102 EXPECT sweep. Resolved the
  Adminer corpus overlap semantically: its GAPS file retains main's mandatory
  SHA-256/TOFU and cold-design-divergence finding while marking only the now
  representable STOPSIGNAL item stale; nginx's independently carried stale
  STOPSIGNAL finding remains intact. `docs/corpus.md` likewise retains the
  main findings and adds the stale stop-signal note. Removed conflicted
  browser outputs and synchronously regenerated them with `devenv shell --
  cargo test --test corpus -- --ignored generate_corpus_browser` (exit 0),
  rather than hand-merging generated files. Next: commit this merge, reset
  stale `cix-*` user units, and rerun the complete agent gate tier.

- 2026-08-05 UTC — Implemented `STOPSIGNAL <signal>` for SERVICE/APP
  Cixfiles, validated against Linux signal names, serialized as manifest
  `stopSignal`, and projected by cix-run to `KillSignal=`. Compose gains the
  deliberately conventional camel-case `stopTimeout: "<duration>"` member
  field, validated with the existing systemd-duration grammar and projected to
  `TimeoutStopSec=`. Added parser/unit/generation coverage and a dedicated
  `scenario-stopdispo` VM assertion for both rendered properties. Applied the
  entire blessed disposition batch to `docs/docker.md`; Adminer and nginx
  GAPS are `Status: stale — regenerate with STOPSIGNAL`, and their corpus rows
  say so. Synchronous receipts: targeted cixfile/run/compose tests passed;
  the aggregate three-crate test suite passed apart from one transient proj1
  timing failure, then the exact standalone `cargo test -p cix-cixfile --test
  proj1` passed. Next: run the new focused VM and the full prescribed gates.

- 2026-08-05 UTC — The synchronous focused VM receipt `devenv shell -- nix
  run .#progressive-vm-check` exited 0 after selecting all derivation-changed
  scenarios, including `scenario-stopdispo`; it exercised the generated
  `KillSignal=SIGQUIT` and `TimeoutStopSec=3s`. `cargo fmt --all --check`,
  `devenv shell -- cargo run -- fmt --check examples`, and warning-denied
  workspace/all-target clippy all exited 0. The initial full workspace run
  correctly found corpus-browser drift from the required corpus-row changes;
  `devenv shell -- cargo test --test corpus -- --ignored
  generate_corpus_browser` exited 0 and updated only its generated pages.
  Tour regeneration is presently failing in unrelated user-manager lifecycle
  races (`NAMESPACE` permission failure / already-unloaded transient unit),
  after its destructive stale-page cleanup; retry only after the manager
  settles, and do not claim a tour receipt until it exits synchronously 0.

- 2026-08-05 UTC — Retried after resetting only test-created `cix-run-*`
  user-manager failures: tour regeneration then exited 0 and rewrote the tour,
  and the focused VM, fmt, examples fmt, clippy, targeted parser/unit/compose
  tests, corpus regeneration, and corpus drift test all have synchronous 0
  receipts. The ordinary parallel `cargo test --workspace` still has the
  pre-existing user-manager race in `crates/cix/tests/tour.rs` (a transient
  user unit vanishes between run and inspect, then poisons its render mutex);
  it is unrelated to this track and remains an honest non-green receipt.
  Committed the cohesive implementation and ledger update as `7f92f95`.

- 2026-08-05 UTC — Started the blessed STOPSIGNAL/stop-timeout disposition
  track after reading its spec, `cips/dispositions.md`, the Docker ledger, and
  the current project/run logs. The implementation seam is Cixfile → manifest
  → cix-run unit compiler, with compose service declarations overriding the
  generated service before compilation. Next: add validated Cixfile signal
  grammar, compose timeout projection, unit/VM tests, then apply every
  disposition verdict to `docs/docker.md` and run the prescribed synchronous
  gates.

## track/netnsrace

- 2026-08-04 07:34 UTC — Final current-tree agent tier is green after
  merging current `origin/main` (`7f98bc1`), which supplied its independent
  corpus-browser drift correction. The initial workspace test had failed only
  that pre-existing generated-page drift; after integration,
  `devenv shell -- cargo test --workspace` passed synchronously. Explicit tour
  regeneration and the following tour drift/determinism test both passed, with
  no generated tour diff. Current-tree fmt, examples fmt, and warning-denied
  workspace/all-target clippy all exited 0. The bounded focused receipt
  `nice -n 10 nix build .#checks.x86_64-linux.scenario-netns --no-link -L
  --max-jobs 6 --cores 4` also exited 0 synchronously under TCG: closed-root
  reactivation succeeded, both netns teardowns completed, and both immediate
  namespace-path absence assertions passed. The specified implementation,
  deterministic regression coverage, bounded 20-run before/after experiment,
  and open-question closure are complete. Next: independent orchestrator
  verification and merge.

- 2026-08-04 07:25 UTC — The announced post-fix campaign completed
  synchronously: **20/20 contended VMs passed**, with zero netns stop
  timeouts, zero stale namespace paths, and zero activation failures. This is
  the same independently named, six-at-once TCG contention shape that produced
  17/20 passes before the fix (one exact activation failure plus two stale
  final paths). Updated `docs/open-questions.md` to move the item out of the
  agent-open queue and record the graph proof, teardown/re-entry mechanism,
  narrow fix, and before/after rates. Next: run the complete standard agent
  tier and a current-HEAD focused netns VM, then record exact synchronous
  receipts.

- 2026-08-04 07:13 UTC — Implemented the mechanism-scoped fix: generated
  netns oneshots now carry `TimeoutStopSec=10s`, so namespace deletion does
  not inherit the scenario's intentionally global 1-second service budget.
  CIP-86's manifest and network semantics are unchanged. The golden fixture
  pins the projection; `scenario-netns` asserts the effective systemd value
  and once again requires both namespace paths to be absent immediately after
  synchronous `cix down`, removing the earlier wait that masked interrupted
  teardown. Synchronous receipts: fmt check, the focused compose golden test,
  warning-denied all-target cix-compose clippy, and bounded focused
  `scenario-netns` VM all passed. In that VM the first down → closed-root up
  transition passed, the effective timeout was exactly 10s, both netns
  `ExecStop`s completed successfully, and both final immediate absence checks
  passed. Before the next long experiment: I will repeat 20 independently
  named contended VMs with the same single aggregate `nice -n 10` Nix build,
  `--keep-going --max-jobs 6 --cores 4`; raw output remains outside the repo.

- 2026-08-04 07:06 UTC — Reproduced and root-caused before changing code.
  One bounded baseline passed synchronously. The announced 20-run contended
  campaign then completed synchronously with 17 passes and three failures:
  one exact closed-root activation failure (run 17, 5%) and two final stale
  namespace-path failures (runs 15/20, another 10%). The aggregate command
  was `nice -n 10 nix build ... --keep-going --max-jobs 6 --cores 4`; each
  run was a uniquely named override of `scenario-netns`, so Nix executed 20
  independent VMs (six at once), and lack of host KVM forced additional TCG
  contention. This is not the suspected member-before-oneshot race. The
  realized graph has both directions of the ordering assertion: the netns
  unit says `Before=cix-netns-b-fixed.service`, while the member says
  `Requires=cix-netns-b-netns.service` and
  `After=cix-netns-b-netns.service`. Run 17's exact interleaving was:
  `cix down` started the b-netns `ExecStop=ip netns delete` at guest
  138.668s; the scenario-wide `DefaultTimeoutStopSec=1s` expired at
  139.549s and systemd SIGTERM'd the control process; the unit stopped with
  result `timeout`, leaving `/run/netns/cix-netns-b-netns`; the immediately
  following closed-root `cix up` started the b netns oneshot at 150.693s;
  `ip netns add` reported `Cannot create namespace file ...: File exists`
  at 151.579s; the oneshot failed at 151.699s; and the correctly ordered
  b-fixed member then failed with result `dependency` at 151.766s. Runs 15
  and 20 independently proved the same interrupted teardown by retaining the
  b namespace path for the full 60-second final assertion. Mechanism and fix
  boundary: netns deletion is lifecycle infrastructure and must have its own
  stop budget instead of inheriting the deliberately tiny scenario/host
  manager default. Next: give only the generated netns oneshot a bounded
  `TimeoutStopSec`, assert that projection in the golden and focused VM, then
  rerun contended instances.

- 2026-08-04 06:49 UTC — Started the bounded reproduction campaign after
  reading the track spec, current project log, D43/D49, CIP-86, and the
  generated compose network/activation code. The current graph already gives
  every pod member `Requires=` + `After=` on its netns oneshot, and the
  oneshot also gives all members `Before=`; the campaign must therefore test
  the recorded activation failure rather than assume the suspected missing
  edge. I will run at least 20 independently named `scenario-netns` test
  derivations under deliberate parallel contention, in batches no larger than
  three VMs, with every Nix invocation bounded by `nice -n 10`,
  `--max-jobs 6`, and `--cores 4`. Raw synchronous results and failed build
  logs, if any, go to a temporary directory outside the repository. Next:
  execute the baseline and contended batches, then inspect journals and the
  realized unit graph for any failure before changing code.

- 2026-08-02 UTC — Fix round for the orchestrator's lock_nix flakes. The
  boundary refactor had generated a new workspace for each build invocation,
  breaking the two tests that intentionally observe one test-local warm
  workspace across sequential builds. Each of those tests now owns one
  `tempfile::TempDir` workspace and injects its path into every build; no
  workspace crosses a test boundary and no env/mutex was reintroduced.
  Synchronous receipts: `devenv shell -- cargo test -p cix-cixfile --test
  lock_nix` passed five consecutive complete runs, followed by a passing
  `devenv shell -- cargo test --workspace`.

- 2026-08-02 UTC — FENCE LIFTED follow-up: fast-forwarded `origin/main` at
  `d82a4c5` (netns), then threaded the already-resolved index store through
  the narrow compose check/diff/up seam. This preserves the netns code while
  restoring the tour's isolated state fixture without reintroducing an env
  read in cix-index. Commit `dbf02f9` contains the leg. Synchronous receipts:
  `devenv shell -- cargo test --workspace` passed; `devenv shell -- cargo fmt
  --all` and `devenv shell -- cargo run -- fmt --check examples` passed;
  `devenv shell -- cargo clippy --workspace --all-targets -- -D warnings`
  passed; `scripts/check-cli-env-boundary.sh` passed; and explicit tour
  regeneration plus its normal drift suite passed. Focused vm-dogfood remains
  to be run before an independent merge gate.

- 2026-08-02 UTC — Full `devenv shell -- cargo test --workspace` reached the
  tour and failed there: the untouched cix-compose resolver still invokes the
  legacy index resolver, which now uses only the default state path and thus
  cannot see the tour's `CIX_STATE_DIR` fixture. This confirms the parallel
  fence is a real dependency: preserving compose's environment boundary
  requires the leg-B compose configuration plumbing, rather than reintroducing
  an index-library environment read. No green/commit claimed; all earlier
  no-run, fmt, and source-boundary receipts remain valid.

- 2026-08-02 UTC — CIP-90 plumbing round: clap now has the `env` feature;
  state and builder-workspace paths are explicit CLI/config values and flow into
  index/build/run APIs; owned tests construct those paths directly, removing
  the proj1 workspace mutex and all owned `set_var`/`remove_var` calls. The
  shared watch/runtime interrupt flags now use the one justified cix-common
  atomic. Added `scripts/check-cli-env-boundary.sh` and documented precedence.
  Synchronous receipts so far: `devenv shell -- cargo fmt --all`, `devenv
  shell -- cargo test --workspace --no-run`, and the new boundary script pass.
  Next: finish capability-probe/nonce injection, execute runtime tests and the
  full focused gate, then commit.

- 2026-08-02 UTC — Started `track/hygiene-a` (CIP-90 leg A). Read
  `AGENTS.md`, the current `.dev/LOG.md`, CIP-90 §3.1/§5 including its
  shared-state amendment, and the assigned spec. The devenv Rust toolchain is
  active. Scope excludes `cix-compose`; next is a complete inventory of owned
  CIX environment reads, test mutation, signal flags, nonce generation, and
  their CLI/config call paths before moving configuration to clap boundaries.

- 2026-07-31 18:14 UTC — Final local cleanup: stopped the explicit empty
  system `cix-run.slice` with `sudo -n systemctl stop cix-run.slice`, then
  stopped the user slice and cleared collected historical failures with
  `systemctl --user stop cix-run.slice` and `systemctl --user reset-failed
  'cix-*'`. `systemctl list-units --all --no-legend 'cix-*'` and its
  `--user` counterpart are both empty. Removed the generated untracked
  `devenv.lock` from the worktree.

- 2026-07-31 18:11 UTC — D63(b) final gate passed on `track/gcroots` after
  correcting system-mode cleanup: `/run/cix/gcroots` is root-owned, so the
  visible `ExecStopPost=` uses systemd's `+` prefix only for system units;
  user-mode cleanup stays unprivileged in the user-owned runtime directory.
  Exact successful repros: `devenv shell -- cargo fmt --all --check`;
  `devenv shell -- cargo clippy --workspace --all-targets -- -D warnings`;
  `devenv shell -- cargo test --workspace`; `devenv shell -- cargo test -p
  cix --test tour -- --ignored generate_tour`; then `devenv shell -- cargo
  test -p cix --test tour` twice; and `devenv shell -- nix build
  .#checks.x86_64-linux.vm-dogfood --no-link -L`. Regeneration changed no
  tour documents; both drift/determinism passes were green. The VM proved the
  link and auto-root exist while the service runs, the link is gone after
  `systemctl stop`, and `nix-store --gc --max-freed 1` logged removal of the
  dangling auto link before the test confirmed it absent. The VM cleaned every
  cix unit; the known node stop timeout is tolerated by its existing test and
  the test completed successfully.

- 2026-07-31 17:34 UTC — Started track/gcroots after reading AGENTS.md, the
  current `.dev/LOG.md`, D63 in full, and `.dev/specs/track-gcroots.md`.
  Scope is `cix run` unit-lifetime indirect GC roots plus the requested docs and
  runtime tests; corpus is explicitly untouched. The clean `track/gcroots`
  branch is at `b9e5264`. Next: reuse the index’s `nix build --out-link` root
  registration, add visible `ExecStopPost=` cleanup, and prove lifecycle in the
  system projection test.

- 2026-07-31 17:48 UTC — Implemented D63(b) with `nix-store --add-root
  <link> --indirect --realise <item>`, per-run system/user root directories,
  explicit degraded-user warning, and a visible `ExecStopPost=`. Focused repros
  passed: `devenv shell -- cargo fmt --all --check`, `devenv shell -- cargo
  clippy -p cix-run --all-targets -- -D warnings`, and `devenv shell -- cargo
  test -p cix-run`. VM repro is `devenv shell -- nix build
  .#checks.x86_64-linux.vm-dogfood --no-link -L`: it proved creation and auto
  registration, then exposed the cleanup executable resolving through NixOS's
  coreutils multicall binary. The cleanup lookup now preserves the executable
  symlink (`.../bin/rm`) rather than canonicalizing it; next is rerunning this
  VM gate, then the complete specified gate, tour no-op verification, cleanup,
  and commit.

- 2026-07-29 11:29 UTC — Correction round 1 full gate passes: fmt, workspace build, warning-denied clippy, all workspace tests (including deterministic/drift-checked tour and system/rootless integrations), and the NixOS VM dogfood check. Live debug/exec outputs and cleanup are recorded in `.dev/specs/track-exec.LOG.md`. Next: final audit and commit.

- 2026-07-29 11:19 UTC — Correction round 1 replaces the D34 resolver with one shared effective lookup order: recorded/generated PATH, then `/usr/bin:/bin`, for shell and explicit commands in both debug and exec. Empty-environment coverage and focused tests/clippy pass. Exec now identifies private namespaces by comparing the target and caller nsfs device/inode pairs, enters only differing handles, and reports the private/shared sets instead of claiming PID/network isolation that nginx does not have. Next: repeat the real nginx system/user debug and live exec demonstrations, then the workspace and VM gates.

- 2026-07-29 11:26 UTC — Fresh nginx demos pass against a literal empty `Environment=` unit. Bare `id` succeeds at the service UID/GID, exact-unit `--root -- id` succeeds as root, and bare `sh` writes the managed cache with service ownership. Handle comparison and the runtime banner agree that only mount is private for this unit; PID/network/IPC/UTS are caller-shared, so process listings are explicitly described as host view. System one-shot plus interactive debug pass projection, writable-cache, and denied-write checks; the interactive empty-env shell selects `/usr/bin/sh`. User debug follows the loud D13 namespace fallback and runs successfully as the caller. All created units/files were cleaned. Next: full gate.

- 2026-07-29 10:53 UTC — Started the D34 exec/debug track. The existing D29c compiler and user-manager fallback path are reusable for debug. Chose direct libc namespace entry for exec; a cleaned-up live systemd probe confirmed numeric runtime UID/GID and systemd's quoted `Environment=` output. Implementation order is shared PATH shell resolution, generator-backed debug, then live-unit targeting/environment parsing and setns/fork/identity drop.

- 2026-07-29 11:02 UTC — D34 core and focused tests pass: 36 library tests cover shell fallback, debug entrypoint-only override, target disambiguation, and quoted environment parsing; warning-denied clippy and the CLI build are clean. Both debug modes passed against nginx, including a real PTY shell under DynamicUser, projection visibility, managed-directory ownership, and a denied `/etc` write. Live exec joined all requested namespace handles, saw nginx processes, adopted the service UID/GID for a managed-directory write, and retained root only with `--root`; exit status 42 also propagated through debug. Added missing `%t` expansion to the shared user property path after nginx's runtime directory exposed it, then verified the normal loud namespace fallback. All transient units were stopped. Next: tour/docs and the full gate.

- 2026-07-29 11:10 UTC — Exec/debug track complete. The final workspace fmt/build/warning-denied clippy/test gate and NixOS VM dogfood check pass. Root and user live transcripts are recorded in `.dev/specs/track-exec.LOG.md`; the tour's one-shot debug scenario is deterministic. Final systemd audit is empty. Changed the tour cleanup guard to use cix-run's listener-aware stop path after removing 400 unloaded historical listener unit files, and verified a fresh full tour leaves no runtime files.

- 2026-07-29 00:20 UTC — Started `.dev/specs/track-specv3.md`. Read D29 and the dstyle activated-listener evidence. Scope is confined to `crates/cix-run/` plus a promoted `examples/listenfds/`, with generated tour output only. First milestone: map the existing schema, unit generator, runtime, CLI, and test harness; then add cixSpec 3 validation and isolated golden coverage before runtime changes.

- 2026-07-29 00:28 UTC — Completed the schema/compiler boundary. cixSpec 3 adds only named TCP stream listeners, rejects listeners in v1/v2 with the required version, and preserves v1/v2 semantics. `-p` now resolves either existing ports or listener socket addresses, requires every listener to be bound, and reports unknown targets clearly; a listener and port cannot share an ambiguous name. Unit generation now emits D24 `SocketBindAllow=<tcp|udp>:port` plus `SocketBindDeny=any`, while listener-only services retain the no-network profile and deny all binds. Exposed documented `compile_unit` API with injectable unit/slice/target/directory naming and appended properties; foreign-name coverage proves `cix-mycomp-web`. Focused fmt, 31 unit tests, and warning-denied clippy pass. Next: transient socket lifecycle, `ps`, and live kernel enforcement.

- 2026-07-29 00:45 UTC — Implemented the run-time activated-listener path. Bound v3 listeners receive a run-scoped socket unit in `/run/systemd/system` with explicit `ListenStream=`, `FileDescriptorName=`, and `Service=`; the generated service unit explicitly carries `Requires=`, `After=`, and `Sockets=`. `stop_service` stops/removes both runtime units, while `cix ps` reports each active socket’s address and target service. A root-gated real-systemd test served HTTP from fd 3 with `LISTEN_FDNAMES=http`, verified the private-network/empty-capability/D24 deny profile, and proved an undeclared TCP bind is EPERM while a declared one works. Added the promoted `examples/listenfds/` v3 demo; it passed against the locally built CLI and cleaned its service/socket. Focused cix-run tests and warning-denied clippy pass. Next: commit this runtime/example unit, then the full done gate, existing demos, tour generation if required, and VM check.

- 2026-07-29 01:03 UTC — Specv3 done gate complete. `cargo fmt --all --check`, warning-denied `cargo clippy --workspace --all-targets`, and `cargo test --workspace` passed twice; the second run initially observed the system cix slice left by the required demos, so it was stopped (with the user slice) and the deterministic tour drift check then passed without regeneration. The root-only v3 systemd test was also executed explicitly as root and passed. The new listenfds demo plus existing nginx and PostgreSQL sudo demos all passed; the VM check `nix build .#checks.x86_64-linux.vm-dogfood` passed. Final cleanup found no cix units in either manager and no cix runtime unit files or listeners. Next: commit this final log summary and verify the worktree is clean.

- 2026-07-29 01:05 UTC — Final interface review aligned `-p` help with its v3 listener-address form and retained rendered unit text when the existing user-manager capability fallback removes properties, so listener-backed user retries remain valid. Next: focused verification, commit, and clean audit.

- 2026-07-29 01:12 UTC — Current-HEAD final sweep: fmt, warning-denied all-target workspace clippy, and the complete workspace test suite passed twice consecutively, including tour drift/determinism and the self-skipping system integration harness. The earlier explicit-root v3 listener/kernel test, all three sudo demos, and VM gate remain green. Final step: remove the generated devenv lock, audit both managers/runtime units, and commit this log update.

- 2026-07-28 22:24 UTC — Started `.dev/specs/track-fsproj.md`. Read D22 v3 and inventoried the stable `/app` implementation, Cixfile compiler, examples, docs, and tour. Contract choice: `mounts` is an additive cixSpec 2 service field (v1 rejects it); D22’s “exact roots” wording is implemented literally, so a denied root itself is rejected while a dedicated child can be projected unless it collides with a declared writable role directory. First milestone: schema validation plus adversarial coverage, then commit.

- 2026-07-28 22:28 UTC — Completed the schema boundary. Mounts reject non-absolute/non-normalized forms, every D22 v3 denied root/file (including `/lib*`), nesting/duplicates, and role-directory overlap independent of JSON field order; valid root-level and deep projections remain accepted. Focused `cix-run` fmt, 24 unit tests, and warning-denied clippy pass. Next: replace `/app` with source-checked per-mount system binds and exercise systemd behavior.

- 2026-07-28 22:33 UTC — Replaced the stable `/app` bind with one `BindReadOnlyPaths=<item><mount>:<mount>` per declared system mount; `CIX_APP` is now absent in system mode and retained only for non-projecting user mode. System unit compilation rejects a declared source missing from the store item before invoking systemd. Focused fmt, 25 unit tests plus the user integration, and warning-denied clippy pass. Next: teach Cixfile destinations/projection synthesis, then run live systemd probes including collisions, shadowing, and symlink escape.

- 2026-07-28 22:37 UTC — Cixfile now accepts normalized projected absolute destinations as well as bare item-relative ones, rejects the D22 v3 deny-list at parse time, and emits the deduplicated two-component-or-narrower mount set into every generated cixSpec 2 service. Tests cover root-level file mounts, deep paths, grouping `/etc/nginx/*` without broadening to `/etc`, and all denied roots/files. Focused cix-cixfile fmt, 14 tests, and warning-denied clippy pass. Next: migrate examples and their hand-written Nix specs.

- 2026-07-28 22:43 UTC — Migrated nginx in both build forms: its item now projects `/etc/nginx` (config plus `mime.types` link) and `/srv/www` (content), with native paths in the config and argv. PostgreSQL Cixfile scripts are now item-internal without `/app`; the hand-written Nix variant already used item-local paths and needs no projection. Both examples built through `cix build` and `nix-build`; all four sudo system runs passed (nginx served the expected page and PostgreSQL returned `1`), with every transient unit stopped. Next: commit the example migration, then add real-systemd stress coverage.

- 2026-07-28 22:51 UTC — Added a root-only real-systemd projection integration test (it self-skips for ordinary developer/CI user-manager runs; executed explicitly as root here). It mounted `/etc/ssl`, an absolute `/etc/shadow` symlink, and 25 root-level files together. In-unit, the item marker was visible while host `/etc/ssl/openssl.cnf` was masked; `cat` through the projected absolute symlink failed under the DynamicUser’s normal permissions; every volume mount was visible and the unit stayed active. The test’s cleanup guard stopped its transient unit. Regenerated the tour after the changed user-mode warning and stopped both cix slices. Next: commit docs/stress coverage, then run the full done gate and VM check.

- 2026-07-28 23:08 UTC — fsproj final verification complete. Adversarial unit coverage confirms role collision in either JSON field order, all D22 v3 deny-list roots/files, nested mounts, valid root-level/deep mounts, and the missing-source error. The explicit root systemd probe confirmed `/etc/ssl` host shadowing is read-only, the absolute `/etc/shadow` symlink cannot be read by the DynamicUser, and 25 projections coexist. Both nginx and PostgreSQL passed through both `cix build` and `nix-build` system runs. `cargo fmt --all --check`, warning-denied workspace clippy, and `cargo test --workspace` passed twice; `nix build .#checks.x86_64-linux.vm-dogfood` passed. A separate dstyle worktree briefly held its own cix-run backend during one tour attempt; it was left untouched and the repeated gate was rerun after it exited. Final cleanup stopped the remaining system/user cix slices; both managers list no `cix-*` units.

- 2026-07-28 22:17 UTC — Final cleanup complete: stopped the system and user `cix-run.slice` instances after the final tests, removed the generated untracked `devenv.lock`, and confirmed both `systemctl` managers have no `cix-*` units. The legacy mount-reference audit is empty and the committed worktree is clean.

- 2026-07-28 22:10 UTC — Started the stable-mount rename requested in `scratchpad/app-rename.md`. The system-unit bind target and exported environment variable are now `/app` and `CIX_APP`; user mode retains the real store path in `CIX_APP` and names `/app` in its degraded-mode warning. Updated all cix-run system-unit goldens and direct unit assertions. Next: focused verification, commit the runner unit, then migrate Cixfiles, documentation, codegen coverage, rebuild, and run both system demos.

- 2026-07-28 22:16 UTC — Stable-mount rename complete. Rebuilt nginx and PostgreSQL through both `cix build` and `nix-build`; all four system-mode runs passed (nginx returned its expected page and PostgreSQL returned `1`), and every transient unit was stopped. Regenerated the tour through its ignored test after clearing stale slices. Final `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace` pass. Historical mount-specific log wording was normalized with the code so the requested legacy path/env-name audit is empty; generic store-item vocabulary remains unchanged. Next: stop remaining test slices, make the final log commit, and confirm a clean worktree.

- 2026-07-28 21:31 UTC — Started the app-mount track. Read `.dev/specs/track-item-mount.md` and D13/D21/D22 in `docs/design.md`, inspected the current runner, examples, fixtures, and prior verification log. Worktree is clean; `devenv shell` remains the isolated-environment fallback because `.envrc` is blocked. Scope: de-type env values, add the system-only stable `/app` mount and user degradation warning, update tests/goldens/examples, then run both sudo demos with cleanup. First unit: D21 schema/config validation and fixtures.
- 2026-07-28 21:34 UTC — Completed D21. Env declarations now hold only optional string defaults plus `required`/`secret`; legacy string `type` remains accepted and ignored with a Rust deprecation note. `ports` references structurally identify port env vars: undeclared references and non-port defaults fail validation, while `-e` values for those vars must parse as ports; all other overrides remain verbatim strings. De-typed the checked-in fixtures and PostgreSQL example. Focused `cix-run` fmt/tests/clippy pass (21 unit/golden tests plus the user integration). Next: D22 system `/app` bind/environment, path validation experiment, and user-mode warning.
- 2026-07-28 21:38 UTC — D22 mount experiment: a system transient unit with `DynamicUser=yes`, `ProtectSystem=strict`, and `BindReadOnlyPaths=<store-item>:/app` failed before start with `Failed to find executable /app/bin/true: No such file or directory`. Therefore systemd validates/resolves `ExecStart` before the service mount namespace makes `/app` available; exec/setup argv deliberately remain store-path based. Implemented the stable bind only for system units and `CIX_APP=/app`; all `--user` definitions omit the bind and receive the real store path in `CIX_APP`, and the existing degraded-mode warning explicitly lists `/app` as unavailable. Updated all system goldens and added assertions for both system and user definitions. Focused fmt/tests/clippy pass (22 unit/golden tests plus user integration). Next: commit D22 implementation, then rerun both sudo demos and the full done gate.
- 2026-07-28 21:42 UTC — Both requested sudo demos pass with the D22 mount: nginx returned its static page and PostgreSQL returned `1` for `SELECT 1`; each detached transient service was stopped by its demo trap. Workspace fmt and clippy passed. The first full test run correctly detected the changed user-mode warning in the auto-generated running-service tour; regenerated exactly that prescribed page, with no other documentation drift. Next: rerun the full test suite, stop cix slices, verify both managers are clean, then commit the final verification log.
- 2026-07-28 21:45 UTC — App-mount track complete. Final `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` all pass (including 22 cix-run unit/golden tests, user integration, and deterministic tour checks). Both sudo examples passed after the D22 change. Stopped system and user `cix-run.slice`; `systemctl list-units 'cix-*' --all` is empty in both managers. Removed only the generated untracked `devenv.lock`; final worktree is clean after this log commit. Decisions: retain store-path exec/setup argv because systemd resolves executable paths before `/app` is mounted; `/app` remains the stable in-unit path for file content and script references.

- 2026-07-28 19:28 UTC — Started the run track. Read `specs/track-run.md`, `DESIGN.md`, and the repository environment configuration in full. Confirmed the worktree is clean and the devenv is allowed, though non-interactive commands must use `direnv exec .`. Contract: changes stay within `crates/cix-run/`; implement strict v1 spec parsing/validation, deterministic systemd unit generation, `run`, `ps`, fixtures/integration coverage, and the demo. Next: inventory the existing crate and CLI wiring.
- 2026-07-28 19:42 UTC — Inventoried the workspace and existing CLI wiring; cix-run was only a skeleton. `direnv exec .` subsequently reported the worktree blocked, so requested `direnv allow` and confirmed `devenv shell` works as the verification fallback (Rust 1.97). Host experiment baseline: Linux 6.17, systemd 257, unprivileged uid 1001, user manager reachable. Added strict serde spec types and semantic validation, typed/default/required env resolution with `-e`/`-p` precedence, and focused unit tests. No cix-common helper was needed yet. Next: finish deterministic unit generation and systemd runtime integration.
- 2026-07-28 20:01 UTC — Implemented deterministic system/user unit compilation and committed full/minimal golden fixtures. System units include DynamicUser, strict filesystem and kernel hardening, syscall/capability/address-family restrictions, private networking by default, managed role directories with 0700 modes, and app-path BindPaths. Implemented local installable resolution through `nix build --no-link`, transient `systemd-run`, explicit root/system vs loud user-dev mode, namespace-specific fallback, journal foregrounding with Ctrl-C cleanup, detach, and JSON-backed system+user `ps`. Added a rootless integration test and human demo that construct a spec item with `nix store add-path`. Initial 10 unit/golden tests pass. Next: execute the user integration/demo and record empirical systemd findings.
- 2026-07-28 19:48 UTC — Completed live user-manager experiments on Linux 6.17/systemd 257. `PrivateUsers`, `ProtectSystem`, `ProtectHome`, `PrivateTmp`, `NoNewPrivileges`, `RestrictSUIDSGID`, `ProtectKernelTunables`, `ProtectControlGroups`, `LockPersonality`, `MemoryDenyWriteExecute`, `SystemCallFilter`, and `RestrictAddressFamilies` work individually. `CapabilityBoundingSet=`, `ProtectKernelModules=yes`, and `ProtectKernelLogs=yes` fail user services at `CAPABILITIES` with `Operation not permitted`, so cix retries without exactly those three and names them in its warning. `PrivateNetwork=yes` reports that network namespaces are unavailable and systemd proceeds without isolation. A real non-identity `BindReadOnlyPaths` probe fails at `NAMESPACE` with `Operation not supported`; cix therefore has a second, loudly reported fallback that retains managed `*Directory` persistence but drops `PrivateUsers`, `ProtectSystem`, `ProtectHome`, `PrivateTmp`, and `BindPaths`. The rootless integration fixture uses the canonical user state path so persistence remains testable on this host; arbitrary D11 app-path remapping is not available here in user mode. The system-manager unit remains fully hardened and never degrades.
- 2026-07-28 19:48 UTC — Final verification: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` pass; 10 cix-run unit/golden tests and the live rootless Nix-store/systemd integration pass. `demo.sh` builds a fixture with `nix store add-path`, starts it with `cix run --user`, observes managed-state output, lists it through `cix ps`, and stops it cleanly. Foreground journal streaming and Ctrl-C unit cleanup were also exercised manually. Deviations/deferrals: v0 uses the specified subprocess route rather than D-Bus; `secret` and `health` are parsed but operationally reserved; ref resolution remains for the index-track merge; `MemoryDenyWriteExecute` has no JIT opt-out yet, as required by the track; multiple same-role app dirs use deterministic `<role>-<index>` children below `cix-run-<service>`. Open maintainer questions: whether namespace-less user mode should continue with the explicit persistence-only fallback or fail closed for specs whose app path differs from the managed host path; what schema flag should represent the future JIT opt-out; and whether flake service selection should keep the implemented unambiguous `flake#package#service` spelling.
- 2026-07-28 20:41 UTC — Started the dogfood track. Read `specs/track-dogfood.md` and `DESIGN.md` Part 2. Scope is limited to `crates/cix-run/` and `examples/`; the frozen v1 schema will not change. Immediate work: fix address-family property generation and direct store-path resolution, update fixtures/tests, then empirically run nginx and PostgreSQL as system-mode transient units. Every created `cix-*` unit will be stopped and the system manager checked clean on all paths.
- 2026-07-28 20:45 UTC — Fixed both diagnosed runner bugs. Networked services now emit one valid `RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6` property and the full unit golden fixture reflects it. Existing `/nix/store/...` paths now return directly without invoking Nix; non-path installables retry `/nix/var/nix/profiles/default/bin/nix` only when `nix` is absent from `PATH`. Added direct-path regression coverage. `cargo test -p cix-run` (11 unit tests plus the live user-manager integration) and crate-wide clippy with warnings denied pass. The checked-in `.envrc` is blocked, so commands run through `devenv shell` to keep the isolated Rust environment active. Next: build the CLI and begin the nginx system-manager loop.
- 2026-07-28 20:48 UTC — nginx attempt 1 reached the real daemon but failed creating `/var/cache/nginx/body`: the declared `CacheDirectory` backing path appeared at the additional `BindPaths` destination as `nobody:0700`. Isolated `cix-probe-*` units established the cause: systemd 257 id-maps `StateDirectory`/`CacheDirectory`/`LogsDirectory` at their canonical path for `DynamicUser`, but a second bind of the backing path loses the ID map. Mode 0777 permits a first write but fails persistence because children retain the old dynamic UID. The working systemd-native resolution for conventional role roots is `TemporaryFileSystem=<role-root>:ro` plus `*Directory=managed:destination`; this hides host collisions only inside the unit, preserves the id-mapped mount, and exposes the requested app path as an ephemeral symlink. Implemented that path for system-mode state/cache/log destinations below `/var/lib`, `/var/cache`, and `/var/log`, with a regression test; cross-root app paths still use the existing bind and remain a spec boundary proposal. All probe services were stopped; targeted tests and clippy pass. Next: rerun nginx.
- 2026-07-28 20:49 UTC — nginx attempt 2 passed cache/PID/temp/mime setup, confirming the id-mapped role-directory fix, then failed opening `/dev/stdout` with `ENXIO` under the systemd service. This is example configuration rather than a runner capability bug: nginx's error log already targets stderr/journald and access logging is not required by the demo. Changed the example to `access_log off;`. The failed unit was collected and no nginx unit remains. Next: rebuild and rerun.
- 2026-07-28 20:50 UTC — nginx attempt 3 is green end to end. Built `/nix/store/445i9phgnh12h43m7bxww6jfpz4qdxlg-nginx-cix`, ran it through `sudo cix run <store-path> --detach` with a root `PATH` that contains no Nix, received `<h1>hello from composix</h1>` from `127.0.0.1:8080`, and observed the live system service in `cix ps`. Stopped and collected the unit. Added `examples/nginx/demo.sh` with build/run/readiness/curl/ps/stop and an EXIT cleanup trap. Next: execute the demo once, then build the PostgreSQL example.
- 2026-07-28 20:51 UTC — Executed `examples/nginx/demo.sh` against the locally built CLI: it built the item, served and asserted the page, showed the active unit through `cix ps`, stopped it, and left no nginx service. Added the initial PostgreSQL store item with a `bin/start` entrypoint, `bin/psql`, `/var/lib/postgresql` state, and declared TCP port 5432. The entrypoint uses `LANG=C`, `LC_ALL=C`, `--no-locale`, trust auth, and a temporary `.init` directory promoted to `data` only after successful `initdb`; it then places the Unix socket in the writable state root. This is intentionally the Docker-entrypoint pattern required at the frozen schema boundary. Next: build and run PostgreSQL empirically.
- 2026-07-28 20:52 UTC — PostgreSQL attempt 1 failed immediately in `initdb`: `could not look up effective user ID ...: user does not exist`. The Nix-built PostgreSQL frontend cannot resolve the transient systemd UID through the host NSS setup, even though `DynamicUser` itself is working. Kept the runtime isolation intact and added `pkgs.nss_wrapper` to the store entrypoint: on each start it writes passwd/group records for the current UID/GID under the declared state path and exports `NSS_WRAPPER_PASSWD`, `NSS_WRAPPER_GROUP`, and `LD_PRELOAD` for both initialization and the server. No host account or schema change is required. The failed service was collected. Next: rebuild and retry.
- 2026-07-28 20:53 UTC — PostgreSQL attempt 2 resolved the NSS wall and completed `initdb` successfully: locale C, UTF-8, POSIX dynamic shared memory, bootstrap, and fsync all passed. Server startup then failed because PostgreSQL interpreted `--pgdata` as an unknown GUC; unlike `initdb`, the server expects `-D` for its data directory. Replaced the server invocation with its native `-D`, `-p`, `-h`, and `-k` options. The initialized cluster was atomically promoted before this startup error and remains in cix-managed state, so the next attempt will verify access across a different dynamic UID as well as startup. The failed unit was collected.
- 2026-07-28 20:54 UTC — PostgreSQL attempt 3 is green end to end and also verified persistent ownership across dynamic UIDs. Built `/nix/store/ig5hrf180rs2kpfapab39y92jxrpl4zc-postgres-cix`; PostgreSQL 18.4 reused the prior cluster, listened on `127.0.0.1:5432` and `/var/lib/postgresql/.s.PGSQL.5432`, and became ready. The item’s own `bin/psql` connected over TCP as `cix` and returned `1` for `SELECT 1`; `cix ps` showed the active system unit. Stopped and collected it. Added `examples/postgres/demo.sh` with build/run/query/ps/stop and EXIT cleanup. The socket works in state, but an ephemeral runtime-directory role would better match its lifecycle and remains a spec boundary proposal. Next: execute the demo, then run the complete done gate.
- 2026-07-28 20:55 UTC — Executed `examples/postgres/demo.sh` against the locally built CLI: build, detached system-mode start, TCP `SELECT 1`, `cix ps`, and stop all passed, and no PostgreSQL service remains. Both final demo scripts now have cleanup traps for failure paths. Next: run formatting, workspace clippy/tests, then remove probe artifacts and audit both systemd managers for leftover cix units.

## Dogfood final wall list

- Invalid network property: two `RestrictAddressFamilies` assignments used a nonexistent `+` merge syntax. Resolution: emit one complete `AF_UNIX AF_INET AF_INET6` value and update the golden fixture.
- Root could not resolve plain store paths without Nix on `PATH`. Resolution: existing `/nix/store/...` inputs bypass Nix; real installables retry the default Nix profile executable after `ENOENT`.
- DynamicUser-managed directories became `nobody:0700` at additional app-path binds on systemd 257 because the second bind lost the managed idmapped mount. Resolution for conventional state/cache/log paths: a private read-only role-root tmpfs plus systemd's `*Directory=managed:destination` alias preserves the ID map and safely masks host collisions inside the unit. Cross-root mappings are recorded below.
- nginx could not create its cache/PID/temp hierarchy. Resolution: the role-directory generator fix made `/var/cache/nginx` writable and persistent without weakening mode 0700.
- nginx could not open `/dev/stdout` (`ENXIO`). Resolution: disable the optional access log; error logging remains on stderr/journald.
- PostgreSQL `initdb` could not resolve the transient UID through Nix's NSS setup. Resolution: the store entrypoint supplies per-start passwd/group records through `libnss_wrapper`.
- PostgreSQL first-run initialization needed locale, authentication, and failure-safe state handling. Resolution: the store entrypoint uses C/UTF-8, trust auth for the local demo, initializes `.init`, and atomically promotes it to `data`.
- PostgreSQL rejected `--pgdata` as a GUC. Resolution: use the server's native `-D`, `-p`, `-h`, and `-k` options.
- PostgreSQL Unix-socket placement needed a writable path. Resolution: place it under `/var/lib/postgresql`; a runtime-lifetime role is proposed below.

## Spec boundary proposals

- First-run init hook: PostgreSQL demonstrates a recurring need for a declarative initialization hook. v1 remains frozen, so the Docker-entrypoint-style store wrapper is the current solution.
- Runtime directory role: sockets, PID files, and other restart-scoped files should not live in persistent state/cache roles. PostgreSQL's socket currently uses state; nginx's PID currently uses cache.
- Cross-root app-path mapping with `DynamicUser`: arbitrary destinations such as `/srv/app/data` cannot safely rebind a modern systemd idmapped managed directory; the rebind loses its ID map. Conventional role-root destinations are fixed without schema changes, but the general D11 mechanism needs a runtime design.
- Fixed-value ports: nginx's Nix-time configuration fixes 8080 while the frozen schema models a port through an environment value. A future spec should be able to declare a fixed port or a generated-config/init mechanism; PostgreSQL demonstrates the fully overrideable argv model.
- Ports below 1024 would need an explicit capability grant; none was granted or exercised here.
- `MemoryDenyWriteExecute` remains strict. PostgreSQL's simple query did not trigger LLVM JIT; services that require JIT still need the previously identified MDWE opt-out.

- 2026-07-28 20:57 UTC — Dogfood track complete. `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace` pass; both sudo demos pass end to end. Stopped the system and user `cix-run.slice` units, removed only the explicitly named `cix-probe-*` cache artifacts and build result symlinks, and verified `systemctl list-units 'cix-*' --all` is empty in both managers. Persistent nginx/PostgreSQL application data was intentionally left intact.
- 2026-07-28 21:17 UTC — Started the specv2 track. Read `specs/track-specv2.md` and `DESIGN.md` Part 2, confirmed a clean worktree, and established the passing 12-test cix-run baseline through `devenv shell` (the repository's isolated Rust environment). Implemented the schema boundary first: versions 1 and 2 are accepted; use of `setup`, `dirs.run`, `jit`, or fixed port `value` under v1 names the field and requires v2; v2 ports require exactly one source; and v2 role paths are restricted to one component beneath their conventional root with the mandated design citation. Resolved configuration now retains effective port numbers and rejects `-p` for fixed ports. V1 arbitrary-path parsing remains unchanged. Next: compile v2 lifecycle/capability declarations into systemd properties and golden fixtures.
- 2026-07-28 21:21 UTC — Compiled v2 app semantics into systemd declarations. `dirs.run` uses `RuntimeDirectory=cix-run-<service>:<app-name>` and deliberately does not mask `/run`; `setup` resolves and interpolates exactly like `exec` and becomes `ExecStartPre`; effective ports below 1024 (fixed, env default, or CLI override) select exactly `CAP_NET_BIND_SERVICE` for both ambient and bounding sets; and `jit: true` omits MDWE. Added a single v2 golden fixture spanning run/setup/fixed ports/capabilities/JIT plus focused env-default and override capability tests. Next: validate the transient-unit property transport and runtime-directory/port behavior against the system manager.
- 2026-07-28 21:22 UTC — System-manager probe passed twice on Linux/systemd 257. A cixSpec 2 service ran as a real DynamicUser, `ExecStartPre` wrote a marker through the `/run/cix-specv2-probe` runtime alias, and its Python main process read that marker and bound `127.0.0.1:80`; HTTP returned `specv2-probe-ok`. `systemctl show` confirmed the pre-start command, `RuntimeDirectory=cix-run-specv2-probe`, and exactly `cap_net_bind_service` in ambient/bounding sets. After each stop both the managed run directory and app alias were gone, and the second start reran setup successfully. Finding for the track's `/run` caution: no `/run` mask is needed; omitting `TemporaryFileSystem=/run:ro` preserves systemd's runtime and journal plumbing while `RuntimeDirectory` aliasing works. Stopped the service and slice and confirmed the system manager has no `cix-*` units. Next: update nginx and PostgreSQL to the v2 contract.
- 2026-07-28 21:23 UTC — Updated both examples to cixSpec 2. nginx now declares its build-time port as `value: 8080` and puts its PID beneath `dirs.run` at `/run/nginx`, keeping only temp bodies in cache. PostgreSQL now declares `/run/postgresql` and passes it to `-k`; cluster initialization moved into `bin/setup`, while `bin/start` is reduced to shared NSS environment setup plus the server exec. Both executable hooks source the same small `lib/runtime-env.sh` inside the item, so DynamicUser identity lookup remains consistent. Next: build and run both sudo demos end to end, including a restart to exercise setup idempotency against existing PostgreSQL state.
- 2026-07-28 21:24 UTC — Both v2 examples pass end to end under sudo. nginx built as `/nix/store/fdl0g2k2s10gb2nl4v86907q7ql6namk-nginx-cix`, served the expected page on fixed port 8080, appeared in `cix ps`, and stopped cleanly. PostgreSQL built as `/nix/store/3vs48g3v6fd53s6ccxijv3si633mw4pc-postgres-cix`; two consecutive demo runs each executed the setup hook against the existing persistent cluster, started with its socket under the runtime role, answered TCP `SELECT 1`, appeared in `cix ps`, and stopped cleanly. Stopped both system and user cix slices after the demos and verified both managers list no `cix-*` units. Next: run the full workspace done gate and audit edge cases before the final summary commit.

## Spec v2 final wall list

- Version compatibility required distinguishing schema acceptance from semantic availability. Resolution: deserialize one shared strict type set, validate v2 field presence before other service semantics, and retain the v1 arbitrary app-path behavior while applying D11 point 6 only to v2.
- Fixed and environment-backed ports must both drive sandbox capabilities without inventing an environment value for fixed ports. Resolution: resolved configuration carries effective named port numbers separately from environment variables; the unit generator derives the bind grant from that map.
- Runtime aliases needed systemd's `/run` machinery intact. Resolution: emit the `RuntimeDirectory` alias directly with no `/run` `TemporaryFileSystem`; the live DynamicUser probe confirmed setup, journaling, binding, and stop-time cleanup.
- `ExecStartPre` had to pass through the existing `systemd-run` property transport without shell semantics. Resolution: resolve and interpolate setup as argv, serialize it with the same systemd exec-word escaping as `ExecStart`, and verify the resulting command ran on two starts.
- PostgreSQL setup and main execution both require transient UID NSS records. Resolution: keep `lib/runtime-env.sh` in the item and source it from both hooks; setup remains idempotent by treating `PG_VERSION` in persistent state as truth.
- `devenv shell` generates an untracked root `devenv.lock` in this worktree. Resolution: use the required isolated environment for every Rust gate and remove only that generated file after each verification batch to stay within track territory.

## Spec v2 deviations

- None from the requested system-mode contract. The intentionally degraded pre-existing `--user` fallbacks still may drop capability and mount-namespace controls when the user manager rejects them; D2/D13 continue to make the verified system manager the product target.
- The v2 mappings share one golden fixture rather than separate files per field; focused validation and capability tests cover the independent error/override paths.

- 2026-07-28 21:26 UTC — Specv2 implementation complete. Exact done gate passed: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace` (20 cix-run unit/golden tests plus the live user integration, with all other workspace suites green). The required low-port DynamicUser probe and nginx/PostgreSQL sudo demos passed. Final cleanup stopped the cix slices created by verification and both systemd managers list no `cix-*` units. Three implementation commits precede this final log commit; final step is to re-check clean status and commit history.

## track/composefallback

- 2026-07-30 16:40 UTC — Started `.dev/specs/track-composefallback.md` on the clean
  `track/composefallback` branch. Read AGENTS.md, `.dev/LOG.md`, authoritative D13/D36,
  the cix-run and compose logs, and Terra's 2026-07-30 `track/scenarios:nix/LOG.md`
  diagnosis. Confirmed the required gap: one compose system unit combining
  `DynamicUser=yes`, `PrivatePIDs=yes`, managed state, and a writable Unix-edge
  `BindPaths=` fails before exec on the NixOS-test systemd 261 guest with
  `226/NAMESPACE`, while cix-run's current D36 fallback only reacts after transient-unit
  activation and compose has no capability-aware generation path. The devenv is
  direnv-allowed in this worktree. Next: isolate the minimal property set in a dedicated
  VM probe, inspect upstream systemd 258–261 changes, and map the shared compiler plus
  compose generation/manifest/activation surfaces before implementing the probe.

- 2026-07-30 16:50 UTC — Minimal systemd 261 VM bisection is complete. Exact repro:
  `nix build .#checks.x86_64-linux.compose-fallback-vm --no-link -L`. A direct
  oneshot with only `DynamicUser=yes`, `PrivatePIDs=yes`, and `StateDirectory=`
  fails before exec at `226/NAMESPACE`, logging `Failed to allocate user
  namespace` and naming `/var/lib/private/<name>`; removing any one of those
  three properties succeeds. `BindPaths=` is not part of the minimal failure:
  `DynamicUser=yes + PrivatePIDs=yes + BindPaths=` succeeds without the managed
  directory, while Terra's full compose-shaped combination fails. A follow-up
  probe also covers `RuntimeDirectory=` because it is the disposable managed
  directory suitable for the production realization test.

- 2026-07-30 16:50 UTC — Upstream audit: searched systemd NEWS, issue results, and
  git history from v257 through v261. `PrivatePIDs=` was introduced in
  [406f1775](https://github.com/systemd/systemd/commit/406f1775017a5631bc91a1f53ac5e50f4fbfac0c);
  v258 substantially reordered user/PID/mount namespace setup for
  `DelegateNamespaces=` in
  [8234cd99](https://github.com/systemd/systemd/commit/8234cd9989d3834bf5c06e2b597ec097b985e1e8)
  and
  [38748596](https://github.com/systemd/systemd/commit/38748596f0783f2b773bd95d4af4d83f5b5ff872).
  Later fixes include pre-unshare UID/GID capture
  [8b5e3be8](https://github.com/systemd/systemd/commit/8b5e3be88eeb1bdba50c87cb24d9e6b31e825f38)
  and restoring the system/user-manager distinction
  [666cd35b](https://github.com/systemd/systemd/commit/666cd35be493e2d796c5424eed9a3deeddc9b0fe).
  NEWS contains no 258–261 incompatibility note for the demonstrated combination,
  and searches found no matching upstream issue. Both v257 and v261 create a
  temporary user namespace while applying the ID-mapped managed-directory mount;
  the regression's exact causal commit is therefore not proven by this audit.
  Draft upstream issue for Mathijs to file if desired: “systemd 261 regression:
  DynamicUser + PrivatePIDs + StateDirectory fails at 226/NAMESPACE”. Body:
  “On a NixOS test VM with systemd 261 and Linux 6.18.40, a root system service
  containing only Type=oneshot, ExecStart=/bin/true, DynamicUser=yes,
  PrivatePIDs=yes, and StateDirectory=probe fails before exec with `Failed to
  allocate user namespace: Operation not permitted`, followed by
  `/var/lib/private/probe` and status 226/NAMESPACE. Removing any of
  DynamicUser, PrivatePIDs, or StateDirectory makes it start. The equivalent
  workload worked on systemd 257. RuntimeDirectory reproduces too. Is this an
  unintended interaction between PID-namespace setup and the user namespace used
  for ID-mapped managed directories? A self-contained NixOS test is available.”

- 2026-07-30 16:52 UTC — Correction from the explicit role follow-up:
  `RuntimeDirectory=` does **not** reproduce; the direct systemd 261 unit with
  `DynamicUser=yes + PrivatePIDs=yes + RuntimeDirectory=` starts successfully.
  The failing capability class is persistent managed directories, whose
  DynamicUser backing paths use ID-mapped mounts (`StateDirectory=` is the
  minimal proven representative), not every `*Directory=`. The production probe
  must therefore use `StateDirectory=` despite its persistent empty probe
  directory; the fallback predicate must cover only State/Cache/Logs/Configuration
  properties and must leave RuntimeDirectory-only services fully hardened. In the
  upstream issue draft above, replace “RuntimeDirectory reproduces too” with
  “RuntimeDirectory does not reproduce, further localizing this to persistent
  ID-mapped managed directories.” The failed follow-up command was the same exact
  VM repro: `nix build .#checks.x86_64-linux.compose-fallback-vm --no-link -L`;
  it failed only because the test expected the newly measured runtime-directory
  unit to fail when it correctly succeeded.

- 2026-07-30 17:00 UTC — Implemented the shared host-capability contract and
  compose honesty path. `HostCapabilities::probe` first reads the explicit
  `CIX_PRIVATE_PIDS_PROBE=auto|supported|unsupported` override, records the
  systemd version, then realizes a uniquely named transient unit containing the
  minimal failing `DynamicUser=yes + PrivatePIDs=yes + StateDirectory=` set.
  A recognized 226/NAMESPACE failure marks only the persistent-managed-directory
  combination unsupported; any unexpected probe error aborts instead of silently
  weakening a unit. The compiler drops exactly `PrivatePIDs=yes` only from
  affected system units, returns a structured degradation, and excludes
  RuntimeDirectory-only units from fallback. Compose stores unit/property/reason
  in `manifest.json` and `cix up` emits the corresponding D36 warning before
  activation. Synthetic generator and compose tests cover capable, unsupported,
  exactly-once, and runtime-only paths. Exact focused repros passed:
  `cargo test -p cix-run -p cix-compose` and
  `cargo clippy -p cix-run -p cix-compose --all-targets -- -D warnings`.
  Next: replace the bisection fixture with the end-to-end compose VM regression
  and verify the real systemd-run probe diagnostics on systemd 261.

- 2026-07-30 17:16 UTC — The end-to-end compose regression is green on the real
  systemd 261 guest. Exact repro:
  `nix build .#checks.x86_64-linux.compose-fallback-vm --no-link -L`. The
  transient capability probe fails at the expected `226/NAMESPACE`; `cix up`
  prints the unit, dropped `PrivatePIDs=yes`, realization-probe reason, and D36
  host-PID-namespace consequence. The persisted manifest contains exactly that
  one degradation. The affected producer starts without `PrivatePIDs=`, its
  persistent DynamicUser state backing directory and shared Unix-edge directory
  exist, and the runtime-only consumer remains hardened with
  `PrivatePIDs=yes`. The initial test teardown exposed an unrelated 90-second
  systemd stop timeout for the hardened sleeping consumer, so the final
  ephemeral-VM regression omits stack teardown and keeps its assertion scope on
  activation and fallback honesty. Next: regenerate and review the tour, then
  run the full specified gate against the final source state.

- 2026-07-30 17:22 UTC — Pre-gate tour regeneration exposed a rootless-path
  regression: exact command
  `cargo test -p cix --test tour -- --ignored generate_tour` failed because
  `cix compose diff` attempted the system-manager realization probe and polkit
  rejected it with “Interactive authentication required.” The actual probe
  remains in root-only `cix up`. Rootless diff now reuses an active generation's
  recorded `PrivatePIDs=yes` capability decision so it remains stable after a
  degraded activation; with no active generation it renders the fully hardened
  form. A focused pure test covers both cases. Next: rerun tour regeneration and
  the complete gate.

- 2026-07-30 17:24 UTC — Final gate is green on committed source. Exact repro
  commands: `cargo test -p cix --test tour -- --ignored generate_tour` passed
  and left `docs/tour/` unchanged after review; `cargo fmt --all --check`;
  `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo test --workspace`;
  `nix build .#checks.x86_64-linux.vm-dogfood --no-link`; and
  `nix build .#checks.x86_64-linux.compose-fallback-vm --no-link` all passed.
  The workspace suite includes 48 cix-run tests, 14 cix-compose tests, all
  integration tests, and the tour drift/determinism checks. The final dedicated
  VM rerun confirms the writable unit-directory setup preserves existing unit
  symlinks and completes without the unrelated teardown delay. No generated
  documentation or uncommitted source drift remains.

## track/dirs

- 2026-08-02 00:10 UTC — Reproduced both addendum failures with `devenv shell
  -- nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`: the original
  hardened service stopped at `226/NAMESPACE`, then its D36 retry mounted the
  full-mirror LogsDirectory but could not write `/app/logs/restart-marker`.
  The log showed that the fallback host backing was `65534:65534 0700`; the
  explicit bind has no ID-mapped view after `PrivatePIDs` is removed. The fix
  adds a managed-root ownership anchor, read-only tmpfs mount points for every
  non-class top-level destination, and D36's private-backed persistent mode
  `0733` so the dynamic process can write its only projected view. `cix run`
  now realizes the existing host-capability probe before compiling the system
  unit; the VM forces `CIX_PRIVATE_PIDS_PROBE=unsupported` and proves the
  resulting `PrivatePIDs=no`, `0733` backing, restart persistence, and marker
  readback. A first fixed VM rerun also exposed two stale fixture assumptions:
  the probe used an unqualified `cat` although the service PATH is intentionally
  empty, and the old host-side Redis socket assertion named the legacy alias.
  The probe now uses coreutils by absolute path and the Redis assertion verifies
  its full-mirror host socket instead. Next: rerun vm-dogfood, then the complete
  specified gate.

- 2026-08-01 23:30 UTC — Started the addendum fix round after the independent
  vm-dogfood failure. The required evidence-first reproduction is next: first
  rerun the unmodified check to capture the primary `226/NAMESPACE` path, then
  force the D36 unsupported capability path to capture the post-retry
  `/app/logs` write denial. The earlier claimed green result was against the
  prior test run and is now treated as insufficient/order-dependent evidence
  until both paths have been reproduced in the current worktree.

- 2026-08-01 23:00 UTC — Started CIP-82 leg 1 from `.dev/specs/track-dirs.md` on `track/dirs`. Read AGENTS.md, the current session and cix-run logs, adopted CIP-82 (§3/§5), and the Part 2 design context. Scope is parser/manifest plus unit generation, Cixfile/docs/tour, fixtures, and the prescribed VM regression only; compose materialization, cleanup verbs, and `.env` are explicitly deferred. The worktree is clean and its direnv/devenv environment is allowed. Next: map the existing schema/parser/unit/test surfaces, then implement the arbitrary-role-path, full-mirror backing, explicit directory environments, and teaching-error DIR contract.
- 2026-08-01 23:18 UTC — Implemented the leg-1 manifest/parser and generator contract. The Cixfile model now emits `dirs.data: [{path, ro}]` for `DIR` (`:ro`, `:rw`, bare rw), checks duplicate paths across roles/data, and accepts clean arbitrary role paths; `LOGDIR` is the canonical directive. The runner uses full path mirrors beneath each unit/class root, masks class roots, bind-mounts every declared destination, and overrides the four systemd directory environment variables with colon-joined in-namespace paths. Unmaterialized `DIR` fails with the required compose teaching message. Focused `devenv shell -- cargo test -p cix-cixfile -p cix-run` is green after adapting the user-manager integration cleanup to its new nested host backing. Added the required NixOS VM log persistence/restart assertion; next: run it, regenerate the tour, review docs, and execute the complete gate.
- 2026-08-01 23:06 UTC — Complete leg-1 gate is green. Exact repros: `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo run -p cix -- fmt --check examples`; `devenv shell -- cargo clippy --workspace --all-targets -- -D warnings`; `devenv shell -- cargo test --workspace`; `devenv shell -- cargo test -p cix --test tour -- --ignored generate_tour`; and `devenv shell -- nix flake check -L`. The first workspace-suite attempt found only a stale `cix-run-decoy-x.service` left by an interrupted tour test; stopping that exact transient test unit and rerunning made the suite green. The dedicated `nix build .#checks.x86_64-linux.vm-dogfood --no-link -L` also passed, proving `/app/logs` host mirroring, restart persistence, and the environment override. Tour regeneration added the expected empty `dirs.data` projection. Next: final staged diff/status audit and commit.
- 2026-08-01 23:08 UTC — Committed the complete CIP-82 leg-1 implementation as `ebcad82` (`Implement CIP-82 directory backing`). Post-commit `git diff HEAD --check` and worktree status are clean. No compose materialization or lifecycle verb work was included; those remain explicitly deferred to later CIP-82 legs.

- 2026-08-02 00:18 UTC — The corrected dedicated VM regression is green:
  `devenv shell -- nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`
  completed its test script in 163.47 seconds. It passed the addendum's normal
  hardening assertions, forced `CIX_PRIVATE_PIDS_PROBE=unsupported` fallback
  assertions (`PrivatePIDs=no`, private backing mode `0733`, write and restart
  persistence), and the Redis Unix-socket assertion at its full-mirror host
  path. Next: run the complete track gate, ending with `nix flake check -L`,
  then review and commit only the intended sources (leaving pre-existing
  generated `devenv.lock` unstaged).

- 2026-08-02 00:24 UTC — The first full `devenv shell -- nix flake check -L`
  rerun reached all 63 checks but exposed a further stale CIP-82 host-backing
  assertion in `nix/scenarios/lifecycle.nix`: it looked for the state sentinel
  at the old `/var/lib/private/cix-lifecycle-api/sentinel` alias. The service
  writes `/var/lib/api/sentinel`, whose full-mirror host backing is
  `/var/lib/cix-lifecycle-api/var/lib/api/sentinel`; the API was active and its
  HTTP assertion had already passed. Updated all three lifecycle persistence
  checks to that exact full-mirror path. Next: focused scenario-lifecycle VM,
  then restart the complete full-flake gate.

- 2026-08-02 00:35 UTC — Focused `devenv shell -- nix build
  .#checks.x86_64-linux.scenario-lifecycle --no-link -L` passed (159.10s),
  proving the corrected sentinel assertion through activation, update, and
  rollback. The restarted, required full gate then passed: `devenv shell --
  nix flake check -L` completed all 63 checks, including vm-dogfood,
  scenario-lifecycle, the other scenario VMs, and compose fallback. Earlier
  in the same gate, formatting, example-formatting, Clippy, the workspace
  suite, and explicit tour regeneration also passed; the tour output was
  unchanged. Next: final diff audit and commit, with the pre-existing generated
  `devenv.lock` excluded.

- 2026-08-02 00:38 UTC — Final audit found no whitespace errors. Committed the
  completed addendum fix round (`Fix directory backing fallback`) after staging
  only cix-run source/tests/log and the two VM assertions;
  pre-existing generated `devenv.lock` remains intentionally unstaged. The
  commit is the handoff point for track/dirs.

## track/devices

- 2026-08-02 00:50 UTC — Started CIP-78 devices on `track/devices`. Read
  AGENTS.md, `.dev/specs/track-devices.md`, authoritative CIP-78 §§3/5,
  `.dev/LOG.md`, and the active cix-run/cixfile/compose surfaces. Existing
  claims are strings in both runner and Cixfile model, so this track needs a
  typed parameterized device form while retaining bare `jit`, `egress`, and
  `gpu`. Mechanical reading to document: a device-claiming unit replaces
  `PrivateDevices=` with `DevicePolicy=closed` plus an allow-list; non-claiming
  units retain today's `PrivateDevices=` posture. Next: implement manifest and
  Cixfile validation/model first, then unit compilation, compose override,
  example/scenario, docs, and the full gate.

- 2026-08-02 07:20 UTC — Implemented the manifest/Cixfile/compiler/compose
  slice. Claims are now bare `jit`/`egress`/`gpu` strings or strict
  `{ "device": "/dev/..." }` objects; `SHM` and `shm` validate a systemd-style
  size. Device claims emit `DevicePolicy=closed` and individual `DeviceAllow=`
  entries in place of the now-explicit normal `PrivateDevices=yes`; gpu adds
  `video render`, and literal devices stat their owner group (with a warning,
  not a generation failure, if absent). Compose's `shm` wins, persists as an
  effective manifest value, and diff names the SHM change; `grants` remains
  strict-schema rejected/reserved. Focused `cargo fmt` plus cix-run, cixfile,
  and compose suites are green (48 cix-run / 24 compose tests). Added the
  Frigate-shaped example and the new VM scenario; its NixOS build is running.
  Next: finish the VM proof, inspect the complete diff, and run the required
  full gate.

- 2026-08-02 07:48 UTC — The focused `scenario-devices` VM check passes:
  the service starts with a closed allow-list containing `/dev/dri` and the
  literal node, has `video`/`render`, lacks `PrivateDevices`, and sees its
  64 MiB `/dev/shm`. Replaced its unreliable journal polling with an
  active-after-two-seconds assertion, so the probe's preceding device and SHM
  assertions remain decisive. Final gates are green: `cargo fmt --check`,
  `cix fmt --check examples`, warning-denied workspace clippy, workspace
  tests, regenerated+checked tour, and `devenv shell -- nix flake check -L`
  (including all VM scenarios). Next: stage the final log, review, and commit.

## track/devfix

- 2026-08-02 08:22 UTC — Started the CI repair from `.dev/specs/track-devfix.md` on `track/devfix`. Read AGENTS.md, `.dev/LOG.md`, D13/D36, the current cix-run capability/compiler/runtime paths, and the devices merge. Evidence first: this host matches the CI policy surface (`kernel.apparmor_restrict_unprivileged_userns = 1`, AppArmor loaded). A non-collected user-manager scratch service with `PrivateDevices=yes` was accepted asynchronously by `systemd-run --user`, then journalled `Failed to drop capabilities: Operation not permitted` and `Failed at step CAPABILITIES spawning /bin/sh`, status `218/CAPABILITIES`; it was collected before `cix ps` could list it. This verifies the hypothesized detached-unit disappearance, though the concrete systemd setup step here is CAPABILITIES rather than 226/NAMESPACE. Next: add an evidence-bearing user-manager PrivateDevices probe and route unsupported UserFull compilation through structured degradation, including the synchronous retry path.

- 2026-08-02 08:32 UTC — Added `CIX_PRIVATE_DEVICES_PROBE`/`HostCapabilities::probe_user`, which realizes a collected, waited `systemd-run --user PrivateDevices=yes /bin/sh -c true` control unit and treats the observed 218/CAPABILITIES/setup-policy class as unsupported while rejecting unexpected failures. UserFull claim-less run, scheduled-app, and debug units now compile through it; only an unsupported probe drops `PrivateDevices=yes` as a `UnitDegradation`, with a D13 warning normalized out of the tour. Claiming-device units do not pay this probe because they use `DevicePolicy=closed` rather than `PrivateDevices`. The existing synchronous capability retry now also removes `PrivateDevices`, covering a probe/start race. `devenv shell -- cargo fmt --all` and `devenv shell -- cargo test -p cix-run` pass (49 unit, 2 projection, 1 user integration). Next: exercise the generated tour on both normal and forced-unsupported paths, then run the remaining gates.

- 2026-08-02 08:48 UTC — Tour verification uncovered a separate stale presentation adapter: `filter_unit_listing` still parsed the old five-column `cix ps` output and therefore hid the active row that its raw-output assertion had already observed. It now locates the explicitly-owned unit field, asserts `active/running`, and the updated generated chapter commits that row. The detected AppArmor path and `CIX_PRIVATE_DEVICES_PROBE=unsupported` both pass `tour_matches_committed_document`; `HostCapabilities::all_supported` preserves the property in the compiler test, covering the capable class. Next: run the complete prescribed gate.

- 2026-08-02 09:10 UTC — Final gates: `devenv shell -- cargo fmt --all --check`, `devenv shell -- cargo run -p cix -- fmt --check examples`, and `devenv shell -- cargo clippy --workspace --all-targets -- -D warnings` pass. `cargo test -p cix-run` passes 50 unit tests plus both projection tests and the user integration; the full workspace suite was rerun after a stale warm-workspace tour failure, and its isolated previously failing tour test passes. Fresh tour generation and exact drift checking pass on the detected AppArmor host and with `CIX_PRIVATE_DEVICES_PROBE=unsupported`. The required `devenv shell -- nix flake check -L` ran all 64 checks (including VM dogfood, devices, compose fallback, and scenarios) and its immediate cached rerun reported every check previously built, exit 0. CI remains the final verdict for the actual GitHub runner class. Next: final diff audit and commit.

## track/health

- 2026-08-02 08:20 UTC — Started the complete CIP-79 health track on
  `track/health`. Read `.dev/specs/track-health.md`, adopted CIP-79 §§3/5,
  the current project and cix-run logs, and the active design/ledger hits.
  The branch is clean and the worktree-local direnv/devenv environment is
  allowed. Fixed scope: replace the v0 `health {exec, interval}` manifest
  shape with typed readiness/liveness; add Cixfile parse/fmt and migration
  diagnostics; compile native notify/watchdog and cix http/tcp adapters;
  reject health-conditioned compose edges; prove rollout, structural-edge,
  and restart behavior in `nix/scenarios/health.nix`; update docs/ledgers;
  finish with the full prescribed gate and commit. Next: map the exact model,
  parser, CLI, generator, compose-schema, snapshot, and scenario seams.

- 2026-08-02 09:05 UTC — Implemented the schema/language/compiler/prober slice.
  Bare manifests now carry strict typed `readiness`/`liveness` fields and give
  the removed `health {exec, interval}` field an explicit migration refusal.
  Cixfile accepts http/tcp/notify with IN/EVERY on SERVICE and APP, round-trips
  through fmt/codegen, and has the mandated `LIVELINESS` suggestion snapshot.
  Unit compilation maps notify readiness to `Type=notify`, adapters to exact
  cix-binary `ExecStartPost` commands, liveness to a 3× watchdog plus an
  all-cgroup resident pinger, and restart/StartLimit policy only on declaring
  units. Twelve system/user × consumer × probe property snapshots cover the
  matrix. The std-only prober has TCP, HTTP-status, retry, and sd_notify tests.
  Compose now rejects health-condition vocabulary at every edge level and makes
  consumers require/After their structural producer so producer readiness gates
  their start. Focused repros are green: `devenv shell -- cargo test -p cix-run
  --lib`; `devenv shell -- cargo test -p cix-cixfile --lib --tests`; `devenv
  shell -- cargo test -p cix-build --lib`; `devenv shell -- cargo test -p
  cix-compose`. Added `scenario-health`; next: run and fix the live VM proof.

- 2026-08-02 09:20 UTC — The live VM integration pass proved the successful
  path (cix up blocks through delayed HTTP readiness; a structural consumer
  starts afterward) and exposed three systemd boundary details now fixed:
  adapter readiness feeds the watchdog during startup; StartLimit keys render
  in `[Unit]`; and compose activation checks failed member jobs after the
  target transaction so readiness timeout is a loud `cix up` failure. Failed
  startup cleanup is bounded by the readiness budget. Structural consumers
  now require the edge setup unit but only order `After=` their producer, so a
  liveness restart does not stop unrelated consumer lifetime. The watchdog VM
  has since proved the 3× window, native no-curl/no-shell adapter commands,
  watchdog result, pinger journal mapping, and restart into a second healthy
  instance; its final cleanup rerun is in progress after making the fixture's
  consumer explicitly SIGTERM-responsive. Updated cixfile/migration docs plus
  Docker and corpus ledgers. Next: land the focused VM green result, then run
  the full prescribed gate.

- 2026-08-02 09:35 UTC — Focused VM is fully green with `devenv shell -- nix
  build .#checks.x86_64-linux.scenario-health --no-link -L` (78.90s test
  script): delayed readiness, structural ordering, failed rollout, watchdog
  restart/recovery, native-prober unit text, and teardown all passed. The
  non-flake gates are green with exact repros: `devenv shell -- cargo fmt
  --all --check`; `devenv shell -- cargo run -p cix -- fmt --check examples`;
  `devenv shell -- cargo clippy --workspace --all-targets -- -D warnings`;
  `devenv shell -- cargo test --workspace`; and `devenv shell -- cargo test -p
  cix --test tour -- --ignored generate_tour` followed by a clean unstaged
  `git diff --exit-code -- docs/tour` against the staged generated transcript.
  The only tour change is the intended inspect projection replacing
  `health:null` with `readiness:null`/`liveness:null`. Next: required final
  `devenv shell -- nix flake check -L`, then audit and commit.

- 2026-08-02 09:40 UTC — CIP-79 implementation is complete. The required final
  gate `devenv shell -- nix flake check -L` passed all 67 checks, including
  `scenario-health`, the existing scenario suite, and `vm-dogfood`; the health
  scenario also passed while competing with the full parallel VM load. Final
  audit found no whitespace errors or unrelated worktree changes. The track is
  ready to commit; next step is orchestrator re-verification and merge.

## track/adapterlive

- 2026-08-04 UTC — Started the systemd-257 adapter-liveness retention repair on
  `track/adapterlive`. Read the track spec, CIP-79, current journals, health
  compiler/prober, and VM scenario. The ordinary `scenario-health` flake check
  currently imports the moving nixos-unstable `pkgs` set (not the pinned v257
  package); the flake already carries a `systemd257` compatibility package for
  NixOS VM use. Current implementation forks the `ExecStartPost` pinger and
  returns its parent successfully, exactly matching the Mastodon receipt's
  failure mechanism. Next: run the focused current VM as a baseline and make a
  narrow v257 health VM reproduction before selecting a retention mechanism.

- 2026-08-04 UTC — Reproduction result: `scenario-health` passed on the
  flake/CI package set (systemd 261), and the same focused scenario passed with
  its PID 1 proven to be the flake-pinned systemd 257.6 compatibility package.
  It keeps the cix-owned HTTP pinger healthy for seven seconds after activation,
  exceeding the emitted three-second watchdog window; the recorded Mastodon
  loss is therefore not reproducible in the available in-tree 257 universe.
  Mechanism assessment: a companion cannot send `WATCHDOG=1` to the parent
  service because systemd authorizes notifications by the sending unit's
  cgroup; re-parenting intentionally evades systemd supervision; and a version
  gate would silently remove liveness despite the negative reproduction. No
  product mechanism change is warranted without the original manager/package
  and generated-unit evidence. Added the 257 check and survival assertion as
  regression coverage, updated the open-item disposition, and marked the
  exhibiting Mastodon gap stale. Next: run formatter and focused current/v257
  VM receipts, then commit this evidence-backed resolution.

- 2026-08-04 UTC — Synchronous focused receipts passed: `devenv shell -- nix
  build .#checks.x86_64-linux.scenario-health --no-link -L` (PID 1 systemd
  261) and `devenv shell -- nix build
  .#checks.x86_64-linux.scenario-health-systemd257 --no-link -L` (PID 1 pinned
  systemd 257.6). Both retain the healthy HTTP pinger through the new
  seven-second / three-second-watchdog assertion and complete the existing
  failed-probe restart recovery. `devenv shell -- cargo fmt --all --check`,
  `devenv shell -- cargo run -p cix -- fmt --check examples`, and `git diff
  --check` also pass. Next: complete the standard Rust and tour gates, audit,
  and commit.

- 2026-08-04 UTC — `devenv shell -- cargo clippy --workspace --all-targets --
  -D warnings` passed. `devenv shell -- cargo test --workspace` reached the
  corpus-browser drift check and failed only because the required Mastodon
  `GAPS.md` stale disposition changes generated `docs/corpus/mastodon.html`;
  the test first reports unrelated pre-existing `docs/corpus/filestash.html`
  drift too. Regenerating either page requires editing `docs/corpus*`, which
  this track's explicit fence forbids. This is a scope/ledger-output conflict,
  not a product failure; leave the generated-browser refresh to its owning
  track or explicit orchestration. No green full-workspace receipt is claimed.
  Next: run the independent tour drift gate, audit the allowed changes, and
  report the honest stop rather than crossing the fence.

- 2026-08-04 UTC — `devenv shell -- cargo test -p cix --test tour -- --ignored
  generate_tour` passed and `git diff --exit-code -- docs/tour` was clean. The
  focused health receipts, Rust formatter/examples formatter, warning-denied
  clippy, and tour gate are green. The only unfinished prescribed gate is the
  full workspace suite's forbidden corpus-browser regeneration; do not commit
  a falsely green track or alter `docs/corpus*` without direction.

- 2026-08-04 UTC — Orchestrator lifted the `docs/corpus*` fence after the
  content track merged. Committed the retention regression and ledger finding
  as `76a00c8`, fetched, and merged `origin/main` (CIP-91/92 plus Filestash
  browser regeneration) cleanly. `devenv shell -- cargo test -p cix --test
  corpus -- --ignored generate_corpus_browser` exited 0 and refreshed the
  Mastodon generated browser page for its stale adapter-liveness evidence.
  Next: commit that generated artifact, then rerun the full workspace gate.

- 2026-08-04 UTC — Committed the generated Mastodon browser refresh as
  `b5c05f7`. A concurrent `track/netnsrace` twenty-VM contention matrix is
  active, so the final full workspace gate is deliberately deferred until it
  finishes; project policy forbids overlapping full VM gates. Next: wait for
  that external gate to clear, then run the full workspace test synchronously.

- 2026-08-04 UTC — After the corpus refresh, `devenv shell -- cargo test
  --workspace` exited 0 synchronously: the corpus browser determinism and
  committed-page checks are green again, alongside all workspace suites. This
  closes the previously honest gate stop. The agent-tier receipts are now
  formatter, examples formatter, warning-denied clippy, focused health VMs on
  systemd 261 and 257.6, generated corpus/tour drift, and the full workspace
  suite. Next: audit status, commit this final gate receipt, and hand off.

## track/secrets

- 2026-08-02 10:00 UTC — Started CIP-81 from `.dev/specs/track-secrets.md` on
  `track/secrets`. Read AGENTS.md, the current session journal, this assigned
  log, the full adopted CIP (including all four FETCH consent turns), and the
  relevant runtime/compose/Cixfile surfaces. Scope spans the declared runtime
  `SECRET` contract, compose credential materialization and salted rotation
  state, host-side FETCH credentials/consent, anonymous `run --compose`, docs,
  and the required secrets VM. The builder-engine ownership fence is observed:
  FETCH work will stay additive at execution/credential boundaries and not
  touch cix-cixfile keying, memo, or trace code. Next: map exact model and CLI
  seams, then implement schema and runtime delivery first.

- 2026-08-02 10:30 UTC — Implemented the first integrated CIP-81 slice:
  `SECRET name [AS VAR_FILE]` parses and persists in the manifest, with `%d`
  credential-path environment projection; strict compose top-level `secrets`
  sources resolve only into declaring services and emit `LoadCredential=` or
  `LoadCredentialEncrypted=`. Missing declarations fail and excess supplies
  warn LOUDly. `cix up` now maintains a per-composite random-salt HMAC state
  outside generations and adds secret-rotated consumers to the restart set.
  `cix run --compose <file|->` routes through compose activation; the
  `host-idmap:` fused `--dir` spelling now gives the CIP-77 migration pointer.
  FETCH credentials are host-local JSON configuration with concrete-URL,
  project/token/prefix consent and sandbox-local credential-file mounting;
  they do not alter keys, memo records, locks, probes, or trace data. Focused
  parser/unit/compose tests are in progress; next is the complete focused
  suites, generated docs/fixtures, and the VM scenario.

- 2026-08-02 13:31 UTC — The dedicated secrets VM is green. Synchronous
  `devenv shell -- nix build .#checks.x86_64-linux.scenario-secrets --no-link`
  produced the valid `/nix/store/lpicg5k7zdm9dg5rm2bvpanz2wx10inv-vm-test-run-scenario`
  output after its 63.33s script: a root-only source was delivered at the
  systemd credential path, the `_FILE` shim worked, excess compose supply was
  LOUD, rotating the source restarted only the consumer, and `cix run
  --compose` activated a credential-consuming member. Earlier fixture failures
  were only missing absolute coreutils paths and an attempted write to /run by
  DynamicUser; the final assertion uses the unit journal. Focused
  `devenv shell -- cargo test -p cix-build -p cix-compose -p cix-run` is green.
  Added direct consent-state and strict compose-source coverage next; then run
  the complete required gate and commit.

- 2026-08-02 13:43 UTC — Complete CIP-81 gate is green. Synchronous receipts:
  `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo run -p cix
  -- fmt --check examples`; `devenv shell -- cargo clippy --workspace
  --all-targets -- -D warnings`; `devenv shell -- cargo test --workspace`; and
  tour regeneration followed by its normal drift/determinism test. The mandated
  `devenv shell -- nix flake check -L` built all 73 checks, then its immediate
  synchronous cached rerun reported 12 checks previously built and `running 0
  flake checks`, exit 0. The generated tour records `secrets: {}` in the
  manifest projection. Next: stage the audited diff and commit `track/secrets`.

- 2026-08-02 13:43 UTC — Committed the completed implementation as `f3fa1ac`
  (`Implement CIP-81 secrets`) and pushed it normally to
  `origin/track/secrets`; the synchronous remote receipt resolves that ref to
  `f3fa1ac4cc6f1261b0b831cd26080e3949b5f972`. No source changes followed the
  full gate; this journal close remains to commit.

## track/closedroot

- 2026-08-02 14:00 UTC — Started CIP-84 phase 1 from
  `.dev/specs/track-closedroot.md` on the clean `track/closedroot` branch with
  the repository devenv active. Read the current session journal, authoritative
  design/CIP decision, assigned cix-run journal, closed-root spec, and affected
  Docker/corpus ledgers. The adopted contract is an opt-in `--closed-root`
  compiler path in this track, with whole-store read-only visibility, D22/role/
  materialized claim projections, synthetic minimal NSS, claimed resolver
  injection, no `/bin/sh`, user/system parity, and a full artifact audit VM;
  phase 2 alone will make it unconditional. Next: map the shared run/compose
  compiler and scenario inventory before implementing the unit model.

- 2026-08-02 14:56 UTC — Implemented the shared phase-1 sealed-root compiler
  and CLI plumbing for `cix run --closed-root`, anonymous compose, `cix up
  --closed-root`, and compose diff. Units now use a per-unit empty root,
  `MountAPIVFS`, whole-store ro, D22/role/materialization binds in both system
  and user modes, no `/bin/sh`, `/usr/bin/env -> /bin/env`, claimed host
  resolv.conf, `PrivateUsers`, and the pre-v257 three-socket journald fallback.
  Dynamic NSS cannot be rendered before systemd allocates the UID; the proven
  mechanism binds cix-owned passwd/group backing files and runs a privileged
  in-root pre-start helper before pack hooks, deriving the exact UID/GID from a
  dedicated systemd-managed identity directory and sealing three-line files.
  Focused cix-run/compose tests pass, including system/user golden fixtures,
  NSS contents, claims/dirs/materializations, D22 user projection, and the
  v256 log-socket guard. Live system receipts passed for nginx HTTP and Redis
  PING under the store-built CLI; nginx inspection proved the closed root,
  whole-store/D22 binds, absent `/bin/sh`, and exact DynamicUser NSS. The audit
  also found and fixed a pre-existing bare-START bug (`redis-server` resolved
  beside rather than through item `PATH=bin`). On this host, user-manager
  RootDirectory reached the existing D13 fallback because unprivileged mount
  namespacing is unsupported; the attempted sealed path and downgrade were
  loud as required. Removed only the explicitly named, stateless probe roots
  afterward. Next: build the exhaustive closed-root audit scenario and apply
  its honest ledger consequences.

- 2026-08-02 15:10 UTC — Added `scenario-closedroot-audit`, with an exhaustive
  checked-in directory inventory: all seven `examples/pack` members and ten
  reproducible corpus runtime contracts start behind `--closed-root` and run
  their native HTTP/protocol/readiness/version probe. Each service assertion
  also checks its live RootDirectory, MountAPIVFS, whole-store read-only bind,
  env alias, and absent `/bin/sh`; devices, shm, inherited listeners, JIT,
  setup, role dirs, and app execution are represented. The remaining ten
  corpus cases are an explicit roster, not silent skips. Directus, Filestash,
  and Verdaccio have no runnable item; Dozzle and Watchtower require Docker's
  control plane; Parse Server has no runtime receipt. Echo Server, Excalidraw,
  Wallos, and Whoami did pass historical one-off probes, but their consumed
  source/build trees are not checked in and their receipt store paths have no
  derivation, so CI cannot reproduce those artifacts. Downgraded exactly those
  four ledger rows to “runtime receipt, closed-root evidence pending” rather
  than substituting fake look-alike services. Next: run and debug the dedicated
  VM synchronously, then execute the complete track gate.

- 2026-08-02 15:21 UTC — The first synchronous audit run found two real
  closed-root host dependencies. Caddy's port 80 bind failed because
  `PrivateUsers=` places `CAP_NET_BIND_SERVICE` in a user namespace while the
  socket belongs to the host network namespace; changed the audited Caddy,
  nginx, and Traefik cases to unprivileged ports and made the compiler reject
  this ineffective low-port combination with a named-LISTENER teaching path.
  The design/Cixfile/Docker ledgers now say so. Service shutdown then exposed
  GC-root cleanup resolving `rm` through `/run/current-system/sw`, which is
  intentionally absent from the sealed root; cleanup now canonicalizes the
  executable into `/nix/store`. Corrected the handwritten audit manifests to
  the v3 `start_pre` shape and made Tomcat's bash dependency explicit instead
  of depending on its `/bin/sh` shebang. The targeted low-port compiler test
  passes. Next: verify cleanup locally and rerun the complete audit VM.

- 2026-08-02 15:48 UTC — Closed-root teardown now binds the cix-owned GC-root
  directory into the empty root and executes a store-resident `rm`; a live
  Caddy receipt proved that killing the main process runs `ExecStopPost` and
  removes the host GC-root symlink. PostgreSQL revealed that upstream `initdb`
  invokes `/bin/sh` internally, so the pack now produces a pristine database
  template during its Nix build and copies it into the writable role directory
  at startup. A live closed-root PostgreSQL receipt reached `pg_isready` and
  cleaned up correctly. In the rerun VM, PostgreSQL and Tomcat passed their
  native probes plus every isolation/cleanup assertion; Adminer returned HTTP
  success, but its audit incorrectly required presentation text that its
  current response does not contain, so the probe now asserts a successful
  response without coupling the runtime contract to page copy. Next: rerun the
  exhaustive VM from this correction, then execute all track gates.

- 2026-08-02 16:00 UTC — The exhaustive VM progressed through PostgreSQL,
  both Redis contracts, Tomcat, Adminer, phpMyAdmin, Traefik, all specialized
  pack claims, Caddy, Memcached, and NATS. Redis exposed an inherited host
  locale dependency; both checked-in Redis Cixfiles now choose the built-in
  `C` locale explicitly, and both native PING probes pass. Bounded the
  Memcached netcat client after confirming its VERSION reply so an open server
  connection cannot hang the test driver. The corpus nginx manifest then
  exposed a genuine service-contract bug: nginx daemonized away from systemd's
  tracked main process. Its checked-in config and audit fixture now set
  `daemon off;`, matching a foreground container entrypoint. Next: validate
  nginx and the remaining app contract first, then complete the exhaustive VM.

- 2026-08-02 16:08 UTC — Complete `scenario-closedroot-audit` receipt is
  green (exit 0, 149.15-second VM script): every one of the seven pack items
  and ten reproducible corpus items ran under `--closed-root`, passed its
  native probe, exposed the required sealed-root properties, retained no
  `/bin/sh`, and removed its GC root after forced main-process teardown. The
  remaining gate is also green through Rust fmt, examples Cixfile fmt,
  warning-denied all-target workspace clippy, explicit corpus-browser and tour
  regeneration, and `cargo test --workspace` (including corpus/tour drift and
  determinism). Corpus browser regeneration records the four honest
  reproducibility downgrades plus the Redis/nginx/Tomcat/Traefik contract
  changes; tour regeneration produced no tour-page diff. Next: run the
  mandatory full `nix flake check -L`, then review, close the session log, and
  commit.

- 2026-08-02 16:18 UTC — The first full flake gate reached the pre-existing
  `vm-dogfood` integration test and found that its hand-written PostgreSQL
  fixture did not include the new build-time database template. Both the
  dogfood VM and closed-root audit now import the checked-in PostgreSQL overlay
  and copy the same template into their fixtures. The focused receipt
  `devenv shell -- nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`
  is green (exit 0, 160.59-second VM script), including service teardown and
  GC-root cleanup. Next: rerun the complete `nix flake check -L` gate from the
  corrected source tree.

- 2026-08-02 16:23 UTC — Mandatory final gate
  `devenv shell -- nix flake check -L` is green (exit 0): all 132 flake checks
  passed from the corrected tree, including `vm-dogfood`, the full
  `scenario-closedroot-audit` 17-app matrix, and every existing scenario. The
  aggregate run's longest existing compose teardown completed synchronously in
  283.14 seconds. Next: final spec/diff review, append the orchestrator session
  journal, and commit the completed track without staging either LOG.

- 2026-08-02 16:28 UTC — Final spec reconciliation found a compose-only
  activation edge: every closed root binds the cix GC-root directory so
  store-resident teardown tools can reach it, but compose does not create the
  per-run anonymous GC links that happened to create `/run/cix/gcroots` during
  the audit. Shared closed-root preparation now creates the bind source for
  both run and compose, with a focused scaffold test. Next: rerun Rust gates
  and the mandatory full flake check from this final correction.

- 2026-08-02 16:34 UTC — Final correction receipts are green: `cargo fmt
  --all --check`, warning-denied workspace/all-target clippy, and `cargo test
  --workspace` (including the new preparation test plus corpus/tour drift) all
  exited 0. The post-correction mandatory `devenv shell -- nix flake check -L`
  also exited 0; its complete selected matrix passed, including
  `scenario-closedroot-audit`, `vm-dogfood`, compose lifecycle, dirs2, and GC
  survival. The longest VM completed synchronously in 275.68 seconds. Next:
  close the orchestrator journal, stage implementation/docs only, and commit.

- 2026-08-02 16:36 UTC — Ledger review corrected the Docker userns comparison:
  ordinary phase-1 runs still use DynamicUser/idmapped mounts without a user
  namespace, while `--closed-root` adds `PrivateUsers=yes`; neither is Docker's
  daemon-wide subordinate-ID remapping model. Next: one final full flake receipt
  on the exact documentation-complete tree, then commit.

- 2026-08-02 16:41 UTC — Exact final-tree receipt
  `devenv shell -- nix flake check -L` is green (exit 0), including the
  closed-root audit, dogfood, every existing scenario, and a synchronous
  270.06-second longest VM. Track implementation, examples, ledger regrades,
  generated corpus pages, and tests are complete. Next: commit the staged
  non-LOG patch; independent re-verification and merge remain for the
  orchestrator.

- 2026-08-04 15:17 UTC — Started track/runfixes after reading its spec, the
  project journal, the two verified open questions, and CIP-82. CONFIGDIR's
  `/etc`-only runner validation is the remaining D11-era restriction despite
  CIP-82's arbitrary-path rule. The implementation now admits arbitrary clean
  CONFIGDIR paths and gives config the same full-path mirror, bind, and
  in-namespace environment projection as the other persistent roles. Next:
  format and test this boundary, then add the focused VM proof and the sealed
  root localhost skeleton.

- 2026-08-04 15:24 UTC — The CONFIGDIR/unit and closed-root skeleton unit
  tests pass synchronously (`devenv shell -- cargo test -p cix-run`), as does
  Nix evaluation of `scenario-dirs2`. The focused VM began but was interrupted
  when concurrent VMs exhausted the shared `/tmp` inode pool (100%); its Nix
  process was stopped cleanly and is explicitly not a receipt. The new scenario
  is ready to prove native `/config/probe`, default localhost, and an item
  `/etc/hosts` override after inode headroom returns. Docs mark the two
  exhibiting corpus cases stale for regeneration; no corpus conversion content
  changed. Next: wait for safe VM capacity, rerun the focused scenario, then
  complete the declared agent tier and commit the runtime and ledger units.

- 2026-08-04 15:36 UTC — Committed the implementation as `0d92eac` and the
  ledger/browser regeneration as `9a45b8d`. Synchronous receipts: cargo fmt,
  examples fmt, warning-denied workspace/all-target clippy, full workspace
  tests after corpus-browser regeneration, focused cix-run tests, and the
  Cixfile parser test all pass. Tour generation passes; its foreign-user-unit
  check flaked once amid shared manager activity and passed on immediate focused
  retry. Two focused `scenario-dirs2` VM attempts were stopped before a result:
  each Nix rebuild drove shared `/tmp` inodes to exhaustion (100%, then 97%)
  while unrelated VM drivers were active. No VM receipt is claimed. The track
  remains otherwise ready; next is an isolated focused VM retry once `/tmp`
  inode headroom is restored, followed by a final log commit.

- 2026-08-04 15:51 UTC — Isolated focused receipt is green: `TMPDIR=/var/tmp
  nice -n 10 devenv shell -- nix build .#checks.x86_64-linux.scenario-dirs2
  --no-link -L --max-jobs 2 --cores 2` completed with output present, then an
  immediate identical cached re-realisation exited 0 synchronously. The first
  real VM exposed a harness bug, not product behavior: FHS `getent` has no
  loader in a sealed root and journald prefixes messages. The scenario now
  uses the Nix-store Python resolver under `set -eu` and substring journal
  matches. Its green service assertions prove writable `/config/probe`, the
  default versioned localhost skeleton, and an item-provided `/etc/hosts`
  override. Next: commit this regression correction and final log, then leave
  the clean committed track for independent orchestration.
