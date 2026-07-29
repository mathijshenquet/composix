# track/runv0 — RUN v0 + FETCH in the Cixfile (D39)

Read AGENTS.md first. Authoritative design: docs/design.md **D39** (with D38 for the
underlying model and `.dev/spikes/run-trace/` for empirical groundwork — reuse its
learnings, not necessarily its code). D39 wins on conflict. This track supersedes
track-buildtool.md (do not resurrect BUILD).

## Surface (parser: crates/cix-cixfile)

Two new directives, positionally between the FROM/PATH/COPY prelude and SERVICE blocks:

    FETCH <command…>       # network allowed; fixed-output; hash pinned in Cixfile.lock
    RUN   <command…>       # no network, offer-only sandbox, memoized

- Steps (COPY/FETCH/RUN) form a LINEAR chain: each step receives the workdir produced by
  the chain so far (COPY semantics unchanged — it stages sibling files into the workdir).
- `${build}` in EXEC/LINK/PATH/SERVICE context = the store path of the final step's
  workdir snapshot. No per-step naming in v0 (no AS) — keep the surface minimal.
- Line-numbered errors in the established style (quoted source, position).

## Execution (cix build orchestrates; steps run OUTSIDE nix eval)

For each step, in order:
1. Compute the memo key: hash(command + offered-closure set + incoming workdir snapshot
   hash + declared env). Look up in the memo section of `Cixfile.lock`. Hit → reuse the
   recorded output snapshot (must still exist / be substitutable; else re-run).
2. Miss → run sandboxed:
   - Mounts: ONLY the offered closure — the nix closures of every FROM/PATH-referenced
     store path (`nix path-info -r` at generation) — read-only at their real store paths;
     a writable workdir seeded from the incoming snapshot; fresh PID/UTS/IPC/cgroup
     namespaces; `--clearenv` plus the repro-env defaults (SOURCE_DATE_EPOCH=1, TZ=UTC,
     LC_ALL=C, umask 022, HOME=<workdir>) plus Cixfile ENV values.
   - RUN: network namespace with loopback only. FETCH: host network.
   - Sandbox tech: bubblewrap (as proven in the spike) or unshare-based — your call;
     document the privilege story (unprivileged userns where allowed; the D36-style loud
     degraded/refusal path where the host restricts userns — refusal is acceptable for v0,
     silent weakening is not).
3. After the step: snapshot the workdir into the store (`nix store add`, NAR), record in
   the memo: key, output NAR hash, store path, wall time.
4. FETCH additionally: on first run, pin its output hash in `Cixfile.lock` (TOFU); on
   subsequent runs with an existing pin, VERIFY the refetched output matches the pin and
   fail loudly on mismatch (`--update-lock` re-pins deliberately, following the existing
   lock conventions).
- No tracer in v0 (D39.4). Do not add strace machinery; leave a clean seam where a
  pruning observer can slot in later.

## The proof: examples/build/projB (single-step) and the chef pattern

- `examples/build/projB`: minimal single-binary rust service —
  COPY manifests+src, `FETCH cargo fetch`, `RUN cargo build --release`,
  `EXEC ${build}/target/release/<bin>`, one declared port. Must `cix build` → `cix run` →
  curl.
- Also add the two-step chef variant as `examples/build/projB-chef` (COPY recipe →
  FETCH → RUN cook → COPY src → RUN build) to prove chain memoization: editing src and
  rebuilding MUST hit the cook step's memo and only re-run the final step. Demonstrate and
  transcript this in the LOG.

## Verification gate

1. Workspace build/test/fmt --check/clippy -D warnings; parser+memo unit tests (chain
   keying, hit/miss on source edit vs lock edit, FETCH pin verify-mismatch error path).
2. Determinism: `cix build` projB twice from clean lock state → identical final store
   path; transcript both paths in the LOG.
3. Live (sudo allowed): projB end-to-end (build, run, curl, stop, clean); projB-chef
   memo-hit demonstration per above.
4. Tour: a build-with-RUN scenario page if deterministic under the normalizers; else
   explain in the LOG. Drift green either way.
5. `nix build .#checks.x86_64-linux.vm-dogfood` passes.
6. docs: cixfile.md gains the RUN/FETCH section (honest about v0 scope: linear chain, no
   AS, no tracer yet); docker.md `RUN` row flips per D39; cixfile-build.md gets a
   superseded-by-D39 banner (do not delete — it is the decision record's context).
7. Commit on branch track/runv0. No commit = failed task.

## Log

Keep .dev/specs/track-runv0.LOG.md current (append-only, timestamped, transcripts;
spec-boundary frictions feed the next design round).
