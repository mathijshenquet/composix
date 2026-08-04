# litdoc work log

- 2026-08-04T15:35:00Z — The file-display unit is complete. `Doc::show_file`
  reads the real file, returns its raw bytes-as-text for the existing semantic
  assertions, and renders normalized content beneath a relative-path H4 label;
  store outputs are labeled relative to their item root. Cixfiles (including
  `Cixfile.dissolved`) use `dockerfile`; nix, nginx config, Python, JSON, and
  HTML extensions use their requested fences; other files use an untagged
  fence. All 17 generated `$ cat` transcripts are gone, including single-file
  built outputs and the raw-item aside; the only remaining `cat` is the RUN
  command that genuinely demonstrates concatenation. Focused synchronous
  receipts pass: fmt, the language-routing regression, explicit tour
  generation, the committed-document match, `git diff --check`, and a generated
  tour grep finding zero `$ cat` prompts. Next: commit this unit, then rebuild
  the listener through a Cixfile and polish Chapter 2.

- 2026-08-04T15:15:00Z — Started `track/tourpolish` from current main after
  reading the repository journal, design decisions, cix tour log, tour source,
  and the prior tour blueprint/voice rules. Scope is recorded in
  `.dev/specs/track-tourpolish.md`: replace all 17 generated `$ cat` file dumps
  with per-file typed blocks sourced from real files; rebuild Chapter 6's
  listener item from a canonical `LISTENER` Cixfile and checked-in Python
  probe; retain and explicitly explain Chapter 2's deliberate CIP-91
  mount-below-linked-tree materialization example; and rename its demo-site
  service. Planned commits separate the harness/file-display migration from
  chapter fixture/prose changes. Final gate is the standard agent tier plus
  three consecutive synchronous tour determinism runs.

- 2026-08-04T13:45:52Z — Final `track/tourfix2` agent tier is green with
  synchronous exit-0 receipts: `devenv shell -- cargo fmt --all --check`;
  `devenv shell -- cargo run -p cix -- fmt --check examples`; `devenv shell --
  cargo clippy --workspace --all-targets -- -D warnings`; and `devenv shell --
  cargo test --workspace -- --test-threads=1`. After staging the intended page,
  `devenv shell -- cargo test -p cix --test tour -- --ignored generate_tour`
  followed by `git diff --exit-code -- docs/tour` proved zero regeneration
  drift, and the exact `tour_matches_committed_document` test passed. The final
  three-run determinism receipt is recorded below. The workspace gate also
  passed `tour_ignores_a_foreign_user_unit`, proving the exact cleanup leaves an
  ambient unit alone. No implementation behavior, VM scenario, Docker/corpus
  ledger row, or Cixfile language changed; this tour-only track correctly stops
  at the standard agent tier. Next: final staged review and commit.

- 2026-08-04T13:45:00Z — Chapter 5 now builds a canonical observer SERVICE
  (`IMPORT ${pkgs.coreutils}`, bare `START sleep 300`) beside the existing web
  SERVICE and cleanup APP. The harness snapshots only the debug unit prefix it
  creates, stops each exact observer unit, waits for systemd collection, and
  unloads `cix-run.slice` only when no active cix unit shares it. `ps --json`
  selects the exact observer unit; `stats` selects that same unit and normalizes
  only its inherently live counters; `logs --explain` names observer. Generated
  Chapter 5 is the only page changed. Final consecutive synchronous receipts:
  the exact `generated_tour_is_deterministic` test passed three times in one
  foreground loop (46.29s, 46.31s, 45.92s). Next: standard agent tier and
  staged regeneration/no-drift proof.

- 2026-08-04T13:29:45Z — Started `track/tourfix2` from the merged
  `track/tourfix` head after main CI's exact consecutive-render assertion diff
  identified chapter 5's ambient table sizing. The bare `cix ps | head -n 1`
  pipeline formats every manager-visible cix unit before `head`, so a transient
  debug unit still awaiting systemd collection changes the next render's header
  width. Scope: synchronously tear down receipt-created units, exercise a
  canonical long-running observer sibling, select its exact `ps --json` row and
  stable `stats` identity, keep logs scoped, regenerate, then prove exact
  determinism three consecutive times plus the standard agent tier.

- 2026-08-04T12:46:00Z — Started the main-CI tour determinism repair on
  `track/tourfix` after reading the current journal/design context and the
  historical `track-tour2` blueprint. Fast-forwarded the stale track head to
  current `main`; preserved the checkout's pre-existing untracked
  `devenv.lock` in `stash@{0}` because main tracks a different lock. CI's user
  manager rejects `PrivatePIDs=`, sending cix through the known D13 whole-set
  degradation without `BindPaths`; Chapter 1's executed `nginx -t` then
  depends on whether `/var/cache/nginx` was projected. Scope remains the tour:
  retain the canonical Cixfile, replace the role-directory-dependent receipt
  with asserted manifest output and clearly labeled system-manager scenario
  prose, and do not change cix's filed degradation defect. Next: regenerate
  the tour, inspect drift, and run the focused plus full agent-tier gates.

- 2026-08-04T12:50:00Z — Removed only Chapter 1's executed `nginx -t`
  receipt; the canonical nginx Cixfile and its generated manifest assertions
  are unchanged. The replacement is explicitly labeled non-executed prose
  pointing to `nix/vm-dogfood.nix`, which runs/probes/stops nginx under the
  production manager. Added the required harness comment naming GitHub
  Actions CI's `PrivatePIDs=` rejection and resulting no-`BindPaths` D13
  retry; made Chapter 5's related prose manager-neutral. Focused synchronous
  receipts pass: `cargo fmt --all --check`; the exact degraded-normalizer
  regression; ignored tour generation; and the exact twice-rendered tour
  determinism test. Generated changes are limited to Chapters 1 and 5. Next:
  run the full standard agent tier, then stage and prove regeneration adds no
  drift.

- 2026-08-04T13:06:00Z — Final standard agent tier is green with synchronous
  exit-0 receipts: `devenv shell -- cargo fmt --all --check`; `devenv shell --
  cargo run -p cix -- fmt --check examples`; `devenv shell -- cargo clippy
  --workspace --all-targets -- -D warnings`; and `devenv shell -- cargo test
  --workspace -- --test-threads=1`. After staging the intended generated
  pages, `devenv shell -- cargo test -p cix --test tour -- --ignored
  generate_tour` followed by `git diff --exit-code -- docs/tour` proved zero
  regeneration drift; the exact `generated_tour_is_deterministic` test also
  passed. An earlier workspace attempt hit one transient cache.nixos.org DNS
  failure; resolver recovery plus the exact failed tour test and the fresh
  complete suite all passed. No VM implementation or scenario changed, so the
  track correctly stops at its declared agent tier. Next: final staged review
  and commit.

- 2026-08-04T00:00:00Z — Started `track/browser3` after reading `AGENTS.md`,
  `.dev/LOG.md`, `docs/design.md`, `docs/corpus.md`'s corpus-maintenance
  loops, and `.dev/specs/track-browser3.md`. Scope is the deterministic corpus
  browser generator/output plus fetched upstream `context.files` manifests and
  the fetch helper; corpus ledgers, GAPS content, Cixfiles, locks, receipts,
  and checks remain fenced. Current browser only discovers selected top-level
  files, so implementation will make recursive artifact discovery explicit,
  add fixture coverage for variants/gap panels, and add the real-parser rot
  guard before regenerating pages and running the declared agent tier.

- 2026-08-04T00:20:00Z — Extended `corpus/migrate/fetch.sh` to write sorted
  `<relative-path>\t<bytes>` `context.files` after each successful fetch, then
  fetched all 15 cases whose SOURCE supplies the helper's required repository,
  immutable revision, and context path: adminer, directus, dozzle, echo-server,
  excalidraw, filestash, memcached, nginx, parse-server, phpmyadmin, redis,
  tomcat, verdaccio, wallos, and watchtower. Each synchronous invocation exited
  0 and produced its checked-in manifest. The remaining SOURCE records cannot
  honestly produce a context manifest with their checked-in provenance: caddy
  reports `SOURCE does not declare a context path`; mastodon and renovate report
  `SOURCE lacks a parseable resolved revision`; nats, traefik, and whoami report
  `SOURCE lacks a parseable repository URL`. Their pages deliberately say
  "context not fetched". Exact repros: `bash corpus/migrate/fetch.sh <case>`.

- 2026-08-04T01:10:00Z — Browser implementation and regenerated `docs/corpus/`
  are complete in two logical commits: `0c271dd` records the fetched manifests
  and `fd1e1ad` expands recursive artifact discovery/rendering. The new focused
  corpus suite passes synchronously, including real-parser rot guard and fixture
  tests for faithful-default Cixfile tabs, tabless solo Cixfiles, and stale gap
  panel markdown. A denied-warning clippy run found and the final source change
  fixes one `push_str("`")` lint in the inline markdown renderer; corpus was
  regenerated again afterward with no additional drift.

- 2026-08-04T01:20:00Z — Final agent-tier receipts, all synchronous exit 0:
  `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo run -p cix
  -- fmt --check examples`; `devenv shell -- cargo clippy --workspace
  --all-targets -- -D warnings`; `devenv shell -- cargo test --workspace --
  --test-threads=1` (the foreground status-capture wrapper recorded `0` in
  `/tmp/track-browser3-workspace.status`); `devenv shell -- cargo test -p cix
  --test tour -- --ignored generate_tour`; and `git diff --exit-code --
  docs/tour`. The first ordinary parallel workspace attempt exposed a live-tour
  user-manager race in `tour_ignores_a_foreign_user_unit`; its exact isolated
  repro then passed, and the complete serialized suite was used for the final
  green receipt. Corpus regeneration and its ordinary drift/determinism test
  both passed after the final lint fix; `git diff --exit-code -- docs/corpus`
  was clean. No VM scenarios are in this track's scope.

- 2026-07-31T23:00:00Z — Started `track-cigreen2` after reading `AGENTS.md`,
  the repository/design journals, and `.dev/specs/track-cigreen2.md`. Scope:
  eliminate the cix-index serve/pull listen race and the systemd-261
  compose-fallback VM regression without weakening the D36 contract. Required
  final repros (to be recorded with results): `devenv shell -- cargo fmt --all
  --check`; `devenv shell -- cargo clippy --workspace --all-targets -- -D
  warnings`; `devenv shell -- cargo test --workspace`; `devenv shell -- cargo
  test -p cix --test cold_audit -- --ignored`; and `devenv shell -- nix flake
  check -L`.

- 2026-07-31T23:20:00Z — Reproduced `devenv shell -- nix build
  .#checks.x86_64-linux.compose-fallback-vm -L --no-link`: after the expected
  loud D36 PrivatePIDs fallback, `cix up` failed at
  `crates/cix-compose/src/runtime.rs:336`, whose `nix-store --add-root
  --indirect --realise` correctly requires every live service item to be in
  the VM's Nix store. `nix/compose-fallback-vm.nix` had used two manifest-only
  fixture paths without placing them in the test system closure; Nix attempted
  an unavailable cache substitution and the test failed at `assert status ==
  0`. Added those exact items to `system.extraDependencies` and assertions
  that both service roots under `/var/lib/cix-compose/gcroots/fallback/` point
  at them. This preserves the GC contract rather than degrading it. The exact
  repro above is now green. The cix-index pull test now has a five-second
  bounded `TcpStream::connect_timeout` readiness guard for each spawned serve
  instance; focused verification passed: `devenv shell -- cargo fmt --all
  --check` and `devenv shell -- cargo test -p cix-index --test pull`.

- 2026-07-31T23:40:00Z — Required Rust gates pass: `devenv shell -- cargo fmt
  --all --check`; `devenv shell -- cargo clippy --workspace --all-targets --
  -D warnings`; and `devenv shell -- cargo test --workspace`. The cold-audit
  sweep initially exhausted the shared `/tmp` tmpfs while the gitsitter
  example's sandbox built its release target (`No space left on device` at
  `/work/target`); this was an environment capacity failure, not an audit
  mismatch. Re-ran it with its cix temporary workspaces on the worktree
  filesystem and it passes: `env
  TMPDIR=/home/mathijs/composix/.worktrees/cigreen2/target/builder-tmp devenv
  shell -- cargo test -p cix --test cold_audit -- --ignored`. Next: the full
  `devenv shell -- nix flake check -L` gate.

- 2026-08-01T00:00:00Z — FINAL GATE GREEN. `devenv shell -- nix flake check
  -L` passed all 61 checks, including `compose-fallback-vm`, `vm-dogfood`, and
  every scenario VM. Complete successful repro set: `devenv shell -- cargo fmt
  --all --check`; `devenv shell -- cargo clippy --workspace --all-targets --
  -D warnings`; `devenv shell -- cargo test --workspace`; `env
  TMPDIR=/home/mathijs/composix/.worktrees/cigreen2/target/builder-tmp devenv
  shell -- cargo test -p cix --test cold_audit -- --ignored`; and `devenv shell
  -- nix flake check -L`. The latter includes the direct VM reproduction
  `devenv shell -- nix build .#checks.x86_64-linux.compose-fallback-vm -L
  --no-link`, which also passed after the fixture correction. Next: final diff
  audit and commit on `track/cigreen2`.

- 2026-07-31T22:30:00Z — Completed `track-cifix`: `artifact_kinds` now retains
  a PATH-resolved `/nix/store` shell, otherwise asks Nix for `nixpkgs#bash` and
  selects the printed output containing `bin/sh` (the derivation can print a
  manual output first). A failed lookup/build is an honest skip; the fixture
  never passes a host executable to cix. Gates pass: `devenv shell -- cargo fmt
  --all --check`; `devenv shell -- cargo clippy --workspace --all-targets -- -D
  warnings`; and `devenv shell -- cargo test --workspace`. Both fixture paths
  pass: `devenv shell -- cargo test -p cix --test artifact_kinds` (devenv store
  shell) and `bash -c 'cix_cargo=$(command -v cargo);
  PATH=/nix/var/nix/profiles/default/bin:/usr/bin:/bin "$cix_cargo" test -p cix
  --test artifact_kinds -- --nocapture'` (host `/usr/bin/sh`, registry fallback).
  The track's literal `PATH=/usr/bin:/bin cargo test -p cix --test
  artifact_kinds` cannot start on this host because neither `cargo` nor `nix`
  is installed there; the recorded equivalent preserves only Nix/Cargo launch
  paths and excludes every store shell.

- 2026-07-31T00:00:00Z — Started `.dev/specs/track-cifix.md` after reading the
  repository context. Scope is `crates/cix/tests/artifact_kinds.rs`: make its
  app fixture select a store-backed `sh` even when host PATH resolves
  `/usr/bin/sh`, with an honest skip if Nix cannot provide `nixpkgs#bash`.
  Planned verification: `devenv shell -- cargo fmt --all --check`; `devenv
  shell -- cargo clippy --workspace --all-targets -- -D warnings`; `devenv
  shell -- cargo test --workspace`; and the stripped-PATH artifact-kinds
  reproduction specified by the track.

- 2026-07-31T01:30:00Z — Final explicit tour gate (`devenv shell -- cargo test -p
  cix --test tour`) and `git diff --check` pass after the VM work. Removed the
  transient untracked `devenv.lock`. The commit contains the ignored D47(e) audit,
  its named unstable FETCH exclusions, D64-correct node/redis examples, and the
  repaired observability cgroup receipt. No FETCH pin changes are committed because
  both audit-discovered stale candidates failed the required two-clean-update stability
  check. `nix flake check`'s independent GC-survival failure remains an open product
  item; all gates required by track-coldaudit itself are green.

- 2026-07-31T00:20:00Z — Orchestrator authorized the required deliberate refresh
  investigation for `examples/build/projB`. Before changing its source lock, I ran
  `target/debug/cix build --update-lock build` against two independently copied
  contexts, each with empty `CIX_STATE_DIR` and `CIX_BUILD_WORKSPACE_DIR`. The cargo
  FETCH pins differed: first `sha256-06PCsUnBBcaZC2A4QNZ9Wgmq4oXAM4yKpNYz3NYoJlw=`,
  then `sha256-kqhKUDMp1Msbe8ohn1j6eAPZ6SUUMpR9OsQ1KBVr2IE=`. This is a second
  cargo-FETCH pin-instability exhibit in the documented dozzle class, not a
  stale-since-toolchain-move pin: no re-pin is valid. The cold-audit sweep now
  explicitly excludes `examples/build/projB` with that exhibit name and diagnostic;
  the remaining sweep will reveal any independent stale pins.

- 2026-07-31T00:30:00Z — The resumed sweep exposed another stale reported pin in
  `examples/build/projB-chef`, for `FETCH cargo chef cook`. Two independently copied,
  clean-state `--update-lock build` samples again disagreed:
  `sha256-vGc+zmLVj4WvqNizTznaDhQZ6DZAjdMOeFB9VYtEH/c=` then
  `sha256-Uq2xVgN/N05IwL3PPy0i1O8BC5bsP0E89I7OpzBNapA=`. It is therefore another
  dozzle-class cargo-FETCH pin-instability exhibit, not stale-since-toolchain-move;
  its source lock remains untouched and the sweep explicitly excludes it with the
  named diagnostic. Next: resume the complete examples scan for any stable stale pin.

- 2026-07-31T00:40:00Z — With the two unstable cargo FETCH examples excluded, the
  audit reached `examples/pack/node-app` and showed a separate, deterministic
  stale-since-toolchain-move example drift: D64 now resolves bare EXEC from the
  artifact's own `/bin`, whereas node-app retained its old external `ENV PATH` form.
  `examples/pack/redis` had the same bare-command pattern. Replaced both external
  PATH declarations with explicit `/bin/node` and `/bin/redis-server` LINKs, the D64
  provenance form. These are no-pin source adjustments; neither FETCH lock was
  refreshed. The only discovered FETCH locks (projB and projB-chef) remain unstable.

- 2026-07-31T01:00:00Z — The fmt, `-D warnings` workspace clippy, workspace tests,
  complete explicit cold audit, and focused foreign-user tour regression all pass.
  The `nix flake check` VM tier then exposed a stale test receipt unrelated to the
  audit: `scenario-observability` correctly found `cix-observe.slice` in `systemctl`
  but asserted the obsolete flat cgroup path. systemd's hierarchical slice naming
  places it at `/sys/fs/cgroup/cix.slice/cix-observe.slice`; the direct serial repro
  failed at the old assertion, so I corrected that receipt before rerunning the VM
  gate. This is a gate-maintenance adjustment, not a product behavior change.

- 2026-07-31T01:20:00Z — Required gate status: `devenv shell -- cargo fmt --all
  --check`, `devenv shell -- cargo clippy --workspace --all-targets -- -D warnings`,
  `devenv shell -- cargo test --workspace`, and `devenv shell -- cargo test -p cix
  --test cold_audit -- --ignored` pass. The direct repaired observability VM repro
  (`devenv shell -- nix build .#checks.x86_64-linux.scenario-observability -L
  --no-link`) passes, as does the spec-required dogfood VM gate (`devenv shell -- nix
  build .#checks.x86_64-linux.vm-dogfood -L --no-link`). The latter exercised the
  linked node-app and succeeded; its node stop waits for systemd's 90s timeout under
  this VM. A broader `devenv shell -- nix flake check` remains honestly non-green on
  `scenario-gc-survival`: following a tag move, its compose profile leaves the old
  API item unrooted and `nix-collect-garbage` removes it. That is an out-of-scope
  D68-era compose/GC contract regression, retained as a failing assertion rather than
  weakened. Next: rerun the explicit tour gate, remove transient `devenv.lock`, review
  the diff, and commit the track work.

- 2026-07-31T00:00:00Z — Started `.dev/specs/track-coldaudit.md` after reading the
  repository context and D47(e). Scope is a new ignored host-side `cold_audit` cix
  integration test plus gate convention record; corpus content and `cix-cixfile`
  sources stay untouched. I will compare each real `cix build <dir>` member JSON map
  against `cix build --cold <dir>`, add an opt-in fetched corpus pair mode, prove the
  mismatch diagnostic with a temporary fixture, then run: `devenv shell -- cargo fmt
  --all --check`; `devenv shell -- cargo clippy --workspace --all-targets -- -D
  warnings`; `devenv shell -- cargo test --workspace`; `devenv shell -- cargo test -p
  cix --test cold_audit -- --ignored`; `devenv shell -- cargo test -p cix --test tour`;
  and `devenv shell -- nix flake check`.

- 2026-07-31T00:10:00Z — Added `crates/cix/tests/cold_audit.rs`: it snapshots each
  `examples/**/Cixfile` tree into ignored `target/test-tmp`, uses isolated
  `CIX_STATE_DIR` and `CIX_BUILD_WORKSPACE_DIR`, seeds the documented `my-nginx:v1`
  producer for `build/from-item`, then compares the real warm and `--cold` JSON member
  maps member by member. `COLD_AUDIT=<pair>` fetches one corpus context with
  `bash corpus/migrate/fetch.sh <pair>` before auditing its snapshot. The ignored proof
  `nondeterministic_builder_is_rejected` passed with `cargo test -p cix --test
  cold_audit -- --ignored nondeterministic_builder_is_rejected`; it confirms the real
  cold bridge reports `COPY ${build}/x (line 6) differs between warm and cold`.
  The prescribed full audit currently stops before comparison on the warm
  `examples/build/projB` build: `devenv shell -- cargo test -p cix --test cold_audit
  -- --ignored every_example_matches_a_clean_rebuild` reports the committed
  `FETCH cargo fetch --locked` pin `sha256-HYh…` differs from current fetched output
  `sha256-jPdm…`. No example or corpus content has been changed. A source-lock refresh
  is required to make the specified every-example gate green; that is outside this
  track's stated crates/cix-plus-gate-doc scope, so work awaits direction.

- 2026-07-29T02:23:35Z — Started the cold-cache repair for scenario 10. The first fresh tour render can include Nix progress in the compose transcript, causing committed-document drift despite later renders passing. I will make the harness warm each build-performing compose command before recording its shown invocation, preserving the displayed command and output as one real execution, regenerate the page if needed, then verify three first-run fresh-state renders and commit the repair.

- 2026-07-29T02:25:32Z — Implemented compose-diff pre-warming in the shared tour harness. `check` only resolves, while each `diff` constructs a generation; each displayed diff is still a second, real invocation captured verbatim after an unrecorded first-use warm-up. Regeneration left `docs/tour/` unchanged. Three complete fresh-temp-state `devenv shell -- cargo test -p cix --test tour` runs passed, including document drift, consecutive-render determinism, and the foreign-user-unit regression; `cargo fmt --all --check` and `git diff --check` also pass. Next: remove the transient devenv lock and commit the repair.

- 2026-07-29T03:00:00Z — Started the post-compose ledger and tour pass. Compose v0 supplies rootless `cix compose check` and `diff`; `up`, `down`, and `rollback` intentionally operate the system manager as root. I will verify a self-contained two-item rootless fixture, update the Compose ledger with implementation receipts, add scenario 10 and its generated page, then run the requested drift/determinism and workspace gates three times.

- 2026-07-29T03:05:00Z — Reconciled docker-ledger §6 against the v0 implementation and stack demo. The section now distinguishes implemented strict JSON, edge ordering, namespaced profiles/units, root activation, rootless check/diff, and retained profile rollback from still-deferred scale, health, credentials, limits, reusable objects, networks, and reconciler work. Next: add and run the rootless compose tour fixture.

- 2026-07-29T03:15:00Z — Added scenario 10, “Composing services,” and regenerated `docs/tour/`. It makes two self-contained cix-spec v2 items with `nix store add-path`, tags a tracked service, shows strict JSON plus rootless `check`, records the actual resolved tag values in the root-only-up lock format, then dry-diffs before and after retagging. Focused generation passes. Next: run tour drift/determinism (including the foreign-unit decoy) three times, then the workspace gate.

- 2026-07-29T03:25:00Z — Verification complete. Three consecutive `devenv shell -- cargo test -p cix --test tour` runs passed committed-document drift, consecutive-render determinism, and the live foreign-user-unit decoy regression. The final workspace gate also passed: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`. Removed the transient `devenv.lock`; no cix test units remain. The compose ledger and rootless scenario 10 are complete.

- 2026-07-29T01:11:49Z — Started the hermetic-tour fix. The renderer currently emits raw `cix ps` rows, including the shared `cix-run.slice`, so foreign `cix-*` units can change generated transcripts. I will filter displayed unit-list rows to each scenario's generated names, add a live foreign-unit regression test with cleanup, regenerate `docs/tour/`, and run the workspace gate three times while a decoy user unit is active.

- 2026-07-29T01:12:00Z — The initial focused test confirmed the decoy regression test works, but also exposed that `cix ps` formats its header and retained rows using the widest foreign unit before the harness filters output. The filter now reparses its retained rows and regenerates the table with widths based only on scenario units; this preserves the real command while making its rendered transcript hermetic.

- 2026-07-29T01:15:26Z — Regenerated `docs/tour/` (only `07-running-service.md` changed) and committed the implementation as `09ef804`. Focused tour drift, determinism, and live `systemd-run --user --unit=cix-run-decoy-x sleep 60` regression tests pass. Then ran `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` three consecutive times, each with a separately started and confirmed-active foreign `cix-run-decoy-manual-*` user unit; all gates passed and every manual decoy was stopped afterward. The transient untracked `devenv.lock` will be removed before handoff.

- 2026-07-29T00:40:54Z — Started `.dev/specs/track-tour5.md`. Scope is confined to the cix tour harness and generated `docs/tour/` pages; examples remain read-only. The checked-out shell has not entered the repository’s `devenv` environment (`IN_NIX_SHELL` is empty), so I will obtain confirmation before running Nix/Rust work that needs the project environment. Next: establish the Cixfile fixture/lock behavior and probe user-manager transient socket feasibility.

- 2026-07-29T00:45:00Z — Resumed `.dev/specs/track-tour5.md` under explicit instruction not to wait for environment confirmation. The tour harness can write a zero-PKG Cixfile into its isolated temp directory, but Cixfile code generation still needs a nixpkgs lock because `SCRIPT` obtains its runtime shell from that pin. The scenario will copy a committed fixed lock into place, document that deterministic contract, and normalize the built store path. Next: compile the current harness and probe the spec-v3 listener fixture against the user manager.

- 2026-07-29T01:05:00Z — Implemented and generated the two new scenarios. The Cixfile scenario writes its own FILE, SCRIPT, SERVICE, and fixed `Cixfile.lock`, runs `cix build . -t tour-app:v1`, cats the generated spec, and lists its tag. The listener probe succeeded against systemd 257's user manager: a v3 fixture received `LISTEN_FDS=1` through a transient `.socket`, served its HTTP response, and stopped cleanly. The committed tour fixture reproduces that with an isolated Python listener, a unique normalized port, readiness polling, curl, and stop. The existing user-service page exposed retained failed fallback attempts in `cix ps`; the renderer now suppresses only those stale normalized rows, restoring its promised deterministic transcript while preserving active-unit output. Focused generation and normal tour drift/determinism tests pass. Next: review the generated pages, commit this implementation, then run the required full workspace gate three times.

- 2026-07-29T01:20:00Z — Committed the implementation as `fb0c623` (`Extend cix tour with Cixfile and listener scenarios`). The complete gate passed three consecutive times in `devenv`: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace`; every run included the generated-tour drift and consecutive-render determinism checks. Removed the transient untracked `devenv.lock`, stopped the test-created user `cix-run.slice`, reset the failed listener test units, and confirmed no `cix-*` units remain in either manager and the worktree is clean. No open work remains for track-tour5.

- 2026-07-28T21:00:00Z — Started the tour fixture one-liner update. On this host (Determinate Nix 2.34), `echo 'hello from my app v1' | nix store add --name my-app-v1 /dev/stdin` exits successfully but stores a `/proc/self/fd/0` symlink rather than stdin content; `--mode flat` rejects that symlink. The honest one-line fallback writes a named regular file before adding and tagging it.
- 2026-07-28T21:05:00Z — Updated every ordinary tour fixture transcript to one executable line: `echo … > my-app-vN && cix tag "$(nix store add my-app-vN)" my-app:v1`. The helper reads cix's real GC root for its assertions, so displayed and executed commands remain identical. Regenerated all tour pages; focused drift and consecutive-render determinism tests pass.
- 2026-07-28T21:49:17Z — Final verification passed three times in `devenv`: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace`. Each full run included the tour drift/determinism checks and user-run integration test. No open questions remain.

- 2026-07-28T19:45:23Z — Started `specs/track-litdoc.md`. Read D19 and the gitsitter reference; scope is limited to the cix integration-test harness, generated `docs/tour.md`, and necessary dev-dependencies. No deviations yet.
- 2026-07-28T19:48:00Z — Inspected the real CLI and local-index layout. The root and sidecar filenames are base64-url encodings of refs, so the tour will deliberately show that observable storage representation. Following the gitsitter header pattern, the commit uses `GIT_COMMIT_HASH` when supplied at build time and otherwise `unknown`; this avoids a runtime git lookup making the drift check unstable.
- 2026-07-28T19:51:00Z — Added the tour harness and direct `regex`/`tempfile` dev-dependencies. Initial generator compilation found a `Cow<str>` pattern mismatch in normalization; corrected it before running scenarios. The harness retains a `TempDir` per scenario so temporary state is removed after rendering.
- 2026-07-28T19:53:00Z — Generated `docs/tour.md` from the real binary. The normal drift check and the required two-consecutive-render determinism test both pass. The generated document shows only tag, ls, untag, GC roots, and sidecars; no serve, pull, or claim surface was added.
- 2026-07-28T19:55:00Z — Final verification passed: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace`. No deviations or open questions remain.
- 2026-07-28T20:00:00Z — Started `specs/track-tour2.md`. Scope is restricted to the cix tour harness, its generated document, and cix dev-dependencies if needed. Next: inspect the serve/pull design and established tour conventions before extending the generator.
- 2026-07-28T20:05:00Z — Read D12, D17 v2, D18 v2, the HTTP/org-workflow sections, and the current harness. The tour will use the required fixed `127.0.0.1:8420` endpoint, normalize it, and manage each serve process with a drop guard plus HTTP polling. To preserve scenario independence and demonstrate the current upstream model faithfully, the adoption scenario uses `--as my-app`, while the moved-tag refresh scenario first creates its own qualified mirror so bare `cix pull` retains the remote `:v1` tag it refreshes. No source behavior will be changed.
- 2026-07-28T20:15:00Z — Extended `crates/cix/tests/tour.rs` with the three requested scenarios. Each starts a real `cix serve --with-store` child on the fixed loopback port, polls its JSON endpoint until ready, and kills/waits for the child through a drop guard. Regenerated `docs/tour.md`; the focused generator, drift check, and consecutive-render determinism test pass. Next: review generated readability and run the full required gate.
- 2026-07-28T20:20:00Z — Final verification passed in `devenv`: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace` (including tour drift and determinism). The generated document was reviewed for readable publisher/consumer transcripts. Deviation: the independent refresh scenario begins with a qualified mirror rather than the prior scenario's `--as` adoption, because bare refresh tracks the local reference's name/tag; this keeps the promised `:v1` refresh real without changing product code. No open questions.
- 2026-07-28T20:25:00Z — Fixed concurrent tour rendering: every `cix serve` call now receives a distinct loopback port from an atomic counter. The renderer normalizes each real address to the documented `127.0.0.1:8420`, keeping the checked-in transcript byte-stable while parallel drift and determinism tests no longer contend for one socket.
- 2026-07-28T20:30:00Z — Started `.dev/specs/track-tour3.md`. Scope is limited to the cix tour harness, generated tour pages, tour links, and any necessary cix dev-dependencies. Read the existing harness, generated tour, D13/D19, and the user-run integration fixture. The new scenario will use a v2 store fixture with one state directory and `--user`; its cleanup guard will stop the generated transient unit on every path. No ambiguity requiring a scope expansion identified.
- 2026-07-28T20:35:00Z — Replaced the monolithic generated tour with `docs/tour/index.md` and seven numbered scenario pages, each with stable Jekyll `.html` navigation. The drift check now compares the full generated filename set and each page's contents, so stale, deleted, and renamed files all fail. Added the final rootless service scenario: it puts a v2 spec plus shell service in the Nix store, runs it detached via `--user`, lists it with `cix ps`, stops it, and guards cleanup with `systemctl --user stop` on unwind. Unit nonces and multi-line host diagnostics are normalized while the degraded-mode warning remains visible. Generated pages and focused tour tests pass.
- 2026-07-28T20:40:00Z — Final gate passed three consecutive times in `devenv`: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, and `cargo test --workspace`. All tour drift/determinism checks and the existing user-manager integration test passed each time. The generated pages were reviewed for complete `.html` index/previous/next links. No deviations or open questions remain.

## 2026-08-04 — track/tour2: new-user guide

- 2026-08-04T00:00:00Z — Started `.dev/specs/track-tour2.md` on
  `track/tour2` after reading the repository journal, design registry, cix log,
  current executable tour, and relevant feature/scenario inventory. The fixed
  plan is seven chapter-sized commits in blueprint order, followed by the
  generated index/link sweep and the standard bounded agent gate. Scope stays
  inside `crates/cix/tests/tour.rs`, generated `docs/tour/`, tour links, and
  this log. Any `ENOSPC` is a reportable stop; no disk workaround will be used.

- 2026-08-04T11:20:00Z — Chapter 1 is implemented and generated as
  `01-hello-composix.md`. Its real fixture builds an IMPORT-based nginx item,
  inspects the emitted manifest, runs through the explicitly degraded user
  manager, probes the served page, and stops the unit. The first probe exposed
  the documented D13 boundary: this host drops item BindPaths, so the fixture
  honestly uses its locked source path and private `/tmp` runtime files while
  retaining the production role-dir declarations. Focused synchronous
  `generate_tour` passed; no ENOSPC occurred.

- 2026-08-04T11:35:00Z — Chapter 2 is implemented and generated as
  `02-cixfile-language.md`. The growing real Cixfile proves earlier-wins
  universal IMPORT, local materialization, package linking, forced
  materialization below a role mount, interpolated FILE, required ENV, all
  lifecycle dirs, TCP+UDP ports, LISTENER, and egress/JIT claims; its compact
  manifest projection and closing directive table are asserted. Focused
  generation passed synchronously after fixture-path corrections; no ENOSPC.

- 2026-08-04T11:55:00Z — Chapter 3 is implemented and generated as
  `03-building.md`. One continuous executable story covers author-EXPECT and
  double-fetched automatic pins, offline RUN, a zero-Nix warm memo receipt,
  compact lock/dev-env evidence, successful cold replay, then a locally served
  genuinely FHS-linked ELF whose first RUN emits the real loader/libc IMPORT
  hint and whose second RUN succeeds after adding glibc. The proj1 capstone
  proves one warm Cargo workspace, two items, member selection, and unchanged
  API identity after a worker-only edit. Focused generation passed
  synchronously; no ENOSPC.

- 2026-08-04T12:10:00Z — Chapter 4 is implemented and generated as
  `04-naming-distribution.md`. It keeps exactly one short raw-tree aside, then
  executes post-build family tagging, compact inspection, move-as-tag+untag,
  removal/GC semantics, a moved tag, content-negotiated serve with a real Nix
  binary cache, adopted pull, and upstream refresh after a second move. The
  product has no literal mv/rm image-object verbs, so the prose teaches the
  actual tag/untag model rather than inventing them. Focused generation passed
  synchronously; no ENOSPC.

- 2026-08-04T12:30:00Z — Chapter 5 is implemented and generated as
  `05-runtime-contract.md`. A real tagged Python HTTP service exercises native
  readiness/liveness adapters, rootless run, probe, debug, filtered ps, stats,
  and logs explanation; a real APP produces and activates a user timer with
  deterministic cleanup. Closed-root write denial, STATEDIR restart
  persistence, credential delivery/rotation, and watchdog restart are stated
  as system-manager guarantees with exact closedroot/dirs2/secrets/health VM
  pointers because this host's user manager rejects mount projection. Narrowed
  age normalization so declared `10s`/`2s` health durations no longer become
  fake `0s`. Focused generation passed synchronously; no ENOSPC.

- 2026-08-04T12:50:00Z — Chapter 6 is implemented and generated as
  `06-compose.md`. It executes LISTENER fd-3 socket activation, builds two
  Cixfile services, validates a Unix edge plus shared STATEDIR and
  `logNamespace`, records the real two-ref lock, dry-builds/diffs generations,
  demonstrates unary run, then moves the tracked producer and diffs again.
  `up`/`rollback`/`down`, setgid sharing, pod netns, and journal namespace are
  clearly system-manager-only prose with exact lib/dirs2/netns/observability VM
  pointers. Focused generation passed synchronously; no ENOSPC.

- 2026-08-04T13:10:00Z — Chapter 7 is implemented and generated as
  `07-dev-loop-docker.md`. Its real watcher rebuilds after an edited source
  file; the watched workspace is created below cix's prescribed cache temp
  area because target-tree paths are intentionally ignored, then linked into
  the continuous story. Faithful and dissolved `--file` Docker twins build
  independently with their own locks, and the chapter links to the migration
  workflow and corpus. Rewrote the generated index around the seven-chapter
  learning path, swept obsolete page names, and removed superseded generator
  code. Focused `generate_tour` passed synchronously (1 passed in 28.11s); no
  ENOSPC occurred.

- 2026-08-04T13:25:00Z — The first full workspace test exposed one
  determinism leak: the FHS fixture's deliberately fresh loopback port changed
  the real FETCH command memo key between consecutive renders. Normalization
  now labels that derived receipt `<command-key>` while retaining the
  content-hash EXPECT and real miss/hit behavior. The exact consecutive-render
  test passed (1 passed in 55.49s), and regeneration passed (1 passed in
  28.83s). This was not ENOSPC and no disk workaround was used.

- 2026-08-04T13:45:00Z — Final bounded agent gate passed synchronously
  on committed head `0ff3131`: `cargo fmt --all --check`; `cix fmt
  --check examples` (exit 0 with the pre-existing LINK deprecation
  notices); warning-denied workspace/all-target Clippy; and serial full
  workspace tests, including tour determinism/drift, the foreign-unit guard,
  watch, integration suites, and doc tests. One intervening workspace attempt
  met the fixed decoy name while the preceding test's transient unit was still
  unloading; read-only inspection immediately showed it absent, and the
  unchanged guard passed in the complete rerun. Explicit tour regeneration
  passed (1 passed in 51.93s), followed by `git diff --exit-code -- docs/tour`
  and the obsolete-page-name sweep at exit 0. All commands used `nice`, six
  Cargo/Nix jobs, and four Nix cores. No ENOSPC occurred.

- 2026-08-04T14:10:00Z — Began the blocking orchestrator review fix for
  CIP-91 teaching canon. Chapters 1 and 5 will retain only bare imported
  commands plus copied absolute item paths in their Cixfiles; this host's
  rootless receipts will validate the physical built item without using a
  locked source path to make the service appear runnable. Next: regenerate,
  review the honest degraded prose, and repeat the bounded agent gate.

- 2026-08-04T14:35:00Z — Chapters 1 and 5 now generate only CIP-91-canonical
  service declarations: imported bare commands, local files copied into
  absolute item paths, and no source binder in START argv. Chapter 1's real
  nginx debug receipt accepts the exact copied config syntax and then exposes
  the unavailable CACHEDIR instead of serving through a source path. Chapter
  5 parses its copied Python directly from the immutable item, uses the
  mount-free APP for its tagged debug receipt, and leaves HTTP health to the
  linked system-manager scenario. The openings and index no longer promise a
  rootless HTTP run. Focused generation passed (1 passed in 15.58s) and the
  exact consecutive-render check passed (1 passed in 28.30s); no ENOSPC.

- 2026-08-04T14:55:00Z — The bounded review-fix gate passed synchronously:
  `cargo fmt --all --check`; `cix fmt --check examples` (exit 0 with the
  existing LINK deprecation notices); warning-denied workspace/all-target
  Clippy; and serial full workspace tests, including tour determinism/drift,
  foreign-unit isolation, watch, integrations, and doc tests. Final explicit
  tour regeneration passed (1 passed in 15.63s). All gate commands used
  `nice`, six Cargo/Nix jobs, and four Nix cores. No ENOSPC occurred.

- 2026-08-04T15:10:00Z — Restored the chapter 5 `cix ps` surface as an
  honest header-only receipt after removing the unprojectable web unit, then
  repeated the complete bounded gate on that final source. Formatting,
  examples formatting, warning-denied Clippy, serial full workspace tests,
  tour determinism/drift, and explicit regeneration (1 passed in 15.62s) all
  passed synchronously. No ENOSPC occurred.
