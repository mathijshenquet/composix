# VM dogfood log

- 2026-07-30T00:00:00Z: Collision cleanup made the side-by-side VM exceed five
  minutes. The collision is intentionally a failed compose and is the last assertion
  in an isolated disposable VM, so it need not be brought down before VM teardown;
  retain alpha/beta cleanup and leave the failed collision to the test harness.

- 2026-07-30T00:00:00Z: Side-by-side collision diagnosis: the conflicting listener
  reliably fails with Address already in use, but current cix up exits successfully
  after systemd reaches the compose target even when that socket is failed. This track
  is fenced from runtime semantics, so the scenario now records the real loud systemd
  failure (failed socket plus its journal line) and cleans down the collision; a
  nonzero cix up exit remains a compose-runtime follow-up.

- 2026-07-30T00:00:00Z: Side-by-side's repaired HTTP checks both passed, including
  beta's systemd retry after a transient namespace setup failure. Its next assertion
  exposed a stale cgroup filesystem path: compose slices live under /cix.slice.
  Replaced the raw path checks with exact systemd ControlGroup assertions for
  /cix.slice/cix-alpha.slice and /cix.slice/cix-beta.slice.

- 2026-07-30T00:00:00Z: Once HTTP was repaired, lifecycle reached its state assertion:
  the app writes through the declared /var/lib/api alias, but DynamicUser keeps the
  host-visible managed backing directory at /var/lib/private/cix-lifecycle-api.
  Updated all lifecycle persistence assertions to that systemd backing path; this
  checks the intended host-visible state across activation, update, and rollback.

- 2026-07-30T00:00:00Z: Store-level fixture inspection found the final root cause
  behind curl's immediate Empty reply from server: the Nix literal emitted doubled
  escapes into Python, producing literal \\r\\n/\\n rather than HTTP line endings
  and a newline-terminated body. Corrected the generated Python source to use normal
  \r\n and \n escapes. This is why the API could stay active with neither a db-error
  log nor a parseable HTTP response.

- 2026-07-30T00:00:00Z: The fast lifecycle probe then gave `curl: (52) Empty reply
  from server` while the API remained active and its journal had no exception. This
  isolated the live stall to the db fixture's single-threaded, unbounded accepted
  connection: a peer that connects without sending can monopolize its `recv()` and
  leave subsequent API requests unanswered. Every db client connection is now capped
  at 2s and discarded on `OSError`, complementing the API's bounded per-request db
  exchange. Restored lifecycle's normal `wait_until_succeeds` contract; its curl is
  still capped at 5s per attempt.

- 2026-07-30T00:00:00Z: The first bounded retry run established that the failure was
  no longer an unbounded client read, but a permanent Unix-edge failure between the
  two DynamicUser services. The db fixture had left its bound socket at the process
  umask mode (`0755`), which permits traversal but not connection by the API's
  different dynamic UID. It now explicitly publishes that fixture endpoint as `0666`
  after bind and before listen; this is a cross-service test socket, not a new runtime
  permission policy. The API also logs a bounded per-request db exception while
  returning 503, so any subsequent fixture regression is observable rather than a
  900-second opaque wait.

- 2026-07-30T00:00:00Z: First forced lifecycle attempt did not run the VM because
  `nix build --rebuild` requires an existing valid output and this changed test
  derivation had none (`some outputs ... are not valid, so checking is not possible`).
  Seed with one ordinary build, then use three consecutive `--rebuild` builds as the
  requested stability evidence; do not count this Nix precondition failure as a test
  result.

- 2026-07-30T00:00:00Z: Independent re-verification exposed a scenario-lifecycle
  liveness flaw: after both services logged ready (`db-line ready` at 47.234s and
  `api-line v1` at 47.242s), its HTTP wait hung for the full 900s because curl had
  no per-attempt deadline on an accepted-but-never-answered connection. Audited all
  nine scenario `wait_until_succeeds` curl calls and added `--max-time 5`. The API
  fixture now bounds client reads to 5s and each Unix-db transaction to 2s; a failed
  or malformed db exchange produces a closed 503 response, so the next probe can
  retry. The db's `ready` log remains after successful bind and listen. Repro before
  the fix: `nix build .#checks.x86_64-linux.scenario-lifecycle -L --no-link` could
  stall for 900s at the first curl; next: three forced-rebuild greens each for
  lifecycle and side-by-side, then the complete scenario/Rust gate.

- 2026-07-30T00:00:00Z: FULL GATE GREEN on the merged scenario track. Scenario repros: `nix build .#checks.x86_64-linux.scenario-lifecycle -L --no-link`, `nix build .#checks.x86_64-linux.scenario-side-by-side -L --no-link`, `nix build .#checks.x86_64-linux.scenario-update-repin -L --no-link`, `nix build .#checks.x86_64-linux.scenario-gc-survival -L --no-link`, and `nix build .#checks.x86_64-linux.scenario-observability -L --no-link`; all passed. Lifecycle now records the systemd-261 D36 `PrivatePIDs=yes` degradation loudly in both `cix up` output and `manifest.json`, with the version-derived reason. Also green: `cargo test -p cix-index --test hammer -- --ignored` (1.63s max child), `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo test --test tour -- --ignored generate_tour`, and `git diff --exit-code -- docs/tour` (zero tour diff). The Nix evaluator intermittently printed `SQLite database ... eval-cache ... is busy` as an ignored cache warning; it did not affect any derivation result.

- 2026-07-30T00:00:00Z: FINAL FRONTIER markers: D43 in `nix/scenarios/side-by-side.nix` — flip when `network: pod` permits identical internal ports with no host-bind conflict once both composites claim pod networking. D44 in `nix/scenarios/update-repin.nix` — flip when `--update <edge>` selectively repins nested composites. Current truth stays explicit: side-by-side uses path-namespaced units/slices/runtime directories and requires distinct host binds; lifecycle/update-repin prove selective service restart, generation rollback, and persistent state; gc-survival proves active profile/current-table roots and callable shortened history; observability proves systemd journal, slice, and cgroup surfaces rather than unimplemented `cix logs`/`ps`/`stats`.

- 2026-07-30T00:00:00Z: Focused lifecycle verification is green: `nix build .#checks.x86_64-linux.scenario-lifecycle -L --no-link`. It exercised the systemd-261 compose fallback end-to-end, including successful HTTP, selective restart, rollback, and state retention after the new warning/manifest assertions.

- 2026-07-30T00:00:00Z: Merged `main` (`track/composefallback`) into `track/scenarios` as `a1e3160`. The shared compose generator now probes the host and, on the systemd-261 DynamicUser+StateDirectory failure class, removes only `PrivatePIDs=yes`, warns that the service shares the host PID namespace (D36 degraded fallback), and records the same structured degradation in the generation manifest. Updated lifecycle's first `cix up` to capture and assert that warning plus the API's manifest entry, deriving the systemd version at runtime rather than baking host-varying detail into the scenario (the tour-normalization rule). Removed the now-irrelevant `kernel.unprivileged_userns_clone` experiment from the scenario VM. Next: `nix build .#checks.x86_64-linux.scenario-lifecycle -L`.

- 2026-07-30T00:00:00Z: Diagnosed the lifecycle `226/NAMESPACE` failure; no safe fix exists within the scenario/VM harness. `nix build .#checks.x86_64-linux.vm-dogfood -L` is green on the same kernel/framework, proving the D36 baseline (`DynamicUser=yes` + `PrivatePIDs=yes`) works. Compose also compiles in `UnitMode::System`, so it does not request `PrivateUsers=yes`; its API additionally has persistent `StateDirectory=cix-lifecycle-api:api` and the Unix-edge writable `BindPaths=` grant. The database sibling has the same D36 baseline and edge grant but no StateDirectory and starts. The API then fails in systemd 261's `sd-pidns` before exec: `Failed to allocate user namespace` / `Failed to set up mount namespacing: /var/lib/private/cix-lifecycle-api`. The NixOS test kernel has no `kernel.unprivileged_userns_clone` sysctl (attempts are logged as nonexistent); explicitly setting `user.max_user_namespaces=1024` and asserting it did not change the failure. A boot-time mutable-/etc experiment also did not change it, so the test's late writable-unit bind is not causal. This is D36's known userns-policy/fallback class, specifically the systemd DynamicUser persistent-directory + writable BindPaths combination, not CI saga class 3's hybrid uid-map denial. `cix run` has a loud D36 fallback, but compose's persistent generated units do not; faking a drop-in would silently weaken hardening and violate this track's fencing. Restored the harness after experiments. Last exact repro: `nix build .#checks.x86_64-linux.scenario-lifecycle -L` (fails/hangs waiting for its deliberately socket-activated API); exact control: `nix build .#checks.x86_64-linux.vm-dogfood -L` (green).

- 2026-07-30T00:00:00Z: Resumed track/scenarios focused on the lifecycle VM's `226/NAMESPACE` (`Failed to allocate user namespace`) failure. First evidence: D36 says `PrivatePIDs=yes` plus `DynamicUser=yes` realizes through an unprivileged user namespace; `vm-dogfood.nix` proves that path green in the same NixOS test framework. Next: compare compose unit properties with `cix run` and inspect the scenario VM's userns sysctls; prefer a VM/scenario configuration correction over weakening generated hardening.

- 2026-07-28: Started the VM dogfood track. The existing nginx and PostgreSQL examples are standalone Nix functions and their specifications already use only store paths, so the VM test will import them with its host `pkgs` and run those outputs directly.
- 2026-07-28: Added the minimal flake, VM test, and runbook. Pinned nixpkgs at `624af665418d3c65d544145b4d34ad696439570e`; `nix flake show --no-write-lock-file` evaluates the package and VM check successfully.
- 2026-07-28: First package build reached the Rust checks, which fail because the existing `cix` tour tests shell out to `nix` and the pure `buildRustPackage` sandbox does not provide it. Set `doCheck = false` for this binary package; the VM check is the integration verification required by this track.
- 2026-07-28: First VM run passed nginx. PostgreSQL then failed before initialization because the generated service script relied on an ambient PATH for `rm`, `mkdir`, and `mv`; transient units do not provide one. Replaced those invocations with explicit `pkgs.coreutils` store paths so the example remains self-contained under cix isolation.
- 2026-07-28: The repaired VM run reached both successful service checks and clean stops. The final `--all` assertion found the empty parent `cix-run.slice`, which systemd retains after collecting its transient services. The test now stops that empty slice and asserts that no active `cix-*` units remain.
- 2026-07-28: First complete VM check is green (50.70s in the guest test script): nginx HTTP and PostgreSQL TCP `SELECT 1` both passed, `cix ps` reported each transient service, and cleanup left no active cix units. Final committed-state verification will run the check twice more.
- 2026-07-28: Final summary: the flake exposes `packages.x86_64-linux.cix` and the `vm-dogfood` check; the isolated VM receives prebuilt example store paths and performs no guest-side Nix build or network access. `nix build .#checks.x86_64-linux.vm-dogfood` passed repeatedly, including a forced rebuild, after the final service and cleanup assertions were in place.
- 2026-07-30T00:00:00Z: Started track/scenarios on `track/scenarios`. Scope is new `nix/scenarios/*.nix`, small `flake.nix` wiring, and `crates/cix-index/tests/hammer.rs`; fenced from `crates/cix-cixfile` and `examples/build/**`. Current implementation is compose v0 plus D45 index tables, so the VM scenarios will prove its present behavior and carry D43/D44 frontier markers without adding runtime semantics. Next: factor the existing compose-stack fixture into hermetic VM scenario inputs, then add the ignored index concurrency hammer.
- 2026-07-30T00:00:00Z: Added five independently wired VM checks plus a shared hermetic two-service/Unix-edge fixture. Current-truth contracts: lifecycle proves HTTP over the Unix edge, selective restart, generation rollback, and state sentinel retention; side-by-side proves path-namespaced units/slices/runtime directories and loud host-bind collision; update-repin proves `track` moves only the tracked service and rollback restores its prior store path; gc-survival proves profile/current-table roots and callable history; observability proves journal, slice membership, and cgroup surfaces. FRONTIER markers: D43 — flip when `network: pod` allows identical internal ports without a bind conflict; D44 — flip when `--update <edge>` selectively repins nested composites. Exact intended repros: `nix build .#checks.x86_64-linux.scenario-lifecycle -L`, `nix build .#checks.x86_64-linux.scenario-side-by-side -L`, `nix build .#checks.x86_64-linux.scenario-update-repin -L`, `nix build .#checks.x86_64-linux.scenario-gc-survival -L`, `nix build .#checks.x86_64-linux.scenario-observability -L`, and `cargo test -p cix-index --test hammer -- --ignored` (green locally; < 2s). CI previously built only `vm-dogfood`, so `.github/workflows/ci.yml` now uses `nix flake check -L`, which includes this tier.
- 2026-07-30T00:00:00Z: VM verification remains not green: guest compose activation reaches systemd, but the scenario API with `StateDirectory=` fails under this nested VM with `status=226/NAMESPACE` / `Failed to allocate user namespace: Operation not permitted`. The scenario node now explicitly enables `kernel.unprivileged_userns_clone=1`; a rerun still did not complete before this journal entry. Do not claim the scenario builds or the full gate green until this guest isolation issue is resolved. Last focused failure repro: `nix build .#checks.x86_64-linux.scenario-lifecycle -L`.

- 2026-07-30T00:00:00Z: Completed the remaining post-fixture gate without
  repeating the already-proven lifecycle/side-by-side 3/3 forced-rebuild series
  (the fixture was unchanged after that evidence). Green: `nix build
  .#checks.x86_64-linux.scenario-update-repin -L --no-link`, `nix build
  .#checks.x86_64-linux.scenario-gc-survival -L --no-link`, and `nix build
  .#checks.x86_64-linux.scenario-observability -L --no-link`; each ran its VM
  test successfully. Also green: `cargo test -p cix-index --test hammer --
  --ignored` (four children, 1.35s maximum), `cargo fmt --all --check`, `cargo
  clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `cargo test --test tour -- --ignored generate_tour`, and `git diff
  --exit-code -- docs/tour` (zero tour diff). `git diff --check` is clean. Nix
  again emitted only its ignored eval-cache SQLite-busy warning during the GC and
  observability evaluations; both derivations completed successfully.

- 2026-08-02T14:27:00Z: Started `track/vmslim` from spec. Scope is strictly
  shared NixOS scenario VM plumbing (`nix/scenarios/lib.nix` and compatible
  shared imports); fenced from `crates/cix-run` and scenario assertions. Next:
  warm the cix package, identify the complete scenario set, then record a
  synchronous per-scenario baseline using `/usr/bin/time`.

- 2026-08-02T14:42:00Z: Baseline measurement in progress. `cix` was warmed
  successfully with `devenv shell -- nix build .#packages.x86_64-linux.cix
  --no-link -L`. The first two forced actual VM runs (the ordinary no-link
  test output is cacheable, so already-valid checks need `--rebuild` to
  execute) passed synchronously: lifecycle 196.56s; side-by-side 251.55s.
  Both expose the same dominant shared-harness cost: the non-cooperative db
  fixture waits for the NixOS manager's default 90s stop timeout on each
  `cix down`. Candidate to measure after the full baseline: a harness-only
  `DefaultTimeoutStopSec` override, along with documentation/profile knobs.

- 2026-08-02T15:15:00Z: Complete synchronous `/usr/bin/time` baseline and
  final sweep (all `devenv shell -- nix build
  .#checks.x86_64-linux.scenario-<name> --no-link -L`, one VM at a time):

  | scenario | before | after |
  | --- | ---: | ---: |
  | lifecycle | 196.56s | 106.94s |
  | side-by-side | 251.55s | 73.78s |
  | update-repin | 196.48s | 72.14s |
  | gc-survival | 179.60s | 81.92s |
  | observability | 156.55s | 55.71s |
  | devices | 141.68s | 46.72s |
  | health | 86.45s | 91.58s |
  | secrets | 76.25s | 71.35s |
  | dirs2 | 160.39s | 74.95s |
  | **total** | **1445.51s** | **675.09s** |

  The retained shared-harness change is
  `systemd.settings.Manager.DefaultTimeoutStopSec = "1s"`. It removes the
  irrelevant default 90s wait for deliberately non-cooperative fixture
  processes during scenario teardown, while preserving every assertion and
  all scenario semantics. The standalone lifecycle comparison was 196.56s to
  80.09s (59.3%). `documentation.enable = false` plus
  `system.switch.enable = false` was tried against health and was slower
  (116.26s versus 86.45s), so both were reverted. The final sweep was briefly
  interrupted by root filesystem exhaustion; `nix-store --gc --max-freed
  10737418240` synchronously reclaimed 10.2GiB of unrooted store artifacts,
  then the full final sweep above passed. Next: Rust/tour gates and commit.

- 2026-08-02T15:34:00Z: Gate and commit complete. Synchronous successes:
  `cargo fmt --all --check`; `cargo run -- fmt --check examples`; `cargo
  clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`;
  `cargo test --test tour -- --ignored generate_tour`; `git diff --exit-code
  -- docs/tour`; and `git diff --check`. The final timed scenario sweep was
  green for all nine checks (table above). Scenario-script diff is empty;
  only `nix/scenarios/lib.nix` changed. Committed as `7f792a7` (`Slim shared
  VM scenario teardown`). `nix/LOG.md` is intentionally left unstaged per
  the track-journal convention, although this repository currently tracks it.
