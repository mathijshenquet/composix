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
