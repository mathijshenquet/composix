# D38 spike report: traced `RUN` across Rust, Go, pnpm, and uv

## Verdict

**D38's explicit promotion gate passes.** Two forced, clean executions of the real command
`cargo chef cook --recipe-path recipe.json` each read the same 37 top-level Nix store paths.
The sets are byte-for-byte identical, and an invocation with the same request and offers
hits the memo.

That is a positive result for the *read-closure* hypothesis, not yet for a production cache.
The cook output itself is not deterministic: Cargo's incremental directory names contain a
per-build session/time component. The following `cargo build --release`, seeded from the
cook output, is byte-identical across runs. uv also produces a nondeterministic environment.
D38's sampled-rebuild condition therefore has real work to do even though its stated
closure-stability gate is satisfied.

## Setup

All commands ran under the same `harness/runtrace` program:

- bubblewrap root containing the read-only `/nix/store`, a writable `/work` and `/out`,
  minimal `/proc`, `/dev`, `/tmp`, passwd/group, and a new PID/UTS/IPC/cgroup/network
  namespace;
- `--clearenv`, then only generic fixed values plus the example's explicitly supplied
  environment and offered-tool PATH;
- `strace -f -z -yy -e trace=%file,%process,mmap,mmap2` inside the sandbox;
- successful store accesses reduced to `/nix/store/<hash>-<name>`;
- output hashed as a NAR with `nix hash path --mode nar`, then added with
  `nix store add --mode nar`;
- memo fields for command/environment hash, generic source/dependency input hash, traced
  store set, NAR output hash, output store path, and wall time. A hit also requires every
  prior traced path to remain in the Nix closure of the current offers.

`prepare.sh` in each example used the network outside the sandbox. This models a future
lock-derived fixed-output fetch; no sandboxed `RUN` had network. Measurements used
Rust/cargo 1.96.1 + cargo-chef 0.1.77, Go 1.26.4, Node 24.18.0 + pnpm 11.15.0, and Python
3.12.13 + uv 0.11.28 from nixpkgs.

Wall times are rough single samples from the final matrix, whose ecosystems ran in
parallel. They are useful for order-of-magnitude overhead, not benchmarking.

## Matrix (a–f)

| Test | Rust via cargo-chef | Go | pnpm | uv |
| --- | --- | --- | --- | --- |
| **a. Offline sandbox succeeds** | **Yes.** Cook then `cargo build --release`; needs prefetched Cargo home, Rust/cargo/cargo-chef, GCC/binutils. The build generically seeds a writable copy of the cook output. | **Yes.** Vendored `github.com/google/uuid`; `GOFLAGS=-mod=vendor`, `GOPROXY=off`, `GOSUMDB=off`, `CGO_ENABLED=0`. | **Yes.** Writable copy of prefetched store, `--offline --frozen-lockfile --trust-lockfile`, project `allowBuilds.esbuild=true`, Node/pnpm/sed. Without `--trust-lockfile`, pnpm 11 still queried registry metadata for supply-chain re-verification and stalled on the absent network. | **Yes.** Writable copy of prefetched uv cache, Nix Python selected explicitly, downloads disabled, `uv sync --offline`, then an import/invocation proof. |
| **b. Store closure stable** | **Yes.** Cook: 37 = 37 identical. Build: 37 = 37 identical, including the seeded cook output instead of cargo-chef. No changing store noise. | **Yes.** 5 = 5: Go, bash, glibc, readline, ncurses. | **Yes.** 34 = 34: Node/pnpm/sed and their runtime libraries. | **Yes.** 13 = 13: Python/uv and runtime libraries. |
| **c. Output byte-stable** | **Cook: no. Build: yes.** Cook differs only in incremental session directory/lock names such as `s-hkuife…` vs `s-hkuifh…`. Final build NAR hash is identical. | **Yes.** Identical binary NAR hash. | **Yes.** Identical bundled `app.js` NAR hash. | **No.** `uv_cache.json` embeds nanosecond source/build timestamps; its hash changes `RECORD`. `_virtualenv.cpython-312.pyc` also embeds a one-second-varying timestamp. |
| **d. Input change misses** | **Yes.** Editing `value: 38 → 39` changes the generic work hash, misses, reruns the release build, retraces, and changes the output hash. | **Yes.** `D38 → D39` changes request and output hashes. | **Yes.** `38 → 39` changes request and bundle hashes. | **Yes.** `/d38 → /d39` changes request and environment hashes. |
| **e. Harness purity** | **Yes.** No Rust branch. | **Yes.** No Go branch. | **Yes.** No pnpm branch. | **Yes.** No uv branch. |
| **f. Tracing overhead** | Representative build: 4.930 s traced vs 2.915 s untraced, **1.69×** (+2.015 s). Cook traced samples: 6.037/5.295 s. | 2.499 s vs 1.790 s, **1.40×** (+0.709 s). | 1.022 s vs 0.768 s, **1.33×** (+0.254 s). | 0.527 s vs 0.326 s, **1.62×** (+0.201 s). |

Every identical invocation also produced a real memo `hit`. Each source edit produced a new
request hash and a traced `miss`. The source edits do not change the store path sets—the
edited sources and ecosystem caches live outside `/nix/store` by definition—but the traces
were rerun and the memo/output hashes were replaced.

## What appeared outside the store

These are successful path-operation counts from the first traced runs, not unique paths:

| Command | `/work` | `/out` | `/deps` | `/proc` | `/tmp` | `/etc` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| cargo chef cook | 94 | 2,079 | 953 | 157 | 117 | 56 |
| cargo build | 68 | 2,170 | 953 | 139 | 117 | 50 |
| go build | 489 | 6 | 0 | 385 | 6,411 | 1 |
| pnpm install + build | 330 | 2 | 0 | 56 | 33 | 4 |
| uv sync + invocation | 3,280 | 1,673 | 0 | 12 | 12 | 1 |

The large Go `/tmp` count is its fresh `GOCACHE`. pnpm and uv initially received their
prefetch data at `/deps` read-only; both failed because offline operation still writes
SQLite/cache lock state. Their successful runs use a fresh writable cache snapshot under
`/work`, so those reads are counted there. Rust can use its prefetched Cargo home read-only.
`/proc`, `/tmp`, and `/etc` accesses are excluded from the store closure.

## Surprises

1. The flagship closure is calmer than its output. Cargo chef's entire 37-path store set is
   stable while Cargo incremental bookkeeping alone changes the output NAR.
2. “Offline” is not equivalent to “read-only dependency cache.” pnpm and uv both need
   writable cache metadata. pnpm 11's supply-chain policy pass also performs registry
   metadata requests despite `--offline`; trusting the already-prefetched lock is required
   for a truly networkless run with this version.
3. Store closure changes are the wrong signal for ordinary source edits. Source changes
   must still invalidate the memo even though the traced *store* set correctly stays fixed.
   The prototype therefore hashes the whole work/dependency snapshot before lookup. That is
   sound but over-invalidates unread files.
4. Composition works, but only after treating a prior immutable output like a lower layer:
   copy it into the new writable output and restore owner-write bits. The second trace then
   includes the prior output's store path generically.

## Tracer and prototype limits

- `strace` does not trace its own startup reads. It follows command children/threads, but
  this is syscall path observation, not proof that mapped/read bytes influenced output.
- Successful metadata probes are conservatively dependencies; failed probes are excluded
  by `-z`. This can over-approximate, while unusual kernel I/O interfaces need separate
  validation.
- The full store is visible on a first run. The prototype detects an ambient, unoffered
  read and refuses a later hit because it is absent from the offered closure; it does not
  prevent that first read.
- The pre-run whole-snapshot hash makes source/dependency changes sound, but sacrifices the
  hypothesis's desired observed granularity for mutable inputs.
- Output timings include bubblewrap and strace startup, and the runs were not isolated from
  host load.

## Sharpest three productization problems

1. **Define all non-store inputs without rebuilding the shim industry.** A product needs
   precise generic tracking for source trees, lock-derived mutable cache snapshots,
   environment, and prior outputs. Whole-tree hashing is safe but makes unread edits miss;
   store-only tracing is insufficient by itself.
2. **Replace prototype tracing with a sound, supportable observer.** Decide how
   fanotify/FUSE/eBPF observes open/exec/mmap/stat across process trees, how it handles
   ambient unoffered reads, and how conservative metadata reads should be. The trace must
   be auditable enough to sign and share.
3. **Specify writable layers and nondeterministic realizations.** cargo-chef and uv already
   produce multiple content hashes for identical request/closure keys, while pnpm/uv need
   writable caches and composed Rust builds need a writable copy-up. Product behavior needs
   overlay/copy semantics, sampled-rebuild quarantine, and a clear policy for multiple or
   rejected realizations.

Compact machine-readable evidence lives in `results/*/summary.json`; the exact stable path
sets and non-store summaries are beside it. Raw strace logs are intentionally excluded.
