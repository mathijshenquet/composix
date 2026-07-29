# track/spike-runtrace work log

## 2026-07-29T13:20:28Z — start and spike design

- Read `AGENTS.md`, `.dev/LOG.md`, the track spec, and authoritative D38. The worktree is
  clean on `track/spike-runtrace`, and direnv reports the repository environment active.
- Available primitives are Nix, bubblewrap, strace, unshare, and sudo. A bubblewrap smoke
  test successfully created a store-only root with a new network namespace; the first probe
  failed only because the deliberately empty root had no `/bin/true`, then passed when the
  Nix bash executable was explicitly linked into `/bin`.
- Chosen tracing design: run `strace -f -z -yy` *inside* a bubblewrap sandbox, tracing
  `%file`, process, and mmap syscalls. `-z` drops failed probes; `-yy` resolves mapped/open
  file descriptors. Reduce every successful `/nix/store/...` access to its top-level store
  path. Known blind spots to measure and report: strace does not observe the tracer's own
  reads, it is syscall/path observation rather than page-level provenance, and successful
  metadata probes are conservatively treated as dependencies even when their result may not
  affect the build.
- The generic harness will know only sandbox, explicit environment, offered store closures,
  read-only dependency directories, optional prior-output seeding, tracing, output hashing,
  and memo validation. Ecosystem commands and prefetch logic stay in their example
  directories. In addition to D38's store read-set, the prototype will hash the writable
  source snapshot and read-only dependency directories: without that generic non-store input
  fingerprint, a source edit could incorrectly hit, contradicting the spec's miss test.

## 2026-07-29T13:35:00Z — generic harness smoke-tested and inputs prefetched

- Implemented `harness/runtrace` and proved its miss → memo hit path with a generic `cp`
  command. The traced run took 8 ms, reduced 8 store paths (bash, coreutils, glibc and
  runtime libraries), produced a NAR hash/store output, and the identical invocation hit
  without creating its requested output directory.
- The smoke test rejected host `/usr/bin` binaries as offers; fetched bash, coreutils, and
  strace from nixpkgs and reran successfully. This is desired: the sandbox sees only the
  read-only Nix store plus its declared writable/bound paths.
- Added all four small projects and their per-example networked prefetch scripts. Prefetch
  completed for Cargo crates, Go vendor content, pnpm's offline store, and uv's cache, and
  generated `Cargo.lock`/`recipe.json`, `go.sum`, `pnpm-lock.yaml`, and `uv.lock`.
  Toolchains are nixpkgs outputs: Rust 1.96.1/cargo-chef 0.1.77, Go 1.26.4, Node
  24.18.0/pnpm 11.15.0, and Python 3.12.13/uv 0.11.28.
- Preparation exposed only host/dev-environment issues, not harness ecosystem coupling:
  the nested Rust fixture needed an empty `[workspace]`, inherited `RUSTC_WRAPPER=sccache`
  had to be removed during network prefetch, Go needed `mod tidy` before vendoring, and
  pnpm's self-version switch was avoided by not pinning a mismatched package-manager version.
  All such logic remains in the relevant example.

## 2026-07-29T13:55:00Z — first offline trials and harness corrections

- Offline traced success so far: real `cargo chef cook` (4.769 s, 37 store paths), Go build
  (2.351 s, 5 paths), pnpm install+esbuild (0.790 s, 34 paths), and uv sync plus import
  proof (successful trial; full timing rerun pending).
- Rust additionally needed nixpkgs GCC on PATH. pnpm 11 needed three project-level facts:
  `--trust-lockfile` to stop its new supply-chain re-verification from querying registry
  metadata despite `--offline`, an `allowBuilds.esbuild=true` workspace setting, and Nix
  `gnused` for esbuild's generated launcher. No pnpm branch entered the harness.
- Both pnpm 11's SQLite store and uv's cache require writes during an offline install; a
  read-only bind fails (`unable to open database file` / uv lock temporary-file `EROFS`).
  The working model is a fresh writable copy of the prefetched cache in the writable input
  snapshot. Cargo's prefetched home works read-only; Go consumes a copied vendor tree.
- The first automated matrix exposed two generic harness bugs before evidence was accepted.
  Seeded Nix outputs retain read-only directory modes, so generic seeding must make the copied
  output writable before a compositional next step. More importantly, the initial jq
  fingerprint expression accidentally hashed only positional arrays and discarded command
  and workdir named arguments, making source-edit calls hit. Corrected both command and
  request fingerprints to include their named fields; the miss matrix will be rerun from
  scratch.

## 2026-07-29T14:10:00Z — final measurement matrix complete

- Reran the complete matrix from fresh snapshots after the generic harness corrections.
  Every ecosystem passed offline, every forced repeat produced a set-identical traced store
  closure, every identical request produced a real memo hit, and every source edit changed
  the request hash, reran/retraced, and changed the output hash. Store path sets after source
  edits remained identical, as expected because source paths are deliberately outside the
  store closure.
- Exact results: cargo-chef cook 37 paths, 6.037/5.295 s; Rust release build 37 paths,
  4.817/4.930 s traced vs 2.915 s untraced; Go 5 paths, 2.786/2.499 s vs 1.790 s; pnpm
  34 paths, 0.980/1.022 s vs 0.768 s; uv 13 paths, 0.567/0.527 s vs 0.326 s. Ecosystems
  ran concurrently, so these are rough overhead measurements rather than benchmarks.
- Output hashes: Go, pnpm, and the composed Rust release output are identical across the
  two traced runs. Cargo cook is not: `diff -qr` shows only Cargo incremental
  session/lock directory names differ. uv is not: `uv_cache.json` records nanosecond
  timestamps (and changes `RECORD`), while `_virtualenv.cpython-312.pyc` embeds a
  one-second-varying source timestamp.
- D38 promotion gate verdict: PASS. A real `cargo chef cook --recipe-path recipe.json`
  traces to an exactly stable 37-path closure across forced runs. This does not waive the
  broader determinism/productization problems; the report distinguishes the narrow gate
  from cache readiness.
- Wrote `.dev/spikes/run-trace/REPORT.md` with the a–f × four-ecosystem matrix, method,
  non-store noise counts, surprises, tracer blind spots, and three productization blockers.
  Compact JSON summaries and exact store path sets are retained under `results/`; raw
  strace logs remain ignored.

## 2026-07-29T14:20:00Z — final verification ready for commit

- Verification passed: `bash -n` over the harness and all prepare/measure scripts; `jq -e`
  over all four result summaries and assertions for the expected stable/unstable outputs,
  hits, and source-edit misses; `cmp` over every pair of forced-repeat store path sets; and
  an ecosystem-term scan of `harness/` (no cargo/Rust/Go/pnpm/Node/Python/uv knowledge).
- Confirmed the harness contains `--unshare-net`, `--clearenv`, offered-closure expansion,
  and `nix store add`; no raw trace, `.prepared`, `work`, or `node_modules` artifact is
  staged. `git diff --cached --check` passes and every staged path is under
  `.dev/spikes/run-trace/` or this required track log. No product crate or
  `docs/design.md` changed.
- The committed measurement scripts can rerun in place and update compact results only
  after all fresh runs complete. Exact reproduction is: run each
  `examples/<ecosystem>/prepare.sh` (networked prefetch), then each `measure.sh`; the four
  final measurement drivers were run concurrently for this report.
