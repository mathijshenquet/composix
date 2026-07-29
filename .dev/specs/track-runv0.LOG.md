# track/runv0 work log

- 2026-07-29T15:54:55Z — Started D39 RUN v0 on `track/runv0`. Read `AGENTS.md`, the current session journal, authoritative D38/D39, the full spike README/report/harness, and the track spec. The worktree is clean; devenv tools (`cargo`, `nix`, `bwrap`, `pnpm`) are available. Initial implementation direction: preserve Cixfile’s existing Nix-generated final item, but make COPY/FETCH/RUN an explicit ordered build chain executed before Nix evaluation; materialize the chain’s final store snapshot as `${build}` during item generation. Next: map parser/codegen/lock seams precisely and record any spec-boundary friction before implementation.

- 2026-07-29T16:10:04Z — Implemented the first complete D39 slice: parser/model retain an ordered COPY/FETCH/RUN chain and `${build}` references with line-numbered position/context failures; Cixfile.lock accepts old input-only files and adds `fetches` + `memo`; memo keys cover directive+command, resolved offered closure, incoming NAR hash, and effective fixed/declared environment. The executor realizes declared package offers, mounts only their closure paths plus the writable workdir into strict bubblewrap PID/UTS/IPC/cgroup namespaces, gives RUN a fresh network namespace and FETCH host networking/DNS fixtures, clears the environment, and refuses rather than weakens namespace setup. A real-Nix integration test ran `RUN cp input output` through this sandbox and resolved `${build}` into the final manifest.

  First clean-lock `projB` build:

  ```text
  step 4 FETCH memo miss 03d9b7a74986 (152 ms) -> /nix/store/ip2qyxk4k1zhabwkqgwz1bcqqxbfazn9-cix-build-snapshot
  step 5 RUN memo miss f09bad3d3628 (1056 ms) -> /nix/store/7cmbky44sycgcf542n7ahsdmxc73807x-cix-build-snapshot
  /nix/store/ig5b88h0sbg7x24m559k8r9x7s832dbl-cixfile-item
  ```

  Immediate rebuild:

  ```text
  step 4 FETCH memo hit 03d9b7a74986 -> /nix/store/ip2qyxk4k1zhabwkqgwz1bcqqxbfazn9-cix-build-snapshot
  step 5 RUN memo hit f09bad3d3628 -> /nix/store/7cmbky44sycgcf542n7ahsdmxc73807x-cix-build-snapshot
  /nix/store/ig5b88h0sbg7x24m559k8r9x7s832dbl-cixfile-item
  ```

  The two final paths are identical. `projB-chef` also completed its recipe→FETCH→offline cook→source→final build chain at `/nix/store/lg4kbsi7fxd5kgw6y5i8w279c90krh88-cixfile-item`. Focused parser/memo tests and the external-RUN integration test pass. Next: prove chef source-edit selectivity, exercise FETCH pin mismatch/update behavior through the product path, then finish docs/tour and the full live gates.

- 2026-07-29T16:13:00Z — Chef selectivity proof passed. Changed only `examples/build/projB-chef/src/main.rs`, rebuilt, then restored the source. The recipe FETCH and offline cook reused their exact snapshots; only the post-source final build reran:

  ```text
  step 2 FETCH memo hit 1af99a469ca7 -> /nix/store/qf6kq1mwzxpmyn0a4g5r2dj8g0nllnsl-cix-build-snapshot
  step 3 RUN memo hit d00b87ba39bb -> /nix/store/qf6kq1mwzxpmyn0a4g5r2dj8g0nllnsl-cix-build-snapshot
  step 6 COPY src/main.rs -> src/main.rs snapshot /nix/store/d6n0badzq73apmg6qzz8as6hjpjwhyx5-cix-build-snapshot
  step 7 RUN memo miss 9cea54feb84b (891 ms) -> /nix/store/ga959vgbkjiicrixh9g63dgicdmvv4gr-cix-build-snapshot
  /nix/store/iksgyr784s1j2lxm2d3q1s3im0l5mb0j-cixfile-item
  ```

  This is D39’s COPY-scoping behavior: the source edit cannot alter the snapshot keyed into the cook step. The lock intentionally retains both final realizations; restoring the original source selects the original key again.

- 2026-07-29T16:16:13Z — Completed the documentation/tour/live slice. `docs/cixfile.md` now documents the linear chain, memo key, FETCH TOFU/update flow, fixed environment, `${build}`, no-tracer scope, and strict bubblewrap privilege refusal; docker.md flips RUN per D39; cixfile-build.md carries its required superseded banner. The Nix package wrapper now supplies `bwrap` and `nix`, and devenv declares bubblewrap. The generated `12-building-with-run.md` scenario is deterministic after normalizing wall time: it shows RUN miss `c307c177f29e` then hit with the same final item. Explicit regeneration plus `generated_tour_is_deterministic` pass. Live `examples/build/projB/demo.sh` passed: built from memo, started `cix-run-projb-18c6ced934ed65210.service`, curl returned `hello from RUN v0`, and the unit stopped cleanly. Existing nginx and PostgreSQL COPY-only Cixfiles also build through the final snapshot assembly.

- 2026-07-29T16:22:03Z — Required gate passes. `cargo fmt --all --check`, `cargo build --workspace`, warning-denied workspace all-target clippy, and `cargo test --workspace` all passed; the workspace suite includes parser/memo keying, source/locked-offer invalidation, FETCH mismatch/update guidance, memo-hit NAR revalidation, and a real bubblewrap RUN miss/hit. Explicit tour regeneration, tour determinism, and committed-page drift passed. The first VM invocation correctly exposed a flake-source mechanics issue—untracked `build_chain.rs` is omitted by Git flakes—so the complete in-scope tree was staged and the exact same `nix build .#checks.x86_64-linux.vm-dogfood --no-link` then passed. Final chef rebuild hit FETCH, cook, and final RUN and returned `/nix/store/lg4kbsi7fxd5kgw6y5i8w279c90krh88-cixfile-item`. Stopped/reset all system and user `cix-*` units; no lock temporaries or result links remain. Final hardening: missing memo outputs are requested from substituters before rerun, and any live memo store path is NAR-hash checked before reuse. Next: rerun the short gate after that last hardening, review staged content, and commit.

- 2026-07-29T16:27:00Z — Post-hardening final gate passed on the exact staged source: workspace fmt/build/clippy/tests; explicit tour regeneration and determinism; and `nix build .#checks.x86_64-linux.vm-dogfood --no-link`. No required work remains before commit.
