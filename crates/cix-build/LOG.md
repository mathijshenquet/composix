# cix-build work log

- 2026-08-01T00:00:00Z — Started track/underlay (D71). Reading the builder
  chain and its persistent-workspace state to replace prefix-reset replay with
  same-builder end-state underlaying, preserve `--cold` as empty/offline, add
  coverage and documentation receipts, then run the full flake gate.

- 2026-08-01T00:20:00Z — Implemented underlay-always: `--cold` is now the
  only path that allocates an empty builder workspace. Warm rebuilds retain the
  same project+builder workspace even when the changed suffix contains FETCH;
  `--update-lock` replays its complete builder in that underlay. Focused real
  Nix tests passed: `devenv shell -- cargo test -p cix-cixfile --test lock_nix
  warm_rerun_starts_on_its_builder_end_state_while_cold_does_not -- --exact
  --nocapture` and `devenv shell -- cargo test -p cix-cixfile --test lock_nix
  changed_step_before_fetch_reuses_its_builder_underlay -- --exact --nocapture`.
  The first proves a source-edit suffix sees its prior RUN append whereas cold
  reports the consumed-output difference.

- 2026-08-01T00:45:00Z — Re-ran
  `examples/compare/gitsitter/measure-warm.sh` with the just-built cix binary.
  `/tmp` was inode-starved during the first faithful attempt (`cargo vendor`
  reported ENOSPC), so the unchanged harness was re-run with only its
  disposable `mktemp` root relocated to ignored `target/`; its cleanup trap
  removed it. Receipt: upstream 28.32 s, crane 13.95 s, Cix 7.46 s. Updated
  only the Cix warm-edit matrix value and its dated receipt in `docs/nix-build.md`.
  Next: run the full required verification sequence, ending with the full
  flake check.

- 2026-08-01T01:05:00Z — The first workspace-test gate exposed a pre-existing
  torture-snapshot failure: `ParseError` rendered its excerpt separator as the
  literal characters `\\n`, while all committed snapshots use a real newline.
  Corrected the formatter (no diagnostic wording changed) and verified
  `devenv shell -- cargo test -p cix-cixfile --test diagnostics -- --exact`;
  the snapshots themselves remain byte-identical to HEAD. Re-running the full
  gate from fmt/clippy/workspace tests now.

- 2026-08-01T01:15:00Z — The second workspace gate reached `lock_nix` and
  exposed two more D72 migration omissions: real-Nix assertions expected
  manifest versions 4/5 while the current generated manifests are v0. Updated
  only those stale test expectations to 0. The underlay tests and all other
  `lock_nix` cases passed; restarting the workspace gate before cold-audit,
  tour, VM, and full-flake checks.

- 2026-08-01T01:30:00Z — The subsequent workspace test found the live
  user-run fixture had also missed D11's role-root restriction. It now
  declares `/var/lib/cix-run-integration-test` and verifies both the normal
  bind-mapped path and the documented mount-namespace-degraded fallback. Exact
  focused repro passed: `devenv shell -- cargo test -p cix-run --test user_run
  -- --exact --nocapture`. Restarting workspace verification once more before
  the required cold audit and remaining gates.

- 2026-08-01T02:05:00Z — Full track gate green. Exact successful repros:
  `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo clippy
  --workspace --all-targets -- -D warnings`; `devenv shell -- cargo test
  --workspace`; `devenv shell -- cargo test -p cix --test cold_audit --
  --ignored`; `devenv shell -- cargo test -p cix --test tour -- --ignored
  generate_tour`; `git diff --exit-code -- docs/tour`; `devenv shell -- cargo
  test -p cix --test tour tour_matches_committed_document -- --exact`; two
  runs of `devenv shell -- cargo test -p cix --test tour
  generated_tour_is_deterministic -- --exact`; `devenv shell -- nix build
  .#checks.x86_64-linux.vm-dogfood --no-link -L`; and, as the final gate,
  `devenv shell -- nix flake check -L`. Ready for final diff review and
  commit.

- 2026-08-01T02:25:00Z — Started track/leaks. Root cause confirmed from
  `tempfile` 3.27: `TempDir::Drop` ignores `remove_dir_all` errors. Our
  `copy_tree` deliberately preserves source directory modes, so a fetched
  `r-x` subtree makes removal fail after the D69 probe snapshots it. The
  snapshots are otherwise neither kept nor passed downstream; explicit
  permission-aware close plus Drop-path cleanup is required for both probe
  success and errors.

- 2026-08-01T02:40:00Z — Implemented a `FetchProbe` owner: explicit close
  recursively restores write bits and propagates removal failures; its Drop
  path applies the same cleanup for `?` error returns. Builder probes now
  explicitly close both snapshots after successful restoration. Added a CLI
  integration test that runs `--update-lock` under an isolated TMPDIR and
  asserts no probe directories remain after a read-only success fixture or a
  deliberately failing FETCH; changed the comparison harness to use TMPDIR.

- 2026-08-01T02:50:00Z — First focused test reached the fixture and correctly
  exposed that sandbox-root cannot write an existing mode-0555 directory.
  The success FETCH now temporarily restores its directory write bit, writes
  the payload, and returns it to mode 0555; the probe snapshot remains
  read-only for the cleanup assertion.

- 2026-08-01T03:00:00Z — Focused verification passed:
  `devenv shell -- cargo fmt --all --check` and `devenv shell -- cargo test
  -p cix --test fetch_probe_cleanup -- --exact --nocapture`. The latter ran
  the real CLI under TMPDIR and covered both requested outcomes. Diff review
  found no remaining hard-coded `/tmp` in `examples/compare/`; removed the
  untracked `devenv.lock` generated locally by devenv before starting the
  full required gate.

- 2026-08-01T03:15:00Z — Full gate green. Exact successful repros:
  `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo clippy
  --workspace --all-targets -- -D warnings`; `devenv shell -- cargo test
  --workspace`; `devenv shell -- cargo test -p cix --test
  fetch_probe_cleanup -- --exact --nocapture`; and, as the final required
  tier, `devenv shell -- nix flake check -L`. The latter built and ran the
  complete scenario/VM suite successfully. devenv regenerates an untracked
  lockfile in this worktree; remove that local byproduct before commit.

- 2026-08-04T09:20:00Z — Started track/fhspaths (CIP-95). Phase 1 is a
  standalone two-builder bubblewrap repro: manufacture GNU- and musl-linked
  FHS ELFs in one root, then execute them in fresh roots containing only the
  declared libc IMPORT surface. The load-bearing comparison is no cache vs
  `LD_LIBRARY_PATH=/lib` vs a generated `/etc/ld.so.cache`; a Nix
  RUNPATH-carrying bash in the same root must keep resolving its closure from
  its own RUNPATH. No phase-2 production wiring until this verdict is recorded.

- 2026-08-04T09:40:00Z — **Phase-1 verdict: STOP — the GNU `/lib` wiring is
  not clean.** Synchronous receipt: `bash .dev/spikes/fhs-paths/repro.sh` exited
  0 after proving all expected outcomes. The `/lib64/ld-linux-x86-64.so.2`
  alias executes an RPATH-free FHS ELF with only glibc imported, but only
  because Nix's loader falls back to its own glibc store `lib/`; a second
  RPATH-free ELF needing `libcix-fhs-probe.so.1` from the `/lib` union fails.
  `ldconfig -C /etc/ld.so.cache -f /etc/ld.so.conf -X` creates a valid union
  cache, but the loader ignores it: Nix glibc hardcodes
  `/nix/store/<glibc>/etc/ld.so.cache`. Making that cache visible would require
  shadowing the imported package's immutable store-path view, and would not
  transfer cleanly to artifact runtime namespaces. `LD_LIBRARY_PATH=/lib`
  makes the missing SONAME case run, but `LD_DEBUG=libs` proves it changes a
  Nix RUNPATH-carrying bash's libc resolution from its store RUNPATH to
  `/lib/libc.so.6`; this is the prohibited shadowing. The musl variant is green
  because musl's default search path includes `/lib`. Per CIP-95's fallback
  boundary and the track spec, phase 2 was not started and no auto-patching
  fallback was improvised.

- 2026-08-04T09:47:00Z — Pulled `origin/main` and read the adopted CIP-95
  post-spike amendment. Phase 2 resumes on its narrowed v1 contract: an
  always-present, skeleton-versioned GNU+musl alias pair backed by only the
  matching loader file from ordered IMPORTs; no `lib/` union or default
  library search path. Failure-only trace facts will feed (E): workdir
  execve-ENOENT, loader-path lookups, and missing SONAME opens, correlated
  with the executed ELF and the imported loader provider. Missing libc gets
  an IMPORT hint; dependencies outside that libc get an explicit aliases-only
  boundary and the taught patchelf escape.

- 2026-08-04T09:51:00Z — Implemented the narrowed loader-only skeleton as a
  separate `fhs` stratum. On x86_64 the fixed aliases point at an internal
  loader bridge populated from the first matching ordered IMPORT; absent
  glibc/musl leaves the target dangling. The skeleton fingerprint is v2, and
  the ordinary IMPORT union remains exactly `bin/etc/share`. Synchronous
  receipts: `devenv shell -- cargo test -p cix-build fhs::tests -- --nocapture`
  (2 passed), and `devenv shell -- cargo test -p cix-cixfile --test lock_nix
  fhs_glibc_and_musl_elfs_run_from_loader_aliases_without_cixfile_fixups --
  --exact --nocapture` (1 passed). The latter manufactures RPATH-free GNU and
  musl FHS ELFs in Nix, then executes both in real fresh Cix builders whose
  Cixfile contains zero patchelf lines.

- 2026-08-04T09:57:00Z — Implemented (E) without persisting negative trace
  state: only a failing step reparses its temporary syscall file for workdir
  execs/execve-ENOENT, known FHS loader misses, and failed SONAME opens. A
  small in-tree ELF reader correlates PT_INTERP + DT_NEEDED with the ordered
  imported libc provider. The missing-loader report names the binary, loader,
  needed libc SONAMEs, and `IMPORT ${pkgs.glibc}`/`${pkgs.musl}`; an imported
  loader plus an unresolved non-libc DT_NEEDED instead says plainly that v1
  has no `/lib` search path and points at `IMPORT ${pkgs.patchelf}` plus the
  taught RUN escape. Focused trace/FHS unit tests passed. Real-Nix diagnostic
  receipts passed synchronously with `/tmp`-inode-safe invocations:
  `env TMPDIR=$PWD/target/fhspaths-tmp devenv shell -- env
  TMPDIR=$PWD/target/fhspaths-tmp RUSTC_WRAPPER= cargo test -p cix-cixfile
  --test lock_nix missing_fhs_loader_diagnostic_suggests_the_libc_import --
  --exact --nocapture`, the analogous
  `beyond_libc_diagnostic_names_the_alias_boundary_and_patchelf_escape`, and
  warning-denied focused clippy for cix-build+cix-cixfile. Host `/tmp` has
  free bytes but zero free inodes; no shared entries were removed.

- 2026-08-04T10:06:00Z — Narrowed Directus acceptance passed in a disposable,
  ignored copy of the pinned corpus case. The only Cixfile change was adding
  `${pkgs.glibc}` to the builder IMPORT; it contains no patchelf command. The
  downloaded `sass-embedded-linux-x64` Dart executable remained an x86-64 ELF
  with PT_INTERP `/lib64/ld-linux-x86-64.so.2`. Synchronous receipt:
  `env TMPDIR=$PWD/target/fhspaths-tmp CIX_STATE_DIR=$PWD/target/fhspaths-directus-state
  CIX_BUILD_WORKSPACE_DIR=$PWD/target/fhspaths-directus-workspaces timeout 1200
  target/debug/cix build --update-lock build target/fhspaths-directus#directus`
  exited 1 only after Sass built the app asset and the monorepo completed its
  package builds, at the already-known separate `Error: Not a directory (os
  error 20)`. The former Sass loader `spawn … ENOENT` is absent. No corpus
  Cixfile was modified; next is documentation and ledger currency.

- 2026-08-04T10:18:00Z — The broad gate exposed an existing nondeterminism in
  `socket_filter_is_accepted_by_bubblewrap`: it selected the first
  `/nix/store/*/bin/bash`, which can now be a cix-item symlink whose target is
  not a reference in that item's NAR-added closure. The test now canonicalizes
  the selected executable before deriving the offered package, matching its
  intent independently of store directory order. Exact focused test and fmt
  check passed. The first broad-gate retries also exhausted filesystem bytes;
  only this task's disposable Directus workspaces and local Cargo target were
  removed, while its acceptance log was retained. Further Cargo work uses the
  task-owned `/dev/shm/composix-fhspaths-20260804` target; no shared Nix garbage
  was collected.

- 2026-08-04T10:31:00Z — Final track gate is green on the committed tree.
  Synchronous exit-0 receipts: `cargo fmt --all --check`; `cargo run -- fmt
  --check examples`; `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo test --workspace`; corpus regeneration followed by `git diff
  --exit-code -- docs/corpus`; tour regeneration followed by `git diff
  --exit-code -- docs/tour` and `cargo test -p cix --test tour -- --nocapture`;
  `nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`; and `nix build
  .#checks.x86_64-linux.scenario-closedroot-audit --no-link -L`. Cargo and VM
  commands ran through `devenv shell` with `TMPDIR` (and Cargo target where
  relevant) under `/dev/shm/composix-fhspaths-20260804`; the two focused VM
  checks completed their real TCG guests in 164s and 274s respectively. The
  full flake matrix was deliberately not run: project policy reserves it for
  the orchestrator's independent pre-merge gate.

- 2026-08-04T10:56:25Z — Started track/cip94 milestone 1 from `origin/main`
  (`1d599dc`). The binding outcome is a pure `nix/lib` eval-from-lock
  `buildCixfile` whose pure-assembly and one-builder FETCH+RUN fixtures are
  byte-identical to synchronous `cix build --cold` results. FHS-consuming
  builders are a loud eval-time boundary; multi-builder and runtime-manifest
  behavior may be cut only with explicit errors. First step: map the Rust
  cold builder, lock schema, and flake check conventions before choosing the
  smallest shared skeleton representation.

- 2026-08-04T11:08:00Z — Added the pure-eval lock substrate. Each FETCH now
  records the NAR hash of its complete immediate post-step workspace
  (`snapshotNarHash`), which is the missing hash required to model exactly one
  fixed-output derivation per FETCH; locks also record the dev-env snapshot
  selected by each builder. A versioned, Cixfile-content-bound `evalPlan`
  serializes the resolved parser AST rather than duplicating the Cixfile parser
  in Nix. Existing locks remain readable and ordinary cold builds merely omit
  the plan with a loud note until their FETCH pin is refreshed. Focused Rust
  receipt (44 tests): `env TMPDIR=/dev/shm/composix-cip94-20260804/tmp
  CARGO_TARGET_DIR=/dev/shm/composix-cip94-20260804/target cargo test -p
  cix-build -p cix-cixfile --lib` exited 0. Host `/dev/md3` briefly reached
  ENOSPC and rustfmt truncated `build_chain.rs`; the zero-byte file was caught
  immediately, restored byte-for-byte from HEAD, and the isolated edits were
  reapplied before the green receipt. All further build state stays in
  `/dev/shm`.

- 2026-08-04T11:51:59Z — Implemented `buildCixfile` under `nix/lib` and
  exposed it through both `?dir=nix/lib` and the root flake `lib`. Milestone 1
  replays pure assembly and one BUILDER with one FOD per FETCH, normal offline
  RUN derivations, the recorded development environment, and the shared
  skeleton fingerprint. Explicit cuts are top-level FETCH, artifact-valued
  FROM, multi-BUILDER graphs, SERVICE/APP outputs, and builders importing the
  CIP-95 FHS loader providers; each has a named eval-time error. The focused
  NixOS VM check ran synchronously in TCG and exited 0 after 50.15s: its
  network-offline guest compared real `cix build --cold` outputs to the Nix
  library derivations and observed matching assembly NAR
  `sha256-ZRz7n5+saF91Ur/koP/FtpND17AtJ8/NlE0Fa5Lsg8I=` and builder NAR
  `sha256-mJCuDVcxbt6bxVjHyNyPqCYcELx4S5QtKteXuQnrSmk=`; it also confirmed the
  CIP-95 rejection. Exact command: `env
  TMPDIR=/dev/shm/composix-cip94-20260804/tmp nix build
  .#checks.x86_64-linux.build-cixfile-byte-identity --no-link -L`.

- 2026-08-04T11:59:51Z — Final agent tier is green. With command prefix
  `nice -n 10 env TMPDIR=/dev/shm/composix-cip94-20260804/tmp
  CARGO_TARGET_DIR=/dev/shm/composix-cip94-20260804/target`, synchronous
  exit-0 receipts were: `cargo fmt --all --check`; `cargo run -j 6 -- fmt
  --check examples`; `cargo clippy -j 6 --workspace --all-targets -- -D
  warnings`; `cargo test -j 6 --workspace`; `cargo test -j 6 -p cix --test
  tour -- --ignored generate_tour`, then `git diff --exit-code -- docs/tour`
  and `cargo test -j 6 -p cix --test tour -- --nocapture`. Nix receipts:
  `nix flake show ./nix/lib --no-write-lock-file`; `nix eval
  .#lib --apply builtins.attrNames --json` (result:
  `["buildCixfile","withSpec"]`); and `nice -n 10 nix build
  .#checks.x86_64-linux.with-spec-redis --no-link -L --max-jobs 6 --cores 4`.
  The final focused receipt on committed head `ff81690` was `nice -n 10 env
  TMPDIR=/dev/shm/composix-cip94-20260804/tmp nix build
  .#checks.x86_64-linux.build-cixfile-byte-identity --no-link -L --max-jobs 6
  --cores 4`: synchronous exit 0, real TCG guest in 46.66s. The first tour
  generator attempt encountered a transient Cargo cache SQLite I/O error; its
  clean-workspace retry and the subsequent full tour harness both exited 0.
  Per project policy, the full flake matrix is reserved for the orchestrator's
  independent pre-merge gate and was not run here. No corpus or migration
  ledger files were touched, as required by the track fence.

- 2026-08-04T12:15:00Z — Started track/buildfixes at `8420a95`. Scope is the
  three verified build-side defects from `.dev/specs/track-buildfixes.md`:
  reproduce then fix warm EXPECT-versus-lock validation (including the
  identical-pin recording path), context-free builder I/O errors from the
  fhsspike Directus shape, and warm-workspace duplicate COPY rejection after a
  plan edit. Corpus files, especially the traefik living repro, remain
  untouched. The worktree is clean, `track/buildfixes` is active, and direnv's
  devenv is loaded. Next: make scratch copies of the three recorded cases and
  capture synchronous pre-fix receipts.

- 2026-08-04T12:52:00Z — Captured all three pre-fix defects without touching
  corpus files. Traefik scratch replay showed the volatile live-fetch control
  failing its first EXPECT and its copied lock independently proves both FETCH
  pins contain the same declared narHash despite distinct stepMemo writes; code
  tracing found `install_declared_expectations` mutates those pins before
  verification. Watchtower scratch: initial ordinary build exited 0; adding one
  overlapping direct go.mod COPY after FETCH also exited 0; adding the recorded
  second direct go.mod COPY then exited 1 at parse time with `line 10: BUILDER
  block destination "go.mod" is already populated`. Directus scratch was fetched
  through the unchanged `corpus/migrate/fetch.sh`, paired with track/fhsspike's
  Cixfile and 148171-line lock, and run synchronously: every pnpm workspace
  package completed its build, then cix exited 1 with only `Error: Not a
  directory (os error 20)`. Timing locates that ENOTDIR before post-RUN read-set
  metrics, inside trace dependency recording. Next: add path+step context to
  expose the exact node, then decide whether ENOTDIR is a narrow transient-path
  case that should record `Absent`.

- 2026-08-04T15:54:49Z — Implemented the three narrow product fixes. Declared
  EXPECTs are compared with the exact command-derived lock pin before completed
  output reuse and again after builder-context resolution before any step memo;
  mismatches retain the recorded pin and name the source line plus declared and
  locked values. The two-FETCH Traefik-shaped regression has two distinct
  workspace NARs and proves a copy-pasted second EXPECT fails with warm step
  memos present. Directus instrumentation identified
  `node_modules/@popperjs/core/dist/cjs/popper.js/index.d.ts`: `popper.js` is a
  regular file, so the child probe's ENOTDIR is semantically the same absent
  dependency as ENOENT. The recorder now handles that case and adds path and
  Cixfile-step context to all other surfaced trace I/O errors. A synchronous
  post-fix scratch replay crossed the former line-16 `pnpm run build` boundary
  and reached line 17's separate networkless deploy command before its expected
  `EAI_AGAIN`; the corpus checkout remained unchanged. Finally, builder COPY
  destinations are sequential while artifact assembly destinations remain
  unique; the warm-workspace regression reuses the old prefix, overwrites the
  repeated destination, and executes the edited suffix. Focused synchronous
  receipts are green: the ENOTDIR unit test, the parser test, both real-Nix
  `lock_nix` regressions, all 45 cix-build/cix-cixfile library tests, and all 32
  parser integration tests. Next: split logical commits, then run the complete
  agent gate and focused VM scenario.

- 2026-08-04T16:03:02Z — Final agent tier is green on the finished source.
  Synchronous exit-0 receipts: `cargo fmt --all --check`; `cargo run -- fmt
  --check examples`; `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo test --workspace`; `cargo test -p cix --test corpus -- --ignored
  generate_corpus_browser`, followed by the workspace corpus drift and
  determinism tests; `cargo test -p cix --test tour -- --ignored generate_tour`;
  `git diff --exit-code -- docs/tour`; and the exact committed-tour drift test.
  The final focused receipt, `nix build
  .#checks.x86_64-linux.vm-dogfood --no-link -L`, built cix from this dirty
  Git source and exited 0 after its real TCG guest completed in 160.40s and
  removed every cix unit and GC root. `git diff --exit-code -- corpus` also
  exits 0: no migration case, lock, receipt, or GAPS file changed; generated
  `docs/corpus/` pages changed only because the required ledger ribbons did.
  The full flake matrix remains reserved for the orchestrator's independent
  pre-merge gate. Ready for final documentation commit and handoff.
