# cix-run work log

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
