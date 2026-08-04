# Cixfile track work log

- 2026-08-04T08:00:00Z — Prescribed agent gate is green with synchronous exit
  statuses: `cargo fmt --all --check`; `cargo run -p cix -- fmt --check
  examples`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo
  test --workspace`; ignored tour regeneration followed by exact committed-tour
  and deterministic-tour tests; and ignored corpus-browser regeneration followed
  by its exact committed-page test. The examples formatter emitted only the
  pre-existing CIP-91 `LINK` deprecation warnings and exited 0. No VM scenario
  is structurally affected: cix-run already carried UDP `SocketBindAllow`
  codegen and its workspace coverage ran. Re-graded the Docker port row
  (UDP declaration vs TCP-only compose publish), marked Caddy's protocol gap
  stale for CIP-92, and regenerated its browser page. `git diff --check` exits
  0. Next: stage the scoped implementation, tests, ledgers, generated docs,
  and this required journal; inspect the cached diff; commit the complete
  logical CIP-92 unit.

- 2026-08-04T07:56:53Z — Implemented CIP-92 end to end. `PORT` now accepts
  bare TCP or `udp:<port>` (including env-backed values), rejects Docker's
  `443/udp` with the `udp:443` correction, and emits protocol-bearing manifest
  declarations. Existing cix-run codegen projects that to `SocketBindAllow`;
  the golden manifest carries UDP, cix inspect serializes it, and compose now
  refuses an attempted UDP publication because its proxy remains TCP-only.
  Added `cix build --file NAME DIR[#member]`: named files select sibling
  `NAME.lock`, preserve selectors/tags/namespaces/update/cold options, reject
  path traversal, and name a missing full path. The real-Nix twin fixture
  proves independent output and locks; the regenerated build tour documents
  the flag. Synchronous focused receipts: parser (31 tests), named-lock
  fixture, tour regeneration, and deterministic-tour run all exited 0. Next:
  run the prescribed formatter/clippy/workspace/tour-drift gate, inspect the
  final scope, and commit the complete logical CIP-92 unit including this log.

- 2026-08-04T07:45:37Z — Started `track/cip92`. Read AGENTS.md, the assigned
  specification, adopted CIP-92 decision, current session journal, and this
  crate journal; direnv/devenv is active. Scope is protocol-aware `PORT`, the
  named-Cixfile build option, their tests and build-tour documentation. The
  concurrent netns/adapter tracks and corpus/docs-corpus fence remain untouched.
  Next: trace the current parser → manifest → runner/inspect and compose
  publish paths, then implement the smallest end-to-end protocol model.

- 2026-08-02T23:42:28Z — Committed the steady-state receipt and its explicit
  revert-branch observability as `b616405` (`docs: distinguish first and
  steady warm builds`). It contains only the timing marker in cix-build and
  docs/nix-build.md; this required journal remains deliberately unstaged.
  Post-commit status is journal-only. Ready for merge review and the
  orchestrator's full flake-matrix gate.

- 2026-08-02T23:42:28Z — Follow-up steady-state receipt resolved the
  first-warm/steady-warm comparison. On the completed single-prime fixture,
  the first one-line edit measured 14.46 s; a second edit measured 8.31 s and
  a third 8.84 s. All three were FETCH memo hits. The steady receipts record
  self validation at 0.217 s / 0.216 s; the third also carries the new
  `fetch-revert` timing marker and it was absent, synchronously proving that
  a hit did not enter the revert branch. Updated docs/nix-build.md to report
  first-warm and steady-warm separately. Next: final format/scope checks and
  commit this receipt without the journal.

- 2026-08-02T23:38:54Z — Committed the completed FETCH self-observation
  implementation as `5b539d2` (`build: validate fetch self-observation`). The
  commit contains only cix-build, the real-Nix regression fixture, and the
  gitsitter measurement harness; this required append-only journal remains
  deliberately unstaged. `git diff --cached --check` and the post-commit
  status are clean apart from this journal. Ready for the orchestrator's full
  flake-matrix verification.

- 2026-08-02T23:38:11Z — Completed the CIP-87 implementation and focused
  receipts. FETCH output content hashes remain deterministic lockfile data;
  local workspace metadata fingerprints are only an accelerator and are
  invalidated when a later traced step changes a FETCH output. Descendant
  self-reads reuse their already-validated output root instead of re-hashing
  the vendor tree once per read. The final single-prime gitsitter harness
  completed its cold control and reports FETCH memo-hit; warm time was 14.46 s
  (not the historical ~8.3 s), now attributed: FETCH self validation is
  0.224 s, while the existing RUN path traces 428 MiB in 6.001 s and executes
  the incremental compile. fmt, examples fmt, warning-denied clippy, focused
  cix-build and real-Nix lock tests, tour regeneration/drift, and the
  workspace suite are green; the full flake matrix remains the orchestrator
  gate. Next: final status/scope audit, commit product files without this
  required journal.

- 2026-08-02T23:12:00Z — Implemented the CIP-87 self-observation rule in
  cix-build. A warm FETCH now removes the previous memo's owned outputs before
  recording a replacement pre-state; FETCH-only validation can accept its own
  output only when the whole recorded write set matches its constructive
  content hashes, while cold and RUN validation retain exact ordinary
  fingerprints. FETCH hits re-apply any root not already proven identical.
  Newly-created multi-file FETCH trees record their complete root, fixing cold
  replay of cargo vendor trees. Added focused unit and real-Nix mini-fixture
  coverage for byte-identical warm/cold lock traces, partial self state,
  a→b scope, and the pre-`--update-lock` pin mismatch; all passed. The
  gitsitter harness now has one prime plus a green FETCH-only cold control.
  The full harness completed synchronously (exit 0) but measured 69.58 s,
  not ~8.3 s: its explicit timing shows FETCH memo-hit and RUN 13.54 s, with
  the remaining time currently un-attributed inside output-self validation.
  Next: complete gate and isolate that performance regression before commit.

- 2026-08-02T22:39:20Z — Started `track/fetchself` from the CIP-87
  self-observation adoption commit. Read the assigned specification, AGENTS,
  current journals, and the full adopted amendment. Scope is cix-build's
  FETCH memo validation/replay path plus the gitsitter measurement harness;
  `track/thin2` owns cix-compose and will not be touched. The existing
  tracefast finding confirms the failure is a warm FETCH observing its own
  prior writes. Next: make the same-memo, content-hash exception constructive
  and cold-excluded; add the partial/self and a→b adversarial receipts.

- 2026-08-02T21:43:00Z — Committed the complete green docs track as
  `158df29` (`docs: align status with implemented board`): nine fenced prose
  files, 44 insertions and 22 deletions. Post-commit status contains only this
  required, deliberately unstaged journal. No product source or generated
  corpus/tour page changed. The requested docstruth track is complete and ready
  for independent verification.

- 2026-08-02T21:42:30Z — Completed the fenced documentation reconciliation.
  Design/index/README now state that the adopted board and all four strata work;
  the frontier is closed-root phase 2, D26/D27, publish, and reconciliation.
  Open questions now separates built D49 from named-network work and records the
  systemd-257 adapter-liveness finding with the Mastodon receipt. Added missing
  implementation bullets for CIPs 79, 81, 82 leg 2, 84 phase 1, and 90 leg A;
  CIPs 85, 86, and 89 already had dated implementation entries and were not
  duplicated. Synchronous receipts are green: both exact corpus deterministic/
  committed-page tests and both exact tour deterministic/committed-document
  tests passed; `git diff --check` passed, local links and the receipt anchor were
  inspected, no generated docs changed, and the diff-stat is docs plus this
  required journal only. Next: stage the fenced docs without this journal,
  recheck the cached diff/scope, and commit.

- 2026-08-02T21:37:44Z — Started `track/docstruth` from a clean branch.
  Read the complete track specification and the prescribed project/design/crate
  truth sources; direnv confirms the devenv is loaded. Scope is pure prose:
  current-status corrections in design/index/README/open questions plus one
  implementation changelog line for each named CIP, with no product or
  generated-doc changes. This tracked journal will remain outside the docs
  commit. Next: reconcile every stale open-question/status claim against the
  landed CIPs and the Mastodon systemd-257 receipt, then make the fenced edits.

- 2026-08-02T19:28:36Z — Committed the complete green track as `800c87a`
  (`docs: add Mastodon integration receipt`): 29 scoped corpus/docs files,
  including six locked member artifacts, executable receipt, two ledger
  regrades, and regenerated browser pages. Cached whitespace and product-fence
  checks passed immediately before commit. Post-commit status contains only this
  required, deliberately unstaged journal; `origin/track/mastodon` does not yet
  exist, and the specification requested a branch commit rather than a push.
  Open product finding: systemd-257 does not retain the forked adapter liveness
  pinger; it is documented in the receipt and above, with no product-code change.
  The requested Mastodon track is complete and ready for orchestrator full-matrix
  verification.

- 2026-08-02T19:27:30Z — Final strengthened focused receipt is green with
  synchronous exit 0 and cleanup: `./check.sh cix` activated generation
  `xl5xsizh…`, reproduced the three delayed readiness failures then success,
  observed three cleanup-timer firings, and kept both native-notify services
  active for seven seconds—past their six-second watchdog window. Final item
  paths remain those recorded in the receipt. This closes the only late audit
  improvement; no Cixfile, lock, docs generator input, Rust source, or broad-gate
  input changed after its prior green result. Next: refresh the two staged files,
  repeat cached whitespace/fence audit, and commit.

- 2026-08-02T19:26:15Z — Prescribed agent gate is fully green with synchronous
  terminal statuses: `cargo fmt --all --check`; `cargo run -p cix -- fmt
  --check examples`; `cargo clippy --workspace --all-targets -- -D warnings`;
  serial `cargo test --workspace -- --test-threads=1` (including committed
  corpus/tour drift, real-Nix builds, and all crate suites); fresh ignored tour
  regeneration followed by zero `docs/tour` diff; and the exact deterministic
  tour test. Focused Mastodon and corpus-generator receipts are recorded above.
  Final unstaged audit has no whitespace errors, no ephemeral `.env`/`cix.lock`,
  and no product source paths: only this required journal, docs/corpus regrade +
  generated pages, and the new corpus case differ. Next: stage exactly the
  corpus/docs deliverables, verify the fence and cached diff, commit on
  `track/mastodon`, then leave a final journal-only close.

- 2026-08-02T19:22:35Z — Corpus browser regeneration and focused drift test are
  green. The house generator required the verbatim upstream file to use its
  recognized `upstream-compose.yml` name; renamed it without byte changes and
  updated SOURCE. `generate_corpus_browser` added only Mastodon's index row and
  page, and `generated_corpus_browser_is_deterministic --exact` passed. Reviewed
  the generated upstream/compose comparison and ledger text; row 9 retains a
  partial ribbon because segmentation is honestly absent, while row 10 points
  only to the empirically shared-rw mechanism rather than claiming Penpot ran.
  Next: execute fmt, examples fmt, warning-denied clippy, workspace tests, and
  tour regeneration/drift; then audit and commit the green track.

- 2026-08-02T19:21:16Z — Focused Mastodon receipt is synchronously green:
  `corpus/migrate/mastodon/check.sh cix` exited 0 after its EXIT cleanup. It
  built six locked items, validated six services/two Unix edges, activated
  generation `4ihz6a…`, showed three refused web probes before the delayed
  readiness success, verified independent web+sidekiq shared writes and the
  `CREDENTIALS_DIRECTORY` marker, observed two timer firings, and proved
  `cix logs corpus-mastodon/web` excluded the sidekiq marker. Native-notify
  watchdogs remained healthy through the receipt. Exact final item paths are in
  `receipt.md`. Re-graded compose rows 9/10 and retired Mastodon from §5's
  candidate list while keeping D26/D27 segmentation visibly unclaimed. Next:
  regenerate and drift-check the corpus browser, then run the prescribed broad
  formatter/clippy/workspace/tour gate.

- 2026-08-02T19:18:00Z — Second focused run found a real product compatibility
  boundary and was interrupted after synchronous journal diagnosis rather than
  counted: on this host's systemd 257, the HTTP/TCP liveness `ExecStartPost`
  parent exits 0 but its forked resident pinger is not retained; healthy Redis
  then dies at the six-second watchdog window. The newer-systemd health VM is
  known green, so this is recorded as a host-version product finding and product
  code remains untouched per the track fence. Switched the two controllable
  stubs to CIP-79 native-notify liveness, retained cix-owned readiness adapters
  on all five long-running members, and removed adapter liveness from packaged
  daemons. Also corrected nginx's invalid `/dev/stdout` open under its sandbox.
  Next: rebuild and rerun the focused receipt, confirming watchdog notifications
  remain live past multiple windows.

- 2026-08-02T19:15:49Z — First focused `./check.sh cix` reached a valid
  six-service/two-edge compose and failed activation. Retained journal evidence
  identified three fixture errors, not product defects: probe adapters embedded
  the worktree binary path hidden by service `ProtectHome` (the check now adds
  non-store cix binaries to the store first); every psql client omitted the
  server's non-default port; and two minimal shell closures assumed ambient
  `cat`. PostgreSQL and Redis themselves had become ready before the adapter
  path killed them. Corrected the check boundary, explicit port, and shell-builtin
  credential reads without widening any artifact or crossing the product fence.
  Next: rebuild the content-changed items and rerun the complete focused receipt.

- 2026-08-02T19:14:10Z — Authored the complete executable corpus shape without
  crossing the product-code fence: verbatim pinned upstream compose; five
  long-running member Cixfiles plus a scheduled maintenance APP; real PostgreSQL,
  Redis, and nginx; Puma/Sidekiq-shaped Python stubs; DB/Redis Unix edges; the
  public-system shared surface; native probes; file credential delivery; and
  egress claims only on web/sidekiq. `check.sh` proves each required behavior and
  uses the approved host-network fallback. Static bash syntax, Cix formatting,
  whitespace, and upstream byte comparison are clean after correcting three
  verbatim comment-indentation bytes. Next: generate each locked nixpkgs input,
  then run the focused stack and fix only corpus artifacts if it exposes a bug.

- 2026-08-02T19:04:53Z — Started `track/mastodon` from `40d44f1`. Read the
  complete track spec, current project journal, authoritative D26/D27 boundary,
  and this crate journal; direnv/devenv is active. Scope is corpus/example/doc
  files only: a five-service Mastodon-shaped integration using shared-rw dirs,
  health, secrets, unix edges, a timer, exact egress claims, and member-selective
  logs. Product-code changes are fenced to concurrent tracks. The assigned log
  is tracked rather than ignored, so it will remain deliberately unstaged as
  required. Next: inspect the adopted CIP syntax and existing executable receipt
  patterns, then author the stack and focused check.

- 2026-08-02T22:05:00Z — Tracefast round 3 GREEN: the measured warm one-line
  edit is 8.31 s (repeats on the same quiet host: 8.54 s, 8.73 s), under the
  ~9 s bar and crane's 16.46 s, with capture completeness untouched — file
  contents, directory listings, negative lookups, and writes all recorded via
  seccomp-BPF-filtered ptrace. Work receipt: FETCH memo-hit, RUN executed,
  `cix_run_compiled_units=1`, 9 Nix subprocesses totaling 0.28 s.

  Q1 (is incremental compilation active): NO, by three receipts —
  `cargo build --release` uses the release profile whose default is
  `incremental = false`; gitsitter's Cargo.toml declares no profile section
  and CARGO_INCREMENTAL is absent from the sandbox env; the warm workspace
  has no `target/release/incremental/`. Lever measurement in a no-Cix bwrap
  replica of the sandbox (same store toolchain, cleared env, /work mount):
  CARGO_INCREMENTAL=1 turns the warm edit into 1.65 s after a 5.93 s priming
  build, versus 5.97 s non-incremental. It is env-only but NOT taken:
  incremental output is not guaranteed byte-identical to a from-scratch
  build, so the fixture pin and normal/repeat/--cold byte-identity would be
  at risk, and neither comparator uses it. Mathijs's lever if wanted.

  Q2 (why raw cargo was 15.54 s where the pre-readset total was 7.46 s):
  fully attributed with CARGO_LOG=cargo::core::compiler::fingerprint=info in
  the same replica. (a) Fixture floor: gitsitter's build.rs declares
  `cargo:rerun-if-changed=.git/HEAD`; the staged source has no .git and
  cargo treats a missing rerun-if-changed file as permanently stale
  (`FsStatusOutdated(StaleItem(MissingFile ".git/HEAD"))`), so build.rs +
  lib + bin rebuild on EVERY build — 5.9 s even for a no-op. The old 7.46 s
  was this floor plus ~1.5 s of chain-keyed Cix overhead; the floor is
  symmetric across upstream/crane. (b) The rest was mtime/inode churn: a
  cp -a copy of the warm workspace rebuilds the edit in 5.97 s, while the
  same workspace with vendor/ rewritten byte-identically but with fresh
  inodes+mtimes — which warm replay did every build — takes 15.65 s and
  recompiles exactly 100 units, matching the measured run's 100 `Compiling`
  lines. Receipt: `ChangedFile { reference:
  target/release/build/anyhow-*/output, stale: vendor/anyhow/src/nightly.rs
  }` — vendored build scripts go stale and StaleDepFingerprint cascades.

  Landed, verify-then-implement, biggest first:
  1. Mtime-preserving staging (H3, the Q2 suspect, confirmed): `sync_node`
     short-circuits when old and new staged inputs are byte-equal;
     `apply_step_memo` replays through a per-node reconciling
     `sync_replay_node` (same end state as remove-and-recopy, touches only
     differing nodes); and memo application is skipped entirely when
     workspace state.json's new `materialized_memos` (owner → memo key)
     records the outputs as already present — the same trust model as the
     existing `rerun_from` prefix. Result: FETCH validation rehashes 0
     bytes, the recorder reuses all 3217 validated reads (1 ms, was 1659 ms
     over 123 MiB), and cargo compiles 1 unit.
  2. Trace-side reuse (H1) was wired by round 2; mtime stability is what
     makes it bite — fingerprints now hit instead of rehashing.
  3. Tracer parallelism (H2): measure-gated DEAD. Traced cargo is 6.44 s
     against the 5.9 s untraced floor — ~0.55 s total ptrace cost on the
     small rebuild with seccomp-BPF already filtering. No FUSE/eBPF
     mechanism switch is warranted for ≤0.55 s; that is the frontier of
     this lever, recorded for the CIP.
  4. Fixture repair: the FETCH now also sets CARGO_HOME=/tmp/cix-vendor-home
     (mirroring the existing CARGO_TARGET_DIR redirect), so the 619 MiB /
     24k-file cargo registry cache no longer lands in the workspace, the
     FETCH write set, snapshots, or warm replay; FETCH changes shrink to
     vendor/ + .cargo/config.toml. Re-pin double-probe: two pristine
     --update-lock runs produced the identical consumed pin
     sha256-NywkGX47LrMn8I/2vzkBO2476pSUUV2tSm8UHfu2EUE= (fixture change
     proven byte-identical); committed pin id builder:build:2-6c46b57a1539.
  5. RUN output snapshots dropped (Mathijs's in-chat coarse-grain/exclude
     prompt, 2026-08-02): the RUN memo's constructive snapshot was the whole
     warm target/ tree — measured 512 MiB / 1913 files nar-added to the
     store on EVERY one-line edit (~1.0-1.2 s plus 512 MiB store growth per
     edit). FETCH memos stay constructive (pins and --cold replay need
     them); RUN memos are verifying-only — a RUN that would have replayed
     re-executes, which in a warm workspace is exactly the cheap path. Read
     capture is unaffected. Layered/overlay snapshots remain the
     constructive alternative if RUN replay-without-execution is wanted.
  6. Nix subprocess diet (doubles as the libnix ROI table): warm-edit calls
     went from 11 (~2.15 s) to 9 (0.28 s: source add 0.02, requisites 0.01,
     2× hash path 0.01, consumed add 0.01, view add 0.01, item build 0.19,
     item add 0.01). Offer realization is skipped when every offered store
     path exists (store-completeness invariant, the ensure_store_path
     assumption); the builder context eval is cached keyed on the generated
     expression (byte-stable across edits) with source-rooted copies
     re-rooted via `nix store add --mode nar --name cix-source` (verified
     path-identical to builtins.path) and validated by path existence;
     content-dependent expressions (hashFile, project-local overlays) never
     take the fastpath. Timing instrumentation is now gated behind
     CIX_TIMING=1 (set by measure-warm.sh) so ordinary output and the
     generated tour stay clean.
  7. The pre-command probe snapshot cleanup (~0.18 s) overlaps the post-RUN
     store add in a scoped thread (disjoint trees, joined before the memo
     lands).

  Descent (complete capture throughout): 94.17 baseline → 34.08 / 28.53
  (round 2) → 27.66 (round-2 candidate re-measured) → 12.28 (mtime
  staging + fixture CARGO_HOME) → 9.90 (memo-apply skip) → 9.36
  (realize-skip + cleanup overlap) → 9.31 (RUN snapshots dropped) → 8.31 /
  8.54 / 8.73 (context cache; quiet host). Final component receipt:
  chain-keys 16+19 ms, COPY 68 ms, FETCH validation 41 ms (0 rehashed),
  RUN validation 241 ms (20 files / 16.8 MiB — honest pre-state of target
  reads), workspace snapshot 398 ms, traced RUN ≈6.5 s (cargo floor 5.9 +
  ptrace 0.55), read-set recording 1 ms, delta 0 ms, probe cleanup 184 ms
  (overlapped), Nix 0.28 s. Concurrent-agent load (LA>9) inflates totals to
  ~9.8 s; the dated receipts are quiet-host.

  Guardrails: no-op 0.07 s, zero Nix subprocesses, zero executed steps
  (--stats receipt). Full agent gate green synchronously: fmt, examples
  fmt, warning-denied workspace clippy, serial workspace tests
  (/tmp/tracefast-workspace-tests.log), tour regen + zero drift
  (/tmp/tracefast-tour-regen.log), vm-dogfood
  (/tmp/tracefast-vm-dogfood.log). Byte-identity: the hermetic CIP-87
  acceptance suite passes and the fixture re-pin double-probe above is the
  compare-side byte-identity receipt.

  FINDING for Mathijs (cold/.cargo pre-state divergence — memo-matching
  semantics, so not taken unilaterally; gitsitter cold control still exits 1
  at 0.21 s, before any build work): the entire warm/cold divergence is
  FETCH self-observation. A FETCH re-executed in a warm workspace observes
  its own prior outputs as pre-state (warm memo records `.cargo`
  DirectoryExists plus 2939 vendor pre-state reads; a virgin-workspace FETCH
  memo differs from a warm workspace at exactly ONE meaningful entry,
  `vendor: absent`; receipt: `recorded read set differs between warm and
  cold at ".cargo" (warm DirectoryExists, cold Absent)`). Proposed rule, two
  halves: (1) a FETCH about to execute first reverts the paths its
  superseded memo recorded as its own writes, making FETCH results
  workspace-history-independent (pin-contract hygiene, same intent as the
  double probe); (2) memo validation treats a read of a path in the memo's
  own write set as satisfied when the workspace currently holds the memo's
  recorded output for that path (the memo itself proves re-execution from
  the recorded pre-state yields exactly that output). Together these make
  warm and cold traces identical by construction, the cold control green,
  and the harness's two-prime workaround unnecessary. Half (2) extends
  §3's definition of "matches" — that is the amendment needing sign-off.
  Also recorded: cargo vendor's random `vendor/.vendor-stagingXXXX` temp
  dirs leave ~25k absent-markers in a virgin FETCH trace (harmless — random
  names stay absent — but ~3.4 MiB of lock noise; CIP-87 open-question-2
  territory).

- 2026-08-02T20:40:00Z — New tracefast engineering round started with the
  completeness decision explicitly parked: trace-side reuse, tracer
  parallelism, and mtime-preserving staging will be implemented and measured
  independently, in that order, without relaxing files, listings, negative
  lookups, or writes. The cold `.cargo` mismatch will be repaired only if it
  is ownership of prior-run pre-state; any semantic carve-out stops for a CIP
  amendment. Current complete floor remains 28.53–34.08 s; next: make the
  executed-step recorder reuse dependencies already validated against its
  exact pre-command workspace and count reused versus hashed inputs.

- 2026-08-02T20:36:00Z — Tracefast STOP on the mandated complete-capture
  frontier. Fixed the handoff candidate's second blocker: file fingerprints
  had accidentally become semantic equality, so an inode/mtime miss rehashed
  identical bytes into `fingerprint: null` and falsely invalidated every
  restaged file. Fingerprints now remain validation-only; replacement,
  changed-content, and deletion tests pass (27 cix-build tests total).
  A fresh synchronous Cix-only harness then exited 0 with the required work
  split (FETCH memo-hit, RUN executed, 11 Nix subprocesses) but took 34.08 s;
  a second complete warm edit from another valid trace took 28.53 s. Both are
  above the ~9 s green bar and crane's fixed 16.46 s floor.

  Complete 34.08 s attribution: COPY 0.135 s; memo validation 0.368 s
  (4/177,385 B plus 655/21,253,716 B rehashed); pre-command snapshot 0.707 s;
  traced RUN plus read materialization 25.731 s (Cargo reported 18.39 s,
  read-set hashing 6.092 s); output snapshot 1.046 s; chain hashing 0.045 s.
  The 11 individually timed Nix calls — also the libnix ROI input — were
  0.39, 0.42, 0.01, 0.01, 0.87, 0.01, 0.01, 0.01, 0.01, 0.20, and 0.01 s
  (1.95 s total). Descent: complete baseline 94.17 s → write-set deltas,
  fingerprint validation, and seccomp-BPF-filtered ptrace 34.08 s → observed
  complete repeat 28.53 s; still red.

  Relaxation frontier, measured rather than proposed as behavior: dropping
  content read-set materialization can save at most the observed 6.09 s
  (34.08 → 27.99 s) but loses CIP-87 content dependencies; dropping all
  tracing/read capture in an otherwise identical sandbox produced 28.96 s
  (FETCH hit/RUN executed) and an untraced RUN of 15.574 s, versus the
  complete RUN's 25.731 s, but loses reads, negative lookups, and new write
  discovery; deleting every Cix cost down to that raw Cargo command still
  leaves 15.54 s, above green and only 0.92 s under crane. Eliminating all 11
  Nix calls alone buys only 1.95 s; skipping output or input snapshots buys
  1.05 or 0.71 s while making replay/input capture incomplete. No measured
  relaxation reaches 9 s.

  Guardrails: no-op is green at 0.16 s, zero Nix subprocesses, all four steps
  memo-hit. The gitsitter cold control exits 1 at 1.15 s before RUN because
  the latest warm FETCH trace observes `.cargo` present while the empty cold
  workspace observes it absent (`mkdir -p .cargo`); this is an exact negative
  lookup frontier, not a green cold receipt. Per the track instruction, did
  not update docs, run the CIP/full agent gates, or commit after the complete
  floor remained above the bar. The complete tracer is restored and rebuilt;
  fmt, diff check, and all 27 cix-build tests pass synchronously. Mathijs must
  choose the completeness trade or reject/rework CIP-87.

- 2026-08-02T20:20:00Z — Tracefast handoff prime failure root-caused
  synchronously. The preserved harness exited 1 only after pristine FETCH and
  RUN completed: the committed fixture pinned `cS9…`, while a fresh
  `--update-lock build` double-probe reported identical pristine outputs and
  produced `Nywk…`. FETCH pins are checked only when FETCH executes; the
  measured source edit must instead memo-hit FETCH and may replace the final
  binary in RUN. `cS9…` is therefore the edited RUN output and cannot prime the
  pristine fixture. Restored the pristine `Nywk…` pin; next: replay the exact
  two-prime harness and measured edit, then complete the descent/guardrails.

- 2026-08-02T20:10:00Z — Diagnosed the benchmark mismatch as replay-cache loss:
  a fresh FETCH reports only `.cargo/.global-cache` as volatile and the
  consumed pin remains only `target/release/gitsitter`. Committed the honest,
  isolated fixture re-pin as `f7542fa` (`bench: repin gitsitter fetch fixture`),
  with obsolete split-chain keys and memo/output receipts removed. Next: build
  the optimized binary, obtain a successful warm receipt, then run CIP-87 and
  the prescribed agent gate before committing implementation.

- 2026-08-02T20:05:00Z — Tracefast STOP: baseline attribution showed full-tree delta scans at 49–51 s per executed step, so I replaced them with exact traced-write comparisons; added mtime/inode validation fingerprints and kernel seccomp-BPF syscall filtering. Focused cix-build tests (26) pass. The required end-to-end receipt is still blocked by the gitsitter harness: its regenerated FETCH output differs from the checked lock pin (`sha256-cS9…` vs `sha256-Nywk…`) even before a valid warm RUN receipt can complete. Do not commit: broader gates, byte-identical CIP suite, no-op/cold receipts, docs, and the ≤9 s measurement remain outstanding.

- 2026-08-02T19:45:00Z — Baseline `examples/compare/gitsitter/measure-warm.sh`
  completed alone in an attached terminal (exit 0): upstream 34.92 s, crane
  16.25 s, and Cix CIP-87 94.17 s. Cix work receipt remained correct:
  FETCH memo-hit, RUN executed, 11 Nix subprocesses. This is well above the
  ≤9 s green bar, so no behavior change has landed. Next: obtain the mandated
  component table (including no-trace RUN control, validation files/bytes,
  COPY/staging, chain hashing, and each Nix invocation) before optimizing.

- 2026-08-02T19:30:00Z — Started `track/tracefast` for CIP-87's post-landing
  warm-edit performance bar. Read the complete track spec, CIP-87 (including
  the performance-criterion changelog), project journal, current Cixfile
  journal, and active dev environment. Scope is `cix-build` trace/build-chain
  memo-validation paths and the gitsitter measurement harness only; hygiene
  owns env/config intake and netns owns cix-compose. Next: record the required
  unoptimized gitsitter cost table (trace, validation rehash, staging,
  chain-hash, and every Nix subprocess) before changing behavior.

- 2026-08-02T17:05:00Z — Track implementation and agent gate complete on
  `track/thin1`: commits `52adf3e` (unit pure move), `a7f3b25` (FETCH
  consent move), `961d626` (tripwire + compat audit), and `62ea95d`
  (test-scoped imports). Synchronous receipts: `cargo fmt --all --check`;
  `cargo run -- fmt --check examples`; warning-denied
  `cargo clippy --workspace --all-targets -- -D warnings`; serial
  `cargo test --workspace -- --test-threads=1` (rerun with terminal status
  captured in `/tmp/composix-thin1-workspace-test.log`); ignored tour
  regeneration plus zero `docs/tour` diff and committed-tour test; focused
  `nix build .#checks.x86_64-linux.vm-dogfood -L --no-link` (terminal status
  captured in `/tmp/composix-thin1-vm-dogfood.log`); and flake-integrated
  `nix build .#checks.x86_64-linux.source-size --no-link`. The first tour
  generation hit the pre-existing intermittent proj1 workspace race (missing
  Cargo fingerprint directory); the immediate fresh rerun passed with no
  generated drift, so it is recorded but not attributed to this refactor.
  Final audit next; this required journal stays unstaged.

- 2026-08-02T16:35:00Z — CIP-89 coupling/audit checkpoint. `fetch.rs` now
  owns ConsentStore and exposes only command-scoped credentials, but the
  memo/workspace code still mutually depends on conductor-local trace,
  sandbox, and store helpers; per §4 turn 1 I stopped rather than inventing a
  fake narrow boundary. The tripwire therefore names `build_chain.rs` visibly
  as a grandfathered conductor with this reason. Compat inventory: deleted the
  unreachable `LegacyLockFile { nixpkgs }` reader (no committed locks match);
  deleted index `tags/` sidecar migration after confirming no live
  `~/.local/state/cix/tags` and no committed state; it now teaches regeneration.
  Retained FetchPin whole-tree and storePath forms plus MemoEntry old output
  fields because 15 committed locks contain them (including corpus legacy
  FETCH/memo records). Exact focused checks: source-size tripwire, cix-build
  24 tests, cix-index 8 tests all passed synchronously. Next: commit the pure
  FETCH move separately, then complete docs/gates and commit the audit.

- 2026-08-02T16:12:00Z — Pure-move unit stratum complete: health,
  device-policy, and managed-directory assembly now live in dedicated modules;
  `unit.rs` still calls them at the original property-order positions. Their
  module contracts make ordering authority explicit in the conductor. Exact
  proof: `cargo fmt --all --check && cargo test -p cix-run --lib` (69 passed),
  plus `git diff --check`, all synchronous. Next: commit this isolated move,
  then split build-chain's owned strata.

- 2026-08-02T16:00:00Z — Started `track/thin1` for CIP-89 leg 1. Read the
  complete track spec, current project/crate journals, CIP-89 including all
  four amended turns, D72, and the active environment (direnv/devenv is
  active). Scope: quiet cix-build/cix-run/cix-index owned-type thinning;
  explicit compatibility audit; module maps and 2000-LOC hard tripwire;
  docs/README/CIP changelog. cix-compose is expressly out of scope. Next:
  inventory the exact build-chain, unit, lock, and refs seams before making
  pure-move commits.

- 2026-08-02T15:30:00Z — Committed the complete green overlay track as
  `d4a9990` (`feat: add overlay package universes`). It contains only the
  scoped implementation, migrations, lock, generated corpus/tour pages,
  tests, and docs; this append-only journal remains intentionally unstaged.
  Final audit next.

- 2026-08-02T15:28:00Z — All prescribed gates are green. `cargo fmt --all
  --check`, `cargo run -- fmt --check examples`, warning-denied all-target
  clippy, serial workspace tests, corpus regeneration/drift, and generated-tour
  determinism passed. Wallos was re-run after its final source/lock update:
  `./check.sh cix` exited 0 and reported `PASS cix`. The required track-ending
  `devenv shell -- nix flake check -L` first completed the 70-check VM tier;
  because that verbose stream truncated before exposing a terminal status, the
  exact rerun was made synchronously and exited 0, reporting all checks
  previously built. Next: final staged-scope audit and commit; this journal
  remains intentionally unstaged.

- 2026-08-02T15:17:00Z — Broad fmt/examples/clippy/serial-workspace gates are
  green. Tour regeneration correctly exposed expected committed-doc drift in
  abbreviated chain keys after D70 key semantics; bumped the codegen fingerprint
  `d80-v1` → `d80-v2` to isolate pre-overlay memos. Next: regenerate and
  recheck tour drift after that bump, refresh the Wallos lock/receipt once more,
  then run the mandatory full flake gate.

- 2026-08-02T15:05:00Z — D70 implementation and its focused receipts are
  green. `FROM … OVERLAY … AS` retains ordered project-local overlay paths,
  evaluates a functionArgs-checked native Nix overlay fixpoint, keys every
  build chain and vendored dev environment on base-pin plus ordered overlay
  content hashes, and leaves `--update-lock` to the base input alone. The
  parser refuses leading EXPECT with its rewrite; fmt migrates it to trailing,
  so the parsed declaration—not directive trivia—feeds execution. Exact green
  checks: cix-build/cix-cixfile unit suites; parser/fmt integration; real-Nix
  overlay order/malformed-overlay fixture; and real builder proof that an
  overlay edit creates a second memo key without moving `inputs.pkgs`.
  Replaced Wallos's `default.nix` with `Cixfile`, `php.nix`, and a checked lock;
  `CIX=.../target/debug/cix ./check.sh cix` exited 0, built
  `/nix/store/rds1fgd05lf8hh46i7h67inc2kxyw28c-cix-item-wallos`, started its
  system unit, and passed `/health.php`. Corpus browser regeneration and drift
  test are green. Next: complete the prescribed formatter/clippy/workspace/tour
  gates, rerun Wallos after the final source changes, then run the full flake
  check and commit only track files (never this journal).

- 2026-08-02T14:20:00Z — Started `track/overlay` from `81586c5`. Read the
  complete overlay specification, project journal, D70 authority, and this
  crate journal. Scope: repeatable ordered `FROM … OVERLAY … AS` universes,
  additive lock/key identity (including vendored dev-env snapshots), trailing
  FETCH EXPECT migration/formatting, Wallos conversion and receipt, docs and
  corpus regeneration. Next: map parser/model/build/lock seams and current
  Wallos corpus inputs before implementation.

- 2026-08-02T13:00:00Z — Began the requested ergo fix round 2 on
  `track/ergo` at `61ca217`. The independent gate reports
  `proj1_multi_item_cache_selectivity_and_clean_rebuild` creates two builder
  workspaces after `fb7df76`; the specification requires a synchronous
  before/after reproduction and a fix that excludes prior-run output data from
  chain/workspace keys. The devenv configuration is present. Next: isolate the
  exact test on `fb7df76^` and `fb7df76`, then trace the key inputs.

- 2026-08-02T13:12:00Z — Completed the prescribed detached before/after
  reproduction synchronously. After stashing only this journal, both
  `fb7df76^` (`e851b0e`) and `fb7df76` passed
  `devenv shell -- cargo test -p cix-cixfile --test proj1
  proj1_multi_item_cache_selectivity_and_clean_rebuild -- --exact --nocapture`
  (0, 5.63s and 5.69s); the workspace identity stayed one directory in both.
  The purported suspect commit changes only `crates/cix/tests/tour.rs` and its
  generated page, not build/key code. Source inspection also shows
  `build_fingerprint` explicitly hashes only inputs, artifacts, fetches, and
  dev environments—not `memo` or `outputs`—and `workspace_identity` hashes
  only canonical directory plus builder. The likely varying input is the
  process-global `CIX_BUILD_WORKSPACE_DIR`: the two concurrently scheduled
  `proj1.rs` tests each overwrite it. The output restore shortens one test and
  changes the race timing, explaining the gate-only failure. Next: reproduce
  under the full test binary/workspace and eliminate the global environment
  race at the test support seam.

- 2026-08-02T13:24:00Z — Reproduced the gate-only failure synchronously with
  `devenv shell -- cargo test -p cix-cixfile --test proj1 -- --nocapture`
  (exit 101): its parallel tests switched `CIX_BUILD_WORKSPACE_DIR` between
  two temporary roots, so the proj1 assertion saw two builder identities under
  its own base. Fixed the varying input rather than changing the assertion:
  `proj1.rs` now serializes that process-global override and restores its prior
  value. Added `build::tests::output_receipts_do_not_change_the_build_fingerprint`,
  which proves distinct prior-run `OutputReceipt` values leave the source/output
  cache fingerprint unchanged. Exact focused unit test, concurrent two-test
  proj1 binary, and formatter check all pass synchronously. Next: review the
  scoped diff, run the remaining mandated gate sequence, then commit source and
  test changes (leaving this journal unstaged).

- 2026-08-02T13:32:00Z — The first broad gate stage is synchronously green:
  `devenv shell -- cargo run -- fmt --check examples`, warning-denied
  workspace/all-target clippy, and serial `devenv shell -- cargo test
  --workspace -- --test-threads=1` all exited 0. The workspace suite includes
  both proj1 tests under their normal shared binary scheduling; the isolated
  new fingerprint guard and a second concurrent proj1-binary run had already
  passed. Next: regenerate and drift-check the tour, repeat its deterministic
  check, then run the track-ending full flake check.

- 2026-08-02T13:46:00Z — Ignored tour regeneration, zero-diff
  `git diff --exit-code -- docs/tour`, and the committed-document test all
  passed. Its first deterministic renderer attempt then caught a stale
  workspace-marker timing failure (`workspace-state: cold` at the expected
  warm step); the test's own end-of-scenario `rm -rf` removed the isolated
  `target/test-tmp/.workspaces-run` state. Two fresh exact deterministic runs
  subsequently passed synchronously (about 24s each). This does not affect the
  output-key repair—the reproduced proj1 cause is independently the test
  process environment race—but it is recorded rather than hidden. Next: stage
  only the two source/test files for the flake source snapshot, run the full
  required flake gate, then commit if green.

- 2026-08-02T14:08:00Z — The track-ending `devenv shell -- nix flake check
  -L` is green with a synchronous, explicit `flake-check-exit=0`. The initial
  run built the staged candidate and completed the 70-check VM tier, but its
  voluminous stream exceeded the tool buffer before an exit status could be
  observed, so it was deliberately not counted. The exact rerun retained its
  log in `/tmp/composix-ergo-fix2-flake-check.log`, reported all 70 candidate
  outputs as previously built, and exited 0. All required fmt, examples fmt,
  clippy, serial workspace, proj1, tour regeneration/drift, and two fresh tour
  deterministic checks are now green. Next: final staged-scope audit and
  commit only the implementation/test files; retain this append-only journal
  unstaged.

- 2026-08-02T14:10:00Z — Committed the green repair as `7917186` (`fix:
  stabilize builder workspace regression`), containing only
  `crates/cix-cixfile/src/build.rs` and `crates/cix-cixfile/tests/proj1.rs`.
  The completed-output regression proves receipts do not affect the build
  fingerprint; the concurrently executed proj1 fixtures no longer race their
  process-global workspace root. Next: final diff/status audit; this journal
  remains deliberately unstaged per AGENTS.md.

- 2026-08-02T14:11:00Z — Final audit passed: `git diff --check` and
  `git diff --exit-code -- docs/tour` both exited 0; `7917186` is HEAD and
  `crates/cix-cixfile/LOG.md` is the sole uncommitted path. This requested
  fix round is complete.

- 2026-08-02T12:00:00Z — Started the requested ergo semantic-fix round on
  `track/ergo` at `e851b0e`. Read the complete spec, current project journal,
  and this crate journal. The merged tree already contains CIP-88's
  `LockFile.outputs` model and writer, but Chapter 3 currently disagrees with
  main in two directions: it adds the outputs receipt and loses the live
  `cix ps` row. Next: reproduce deterministic tour generation, capture the
  user-manager/probe failure synchronously, then reconcile the actual writer
  and runtime seams with regression coverage before the full gate.

- 2026-08-02T12:08:00Z — Reproduced `devenv shell -- cargo test -p cix --test
  tour -- --ignored generate_tour --nocapture` synchronously (exit 0). It
  rewrote Chapter 3 with the expected active user unit; the prebuilt lock
  retained the additive `outputs.copied-greeting = { sourceHash, storePath }`
  receipt, confirming the CIP-88 writer/reader seam is intact. Journal
  evidence identifies the apparent run failure precisely: the PrivateDevices
  probe exits `218/CAPABILITIES`, HostCapabilities drops only
  `PrivateDevices`; the full user unit then exits `226/NAMESPACE`; its existing
  namespace-degraded retry starts successfully. The actual lost row was the
  tour filter's stale assumption that UNIT is column two. Observability's
  COMPOSITE/SERVICE projection can precede it; the merged filter locates the
  known unit anywhere in the row and generation now retains `active/running`.
  Added the missing direct assertion that the rendered consumer lock contains
  `outputs`, so both the writer shape and the generated receipt are guarded.
  Next: focused tests and deterministic tour drift, then the prescribed gate.

- 2026-08-02T12:12:00Z — Focused verification is green: formatter check,
  committed-tour suite, and a second exact deterministic tour run all exited
  0. Committed the scoped repair as `fb7df76` (`fix: preserve ergo tour
  receipts`): Chapter 3 now records the real active row, and the tour asserts
  its regenerated tagged-item lock contains `outputs`. Immediately after the
  commit, `git diff --exit-code -- docs/tour` exited 0. Only this required
  uncommitted task journal remains. Next: prescribed examples formatter,
  warning-denied clippy, workspace tests, regeneration/drift, and full flake
  gate.

- 2026-08-02T12:24:00Z — Prescribed examples formatting and
  warning-denied workspace/all-target clippy both exited 0. The first serial
  workspace run exposed an intermittent existing proj1-tour assertion
  (`workspace-state: cold` after the source edit), which poisoned that
  renderer process's mutex; it is not a PrivateDevices or output-lock
  regression. A fresh exact deterministic-tour run and the isolated
  foreign-user-unit test both exited 0, and the complete serial workspace
  rerun proceeded through the same suite. Fresh ignored regeneration,
  `git diff --exit-code -- docs/tour`, and the committed-document test all
  exited 0; the second exact deterministic invocation was already proven at
  12:12. Next: full synchronous `devenv shell -- nix flake check -L`, then
  final scope/status audit and a journal-only closing entry.

- 2026-08-02T12:38:00Z — Final gates are green with synchronous observed
  status: the repeat serial `devenv shell -- cargo test --workspace --
  --test-threads=1` exited 0 (including tour, real-Nix lock, user-run, and
  doc tests), and the final `devenv shell -- nix flake check -L` exited 0 over
  all 71 checks, including vm-dogfood, compose fallback, health, dirs2,
  devices, lifecycle, observability, update/repin, side-by-side, and GC
  survival. The flake's expected KVM→TCG fallback occurred; an ignored
  eval-cache SQLite-busy warning did not affect the result. Prior formatter,
  examples formatter, warning-denied clippy, fresh tour generation, committed
  tour drift, and deterministic-tour checks are all recorded above. Final
  scope audit next: only this required, intentionally uncommitted journal
  should differ from `fb7df76`.

- 2026-08-02T12:40:00Z — Final audit passed: `git diff --check` and
  `git diff --exit-code -- docs/tour` both exit 0; `fb7df76` is HEAD and the
  sole worktree modification is this intentionally uncommitted track journal.
  The requested ergo fix round is complete.

- 2026-08-02T07:51:00Z — Committed the complete scoped regrade as `778bf54`
  (`docs: regrade corpus after feature wave`): 13 files, including the three
  empirical receipts, 51-row evidence-class sweep, refreshed migration and
  Docker gap ledgers, both rebuilt migration samples, and deterministic tour
  drift. The final fence audit confirmed that corpus rows 7/17 retained their
  mechanism cells and the devices-owned Docker rows are byte-unchanged. Only
  this required journal remains modified and deliberately uncommitted.

- 2026-08-02T07:50:00Z — Final gate is green. The complete serial workspace
  suite passed with `devenv shell -- cargo test --workspace --
  --test-threads=1`, including the committed-tour and deterministic-tour
  checks. The track-ending `devenv shell -- nix flake check -L` evaluated 61
  checks and exited 0; the Linux VM reports were green for `vm-dogfood`,
  `scenario-update-repin`, `scenario-observability`, `scenario-lifecycle`,
  `scenario-side-by-side`, `compose-fallback-vm`, and
  `scenario-gc-survival`. Expected TCG and D36 PrivatePIDs fallback warnings
  did not fail assertions. Final `git diff --cached --check` and `git diff
  --check` pass. All deliverables are staged while this required task journal
  remains deliberately unstaged per repository policy. Next: audit the staged
  fence one last time and commit the complete regrade.

- 2026-08-02T07:42:00Z — Gate through tour is green after one caught
  generated-doc drift. Exact passes: `devenv shell -- cargo fmt --all --check`;
  `devenv shell -- cargo run -- fmt --check examples`; focused cix fmt checks
  for the three changed/new corpus Cixfiles; `git diff --check`; and `devenv
  shell -- cargo clippy --workspace --all-targets -- -D warnings`. The first
  parallel `devenv shell -- cargo test --workspace` passed all non-tour groups
  but failed `tour_matches_committed_document`: current generation renders the
  first Chapter 3 `cix ps` as its empty-table projection, while the committed
  page retained the earlier service row. After clearing stale user-manager
  failed units, the required ignored regeneration passed twice with the same
  one-hunk doc update. Focused committed-document then passed, and
  `generated_tour_is_deterministic` passed twice serially. This is accepted
  generated drift, not hand-edited prose. Next: obtain a complete serial
  workspace-test pass, then run the full `nix flake check -L` gate.

- 2026-08-02T07:36:00Z — Added the grade-flipping logging receipt rather
  than leaving nginx desk-only. Updated its stale bare START to the locked
  `${pkgs.nginx}` executable and set `error_log stderr info` in the existing
  quote-aware `-g` argv. Exact successful repro: `target/debug/cix build -t
  regrade corpus/migrate/nginx` produced
  `/nix/store/s35rsvbhr2hi9qmm1wpj4bibgl3nssvz-cix-item-nginx`; a scratch
  compose check passed; root tag + `sudo env PATH="$PATH" target/debug/cix up
  .dev/scratch/regrade/nginx-compose.json --update='*'` activated generation
  `/nix/store/ih1xx84mpvnwggfwwdcaqkhc0qvysqw6-cix-compose-corpus-nginx-generation`;
  bounded HTTP GET passed; invocation-scoped `cix logs corpus-nginx/nginx`
  returned nginx startup and request records through CIX_COMPOSITE/CIX_SERVICE.
  Service RESULT remained success under the expected D36 fallback. Down and
  both tag cleanups passed. The updated nginx receipt has the exact transcript;
  its corpus row is now receipt-backed. Next: final diff review and full gate.

- 2026-08-02T07:31:00Z — Completed the ledger rewrite. `docs/corpus.md`
  now has an Evidence column on all 51 ribbons, a 2026-08-02 maintained-per-track
  status, re-ranked met/queued demands, and refreshed candidate dispositions.
  Redis and the narrow Renovate timer/log shape were the receipts at this
  milestone; nginx was added next, leaving 48 rows explicitly `desk`. `docs/migrate.md` now
  distinguishes arbitrary role backing from queued `DIR` materialization,
  teaches current START/CLAIM/CIP-79/schedule/logging translations, and keeps
  unbuilt health/host binds incomplete. `docs/docker.md` now reflects CIP-79,
  CIP-81, CIP-82 lifecycle/materialization, CIP-83, and the Docker/Nix profile
  homonym honestly. Devices-owned corpus mechanism cells 7/17 and docker
  `--device`/`--gpus`/`--shm-size`/`--group-add`/tmpfs rows are byte-untouched;
  their regrade is pending CIP-78 impl (in flight). Exact docs-sample proof:
  extracted both complete code fences to ignored `.dev/scratch/regrade`, two
  `awk ... | diff -u` comparisons passed byte-for-byte, then `devenv shell --
  target/debug/cix build .dev/scratch/regrade/dissolve` produced
  `/nix/store/dn8hl026i2b11qpwh8pyl8dls40gi3jy-cix-item-web` and the same command
  for `fetch` produced
  `/nix/store/5c6f9kz9z18m91sjf9fz3kj02d9yhy08-cix-item-readme` after its declared
  EXPECT fetch. Deliberately not re-verified: the other 49 corpus ports, any
  Docker side, health probes (CIP-79 queued), operator/shared compose dirs
  (CIP-82 leg 2 queued), Renovate credentials/config, and the devices fence.
  Next: format/review the scoped diff, then run the complete required gate.

- 2026-08-02T07:20:00Z — Empirical subset milestone: updated the existing
  Redis migration to preserve Docker's `/data` path as `STATEDIR /data`, then
  ran `devenv shell -- cargo build -p cix`, `target/debug/cix build
  corpus/migrate/redis#redis`, a root transient run, bounded TCP PING, and
  `cix inspect --runtime`. Final item
  `/nix/store/0zd94c03qk3gddgg01cwaznwgcywiap2-cix-item-redis` returned `+PONG`;
  inspect showed `/var/lib/private/cix-run-redis/data`, proving CIP-82 leg 1's
  full mirror. The first attempt also caught a stale bare START path, corrected
  to the locked store executable. Added a Renovate-shaped APP/compose fixture:
  build and `systemd-analyze calendar daily` passed; `cix compose check` passed;
  root activation with the corrected `--update='*'` lock produced active
  `cix-corpus-renovate-renovate.timer` with `OnCalendar=daily` and
  `Persistent=true`. Its first run caught the real Node W^X need; after `CLAIM
  jit`, the current invocation exited 0 and `cix logs ... --invocation ...`
  returned Renovate `43.214.1` through the indexed selectors. The exact
  successful/failure transcript and limits are in both receipts. Temporary
  compose units and root/user tags were removed. Next: finish the three ledger
  rewrites, independently rebuild both migrate.md samples, and record the rows
  deliberately left as desk grades.

- 2026-08-02T07:00:00Z — Started track/regrade on the clean `track/regrade`
  branch. Read the current project journal, authoritative design registry,
  complete track spec, this crate journal, and the current corpus/migrate/docker
  ledgers. The devenv/direnv environment is active. Scope is a one-time honest
  regrade after D47/D74 and CIP-75/76/80/82/83, with explicit desk-vs-receipt
  evidence, a small grade-flipping empirical subset, refreshed demands/example
  candidates, and no edits to devices-owned corpus rows 7/17 or docker device,
  GPU, shm, group-add, or tmpfs rows. Next: inventory stale claims, choose the
  empirical subset, and build/run the cheapest representative conversions.

- 2026-08-02T02:30:00Z — Committed CIP-88 as `0fec3ce` (`feat: improve
  builder ergonomics`). The commit includes stats, completed-output receipts,
  vendored builder dev environments, lock metadata interpolation, gitsitter
  simplification, hermetic regression coverage, and refreshed documentation.
  The prescribed format, examples, clippy, workspace, tour, and full flake
  gates are green; `git diff --check HEAD` is clean. This task journal remains
  deliberately unstaged; no other worktree changes remain.

- 2026-08-02T00:20:00Z — Mapped the current build path for CIP-88. The
  full-hit path currently resolves Nix contexts before consulting memo entries,
  and hashes/re-adds immutable store paths; the implementation will add a
  lock-recorded completed-output fast path, invocation stats, vendored
  `print-dev-env` snapshots, and lock metadata interpolation without changing
  the concurrent parser work for CLAIM/SHM. Next: implement the lock/model
  seams, then exercise focused tests before the full gate.

- 2026-08-02T01:15:00Z — Implemented the CIP-88 core: `--stats` JSON,
  in-process host-system detection, completed-output receipts for a zero-Nix
  full hit, lock-resolved FROM metadata, IMPORT dev-env snapshots, and the
  16-MiB informational FETCH-complement note. The simplified gitsitter
  fixture builds with `${src.rev}` and the vendored `PKG_CONFIG_PATH`; a warm
  receipt reported zero Nix subprocesses. Added a local-HTTP FETCH regression
  proving normal/repeat/`--cold` convergence and the no-op assertion. Focused
  parser and mini-fixture tests are green. Next: format/docs review, broader
  workspace checks, prescribed tour/flake gate, then commit.

- 2026-08-02T02:20:00Z — Final verification is green: `cargo fmt --all
  --check`; `cargo run -- fmt --check examples`; warning-denied workspace
  clippy; serial workspace tests; tour regeneration, committed-doc drift, and
  deterministic rerun; and `devenv shell -- nix flake check -L` (full VM
  fleet, expected KVM→TCG fallback). The first combined workspace run exposed
  the known shared tour renderer path; regeneration and serial reruns passed.
  A duplicate flake invocation was stopped while the original completed. Final
  `git diff --check` is clean. Next: stage scoped changes except this ignored
  journal, commit on `track/ergo`, then verify the commit.

- 2026-08-02T00:00:00Z — Started track/ergo (CIP-88). Read AGENTS.md, the
  session journal, complete track specification, and authoritative CIP-88.
  Scope: `cix build --stats`, zero-subprocess full memo hits, automatic
  pkg-config paths, FROM lock metadata interpolation, simplified gitsitter
  fixture, hermetic regression, docs, full prescribed gate, and a scoped
  commit. The requested journal is tracked on this branch despite the global
  ignore preference; it will remain deliberately unstaged.

- 2026-08-01T22:15:00Z — Committed the complete CIP-80 sweep as `cd6cd99`
  (`feat: rename exec to start`). The commit has 140 scoped files and leaves
  this journal deliberately unstaged; `git status` contains no other worktree
  changes. All required gates in the preceding entry apply to this commit.

- 2026-08-01T22:10:00Z — Final corrected gate is green. Exact repros:
  `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo run -- fmt
  --check examples`; warning-denied workspace clippy; serial workspace tests;
  ignored tour regeneration plus committed-doc drift; and the full `devenv
  shell -- nix flake check -L` (89 checks, all VM/scenario checks green under
  expected TCG fallback; final recorded exit status 0). `git diff --check` is
  clean. Final `rg -iw 'exec|setup'` is confined to migration parser/fixtures,
  `ExecStart*`/`Type=exec` and cix-exec implementation, health's independent
  `health.exec`, shell command text, corpus Docker/source material, and
  historical design/CIP/journal prose. The active Cixfile and v0-manifest
  surfaces contain only START/START_PRE and start/start_pre. Next: remove the
  generated devenv lock, stage all scoped changes except this task journal,
  commit on track/start, and verify the commit.

- 2026-08-01T21:25:00Z — Implemented the CIP-80 language and v0 schema
  sweep. Parser accepts START/START_PRE, rejects EXEC/SETUP with standard
  migration diagnostics, and has new torture fixtures for both legacy words
  plus STRAT→START. The model, codegen, runner, compose, VM fixtures,
  examples, corpus Cixfiles/receipts, and active docs now use start/start_pre;
  systemd retains `ExecStart=`, `ExecStartPre=`, and `Type=exec`. Bumped the
  fingerprint to d80-v1. Focused cargo check plus cixfile diagnostics/parser,
  runner, build, and compose tests pass; the ignored tour regeneration passes.
  Historical CIP prose was deliberately restored. Next: tour drift checks,
  full prescribed gates, final case-insensitive inventory triage, and commit.

- 2026-08-01T21:00:00Z — Started track/start (CIP-80) after confirming the
  timer and watch sequencing precondition is present in this branch. Read
  AGENTS.md, the current project/crate journals, D72, and CIP-80 §5. Scope is
  the alpha-only EXEC→START / SETUP→START_PRE rename across the Cixfile,
  manifest v0, all consumers and docs, with crunchy migration diagnostics and
  d80-v1 cache invalidation. Next: map active consumers and implement the
  parser/model/codegen seam before sweeping fixtures and documentation.

- 2026-08-01T20:40:00Z — Committed the scoped rename as `b8e8e91` (`feat:
  rename grants to claims`). The worktree is clean apart from this intentionally
  uncommitted, ignored task log. All prescribed gates are recorded above.

- 2026-08-01T13:45:00Z — Final gates green after staging new source files before the Nix source snapshot: `cargo fmt --all --check`; `devenv shell -- cargo run -- fmt --check examples`; warning-denied workspace clippy; serial workspace tests; tour regeneration and deterministic drift check; and the full `devenv shell -- nix flake check -L` (61 checks, including VM dogfood/compose/scenarios). `devenv` generated an untracked lock during gates and it was removed. `git diff --check` is clean. Next: commit staged implementation; keep this ignored journal unstaged.

- 2026-08-01T13:20:00Z — Implemented CIP-76 watch loop. `cix watch [PATH]` now uses notify recursively with 300ms coalescing (`CIX_WATCH_DEBOUNCE_MS` is the hidden test override), polling fallback on notify setup failure, and a Ctrl-C handler returning success. Bare contexts warm-build and print item paths. Compose contexts map changed local Cixfiles to their service members, retag only rebuilt local items, and call a new internal multi-service `UpdateRequest::Services` so activation resolves/restarts only those services. Ignore matcher covers `.git`, all target dirs, Cixfile/compose locks including atomic temp writes, configured/default builder workspaces, and nested `.gitignore` rules. A CLI fixture edits a real context, sees exactly one build, then proves lock/workspace writes do not self-trigger; ignore and selective-resolve unit coverage also pass. The initial notifier test exposed access-event debounce starvation and a ready-message race; both were corrected. Docs now state the artifact-loop vs `nix develop` split and sync refusal. Next: full prescribed gate, tour regeneration/drift, flake check, review, commit.

- 2026-08-01T12:00:00Z — Started track/watch (CIP-76). Read AGENTS.md, the current project journal, authoritative `docs/cips/0076-devloop.md`, and the current Cixfile/compose CLI and activation paths. Scope: a noisy `cix watch [PATH]` with a debounced, context-aware notify watcher; bare Cixfile rebuild reports its item; compose delegates to the existing `cix up` restart-changed activation path. Ignore rules must cover `.git`, builder workspaces (configured/default), `Cixfile.lock`, `target`, and `.gitignore`; tests will prove scripted one-round rebuild and no self-trigger. Next: design the small reusable watcher surface and determine the compose source-to-member rebuild mapping.

- 2026-08-01T20:35:00Z — Full gate green: `devenv shell -- nix flake check
  -L` completed all 63 checks, including VM dogfood, compose fallback, and the
  scenario fleet under expected TCG fallback. Final diff check is clean.
  Exact successful repros: `devenv shell -- cargo run -- fmt --check examples`;
  `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo clippy
  --workspace --all-targets -- -D warnings`; `devenv shell -- cargo test
  --workspace`; `devenv shell -- cargo test -p cix --test tour -- --ignored
  generate_tour`; and the committed-doc and determinism tour tests serially.
  The one generated `devenv.lock` was moved to Trash. Next: stage all scoped
  changes except this ignored task log, commit on track/claim, and verify.

- 2026-08-01T20:25:00Z — Implementation and focused checks are green. Parser
  now accepts CLAIM, treats the old directive as a migration error, and the
  new 61/62 torture fixtures cover that error plus the CLAM typo suggestion.
  Model/codegen/runner use `claims`; the runner field carries the required
  reserved-name comment, and the codegen fingerprint is d78-v1 so cached old
  outputs cannot survive. Focused parser/diagnostics/proj1, runner, and
  compose tests passed; `cargo run -- fmt --check examples`, fmt, and
  warning-denied clippy passed; workspace tests and tour regeneration passed.
  The first committed-tour assertion hit the known shared tour-workspace
  deletion race, then passed serially; serial determinism passed too. The
  remaining case-insensitive inventory is confined to historical journals,
  CIP/design history, the reservation comment, migration implementation and
  regression fixture/test literals, plus LICENSE boilerplate. Next: full
  flake gate, final scope/inventory review, then commit.

- 2026-08-01T20:05:56Z — Started track/claim. Read AGENTS.md, the session
  journal, CIP-78 §5, D72, the complete track spec, and this crate journal.
  Scope is the alpha rename only: Cixfile `CLAIM egress`/`CLAIM jit`, manifest
  `claims`, migration diagnostics for the old spelling, all active
  consumers/docs/tour/fixtures, and the prescribed full gate. The initial
  case-insensitive inventory has 191 matches, including historical journals,
  CIP/design history, and LICENSE boilerplate; active implementation is still
  to be mapped and converted. Next: update the language and manifest seams,
  then sweep tests, examples, documentation, and generated tour output.
- 2026-08-01T02:20:00Z — Resumed track/decompose. Converted every active
  runner, VM, scenario, fixture, and test manifest to cixManifest 0's bare
  def-node shape (the only nonzero in the tree is the deliberate unsupported
  version-99 rejection fixture). Parser extraction is now substantive:
  machine/directives/migrations/validate modules own their respective code and
  the former in-file parser suite lives in tests/parser.rs. Index implementation
  is split into refs/tags/roots/serve/pull. Focused cargo checks are in progress;
  next is complete workspace tests, tour regeneration, tokei receipt, then the
  prescribed full flake gate and commit.

- 2026-08-01T02:45:00Z — Tokei (crates) before/after receipt:

  | scope | files | lines | code |
  | --- | ---: | ---: | ---: |
  | baseline Rust | 42 | 22,059 | 20,746 |
  | after Rust | 53 | 21,656 | 20,353 |
  | parser facade + modules + external tests | 7 | 3,069 | 2,906 |
  | index facade + refs/tags/roots/serve/pull | 6 | 1,666 | 1,569 |
  | cix-build Rust sources | 6 | 5,211 | 4,380 |

  The requested visible drop is 403 total Rust lines / 393 code lines despite
  the new crate and explicit module seams. Focused cix-run, cix-cixfile parser,
  and cix-index tests are green; workspace clippy is green. The regular parallel
  workspace tour test hit the known transient user-manager namespace race; its
  focused serial reruns reached the same flaky listener fixture and need a clean
  successful run before the tour/gate claim.

- 2026-07-31T00:00:00Z — Started `.dev/specs/track-decompose.md` on
  `track/decompose`. Read AGENTS.md, the current session journal, authoritative
  D72/D73 (including D73's diagnostics addendum), the complete track spec, and
  this crate journal. Baseline `devenv shell -- tokei crates`: Rust 42 files,
  22,059 lines / 20,746 code (tokei reports embedded Markdown separately).
  Scope is mechanical only: cix-build extraction, parser/index module seams,
  v0-only manifest validation, complete manifest sweep, unchanged crunchy
  diagnostic snapshots, and the prescribed full gate. Next: map exact module
  APIs and perform the moves without changing diagnostics.

- 2026-07-31T00:20:00Z — Mechanical crate/module seams are in progress:
  `cix-build` now owns the workshop implementation (chain, lock, model,
  sandbox, build expression helpers) and `cix-cixfile` re-exports its language
  API; parser and index now have their requested facade/module paths. Focused
  `cargo check --workspace`, the full Cixfile parser suite, and the unchanged
  crunchy diagnostics snapshots pass. The v0-only runner parser and codegen
  emission are implemented; the first full workspace test sweep exposed stale
  test/tour/VM manifests still using pre-v0 multi-service shapes. Converted
  the simple Cixfile/tour/compose fixtures and am continuing the complete
  current-shape sweep before regeneration. No gate or commit claim yet.

- 2026-08-01T01:30:00Z — Final D69 correction gate is green. `cargo fmt --all
  --check` and warning-denied workspace clippy passed. The regular parallel
  workspace command twice exposed the pre-existing fixed-name transient-user
  test race (`tour_ignores_a_foreign_user_unit`: systemd still had the prior
  transient unit loaded); after user-manager reset/reload, the complete same
  suite passed serially with `devenv shell -- cargo test --workspace --
  --test-threads=1`. Focused test coverage passed: the timestamped npm log
  lock-equality integration test plus `volatile_facts` boundary unit test.
  Cold-audit remains absent (the prescribed `rg` finds design/journal prose
  only). Tour generation, zero drift, committed-doc matching, and two
  determinism runs passed. VM dogfood was included in the full `devenv shell
  -- nix flake check -L` run: it evaluated 61 checks and completed the current
  source's VM/scenario fleet under expected TCG fallback; a cached second
  invocation completed its remaining seven checks. ProjB double refresh plus
  ordinary memo hit passed and removes its unconsumed `.cargo/.global-cache`
  fact. Dozzle completed its disposable probe with no persisted volatile
  facts. `git diff --check` is clean. Next: final scope review, stage, commit.

- 2026-08-01T00:45:00Z — Root cause confirmed and corrected. The automatic
  pin's seven `paths` were already downstream COPY outputs; the changing lock
  entry came from `execute_builder` serializing the unfiltered
  `volatile_paths` double-FETCH observation through `refresh_fetch_pin`.
  `consumed_volatile_paths` now retains probe facts only below a consumed path;
  raw facts still print. Focused regression creates npm-style timestamped
  `_logs` output after a `find` readdir and proves two update-locks have equal
  bytes and no persisted unconsumed volatile fact. Exact Parse Server proof:
  built `target/debug/cix`, ran `CIX_BUILD_WORKSPACE_DIR=<fresh-one>
  TMPDIR=<fresh-/var/tmp> ../../../target/debug/cix build --update-lock build
  .#parse-server`, copied the lock, then ran the same command with a distinct
  fresh workspace (equivalent to a wiped first workspace); `cmp -s` passed.
  Final lock sha256 is
  `609a7e76aa891c2b66aaec11e0e5c81f43ec509636e1ca2b58bce8508d915bde`,
  carries precisely the seven service paths, and has `volatile_count: 0`.
  The four explicit /var/tmp repro artifacts were moved to Trash. Next: full
  prescribed gate plus `devenv shell -- nix flake check -L`, then commit.

- 2026-08-01T00:30:00Z — D69 stricter stability proof reopened: wiping
  `CIX_BUILD_WORKSPACE_DIR` between two clean Parse Server `--update-lock`
  runs leaves one changing lock entry, npm's timestamped debug-log path. The
  seven automatic pin paths themselves are stable; diagnosis points to the
  serialized double-fetch `volatile` facts, which currently include the FETCH
  command's own read/rotation output. Next: restrict those facts to paths
  consumed by later steps or artifact COPYs, add a regression, then run the
  exact wipe repro and full gates including `nix flake check -L`.

- 2026-08-01T00:15:00Z — Follow-up D69 correction final gate is green. Exact
  Rust commands: `devenv shell -- cargo fmt --all --check`; `devenv shell --
  cargo clippy --workspace --all-targets -- -D warnings`; `devenv shell --
  cargo test --workspace`; focused `devenv shell -- cargo test -p cix-cixfile
  --test lock_nix -- --nocapture`. No cold-audit executable exists on this
  base (the prescribed `rg -n 'cold_audit|cold-audit' . --glob '!target/**'
  --glob '!corpus/migrate/**/context/**'` finds only design/log prose). Tour
  gate, with `TMPDIR=/dev/shm` to avoid the shared `/tmp` inode limit: ignored
  regeneration, `git diff --exit-code -- docs/tour`, committed-doc matching,
  and deterministic generation passed twice. VM command `TMPDIR=/dev/shm
  devenv shell -- nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`
  passed under expected TCG fallback; receipt
  `/nix/store/g4m1zwpm319lqfc80k3wrjbfk41pmhxi-vm-test-run-vm-dogfood` exists.
  Exhibits: Parse Server double-clean byte equality plus fresh `--cold` replay;
  ProjB double-clean byte equality plus ordinary memo hit; Dozzle retains its
  recorded missing-cert/UI failure and no failed refresh lock change. Final
  `git diff --check` / cached check and system/user `cix-*` cleanup pass.
  Next: stage scoped correction and commit on `track/pinkeys`.

- 2026-07-31T23:50:00Z — Fixed the D69 lock churn: new `FetchPin.storePath`
  values deserialize only as legacy compatibility data and are never serialized;
  full replay snapshots live in `$XDG_CACHE_HOME/cix/fetch-snapshots`, keyed by
  the stable pin plus Cixfile directory/FETCH id. `--cold` restores that local
  snapshot and never refetches (missing cache is a clear failure). Restored
  snapshots are made writable so later offline RUN steps retain normal builder
  semantics. Focused `lock_nix` is green, including mutated restored input and
  double-`--update-lock` byte equality. Exact real acceptance: two clean Parse
  Server commands with separate fresh `CIX_BUILD_WORKSPACE_DIR` values and
  `TMPDIR=/tmp` produced byte-identical `Cixfile.lock` (`sha256
  1e5a2a6f69f716245fc1434b6b0a064165518951c2511fe21ddc9be1e4ed9bb2`),
  and no fetch `storePath`; a further fresh-workspace Parse Server `--cold`
  replayed step 5 then ran both offline suffix RUNs successfully. One initial
  cold attempt hit `/tmp`'s inode limit (not a product failure); removed only
  the three exact temporary workspaces created for this reproduction, then the
  retry passed. Next: prescribed workspace/tour/VM/exhibit gate and commit.

- 2026-07-31T23:30:00Z — Independent D69 verification found the required
  double-clean `--update-lock` acceptance failing: Parse Server's automatic
  consumed-path hashes were stable, but `fetches.*.storePath` still named the
  complete volatile `.npm` workspace snapshot and therefore churned. Fix in
  progress: do not serialize automatic replay snapshot paths into
  `Cixfile.lock`; retain a local replay cache keyed by the stable fetch pin so
  `--cold` stays offline, with an explicit no-refetch error if that local
  replay data is unavailable. Next: add focused coverage, run the exact
  double-clean Parse Server repro, then rerun the prescribed gate.

- 2026-07-31T23:05:00Z — Final D69 gate is green. Exact repros: `devenv shell
  -- cargo fmt --all --check`; `devenv shell -- cargo clippy --workspace
  --all-targets -- -D warnings`; and `devenv shell -- cargo test --workspace`
  (exit 0). No `cold_audit` executable exists on this base (`rg -n
  'cold_audit|cold-audit' . --glob '!target/**' --glob
  '!corpus/migrate/**/context/**'` finds the D69 design mention only). Tour
  repros: `devenv shell -- cargo test -p cix --test tour -- --ignored
  generate_tour`; `git add docs/tour && git diff --exit-code -- docs/tour`;
  `devenv shell -- cargo test -p cix --test tour
  tour_matches_committed_document -- --exact`; and `devenv shell -- cargo test
  -p cix --test tour generated_tour_is_deterministic -- --exact` (twice), all
  passed. The dogfood VM repro `devenv shell -- nix build
  .#checks.x86_64-linux.vm-dogfood --no-link -L` passed; receipt confirmed with
  `nix path-info /nix/store/ssxw6w6gx1pivkalbzy220x6xydpyp16-vm-test-run-vm-dogfood`.
  Exhibit repros: Parse Server's two `cd corpus/migrate/parse-server &&
  ../../../target/debug/cix build --update-lock build .#parse-server` plus
  `../../../target/debug/cix build .#parse-server`; ProjB's `target/debug/cix
  build --update-lock build examples/build/projB#projb` twice plus `target/debug/cix
  build examples/build/projB#projb`; and disposable Dozzle backend's
  `/home/mathijs/composix/.worktrees/pinkeys/target/debug/cix build --update-lock
  build .` twice plus `.../target/debug/cix build .` all passed. `git diff
  --check` and `git diff --cached --check` pass. Cleanup repro: `systemctl
  --user stop 'cix-*' >/dev/null 2>&1 || true; systemctl --user reset-failed
  'cix-*' >/dev/null 2>&1 || true; systemctl --user stop cix-run.slice
  >/dev/null 2>&1 || true; sudo -n systemctl stop 'cix-*' >/dev/null 2>&1 ||
  true; sudo -n systemctl reset-failed 'cix-*' >/dev/null 2>&1 || true; sudo -n
  systemctl daemon-reload; ! sudo -n systemctl list-units 'cix-*' --all
  --no-legend --plain | grep -q .; ! systemctl --user list-units 'cix-*' --all
  --no-legend --plain | grep -q .` passed. Next: stage this scoped diff and
  commit `track/pinkeys`.

- 2026-07-31T22:25:00Z — D69 focused implementation is green: automatic pins
  now record consumed `paths` plus a replayable `storePath`; declared EXPECT
  remains a whole-tree `narHash`; `--cold` replays both builder and top-level
  FETCH snapshots without executing FETCH; `--update-lock` double-runs automatic
  FETCH and records/reports volatile name+size facts; memo/chain keys include
  `cix-cixfile`'s D69 codegen fingerprint (`0.1.0:d69-v1`), causing the expected
  one-time global memo miss. Exact focused repros: `devenv shell -- cargo fmt
  --all`; `devenv shell -- cargo test -p cix-cixfile --lib`; `devenv shell --
  cargo test -p cix-cixfile --test lock_nix -- --nocapture`; and targeted
  `lock_nix` tests `automatic_fetch_pins_only_consumed_paths_and_cold_replays_its_snapshot`,
  `cold_replays_a_top_level_fetch_snapshot_without_executing_fetch`,
  `newly_consumed_fetch_path_extends_an_automatic_pin`, and
  `update_lock_double_fetch_records_volatile_files_without_pinning_them` (all
  passed). Fresh `parse-server` context was fetched with `cd corpus/migrate &&
  bash ./fetch.sh parse-server`; `../../../target/debug/cix build --update-lock
  build .#parse-server` completed twice and records the known `.npm` volatility
  while its consumed map matches the seven final service paths. The first attempt
  hit temporary inode exhaustion; removed only five explicitly verified-unheld,
  stale `/tmp/cix-build-cold-*` Cix workspaces (no repo/Nix-store content).
  Next: finish corpus receipts (ordinary parse build/projB/dozzle), full gate,
  then commit.

- 2026-07-31T00:00:00Z — Started `.dev/specs/track-pinkeys.md` on
  `track/pinkeys`. Read AGENTS.md, the session journal, authoritative D69(a,b,c,e)
  in full, the diagnosis report, the complete track spec, and this crate journal.
  Scope: automatic FETCH consumed-set pins; offline `--cold` replay from pinned
  fetch snapshots; `--update-lock` double-fetch volatile-file probe; codegen
  fingerprinting; docs and honest corpus receipts. Expected compatibility effect:
  the new fingerprint creates a one-time global memo miss. Next: map the lock and
  build-chain seams, implement with focused tests, then run the prescribed gate.

- 2026-07-31T22:17:34Z — Final nixcompare gate is green with the corrected,
  self-contained warm benchmark. Exact fixture repros passed in sequence:
  `devenv shell -- nix build
  github:mathijshenquet/gitsitter/29c8a2dede19b5e7d1bd7e65f81829fa0ac66ecd
  --no-link -L`; `devenv shell -- nix build
  path:./examples/compare/gitsitter/crane --no-link -L`; and `devenv shell --
  target/debug/cix build examples/compare/gitsitter/cix#gitsitter`, which
  returned `/nix/store/fniw9p1i9k9xchyzak5clj66vpxn8vgy-cix-item-gitsitter`.
  The required untouched smoke `devenv shell -- cargo test --workspace` passed
  from the start, including 44 compiler unit tests, 18 real-Nix compiler tests,
  proj1, serve/pull, runner, and tour drift/determinism coverage. The committed
  `measure-warm.sh` full replay passed with exactly three receipt lines:
  upstream 30.64 s, crane 16.46 s, Cix 20.28 s. Doc drift audits diffed the
  displayed upstream/crane excerpts and complete Cixfile against their source
  files with no differences; logical LOC re-counted as 38/30/17. `bash -n`,
  executable-mode, `git diff --check`, and 28 balanced Markdown fence markers
  passed. Scope is the requested `docs/nix-build.md`,
  `examples/compare/gitsitter/**`, plus this explicitly required track log; no
  crate source changed. Removed only the generated untracked `devenv.lock` and
  four exact temporary benchmark/distribution directories. Next: stage the
  audited scope, commit on `track/nixcompare`, and verify the committed tree.

- 2026-07-31T22:16:00Z — Corrected the warm-edit receipt after making the
  committed harness independent of prior store/workspace state. The original
  8.16 s Cix number was contaminated: its “prime” accepted the committed
  remote-build memo and did not establish local predecessor workspaces. The
  final `measure-warm.sh` deletes only the copied final memo, retains the input
  and FETCH pins, performs an untimed isolated local prime, then applies the
  committed patch. A complete green replay reported upstream 30.64 s, crane
  16.46 s, and Cix 20.28 s. The honest result is crane fastest on this source
  edit, Cix roughly one-third faster than upstream, and no 8.16 s claim. Two
  earlier harness failures (Nix `--rebuild` before first realization, then
  read-only copied source modes) produced no accepted measurements; the final
  script realizes patched Nix targets before timed checks and makes only its
  temporary Cix copy writable. Next: update the positioning prose with this
  corrected receipt, audit sample drift and scope, then run the all-three-build
  and workspace gates.

- 2026-07-31T21:57:48Z — Completed the dated x86_64-linux measurement matrix
  on this 32-thread NixOS host. Exact no-op results were upstream 0.07 s,
  crane 0.64 s, Cixfile 1.13 s. Subject-cold results (pre-existing toolchain/
  native inputs, substitutes disabled for the rebuilt subject derivations)
  were upstream 28.82 s, crane 37.81 s, and `cix build --cold` 26.94 s.
  The committed one-line `warm.patch`, after priming each route at the
  unpatched source, measured upstream 30.58 s, crane 15.94 s, Cixfile 8.16 s.
  Result sizes from `nix path-info -sSh`: upstream 9.0 MiB/63.5 MiB NAR/
  closure, crane 9.1 MiB/63.6 MiB, Cix ITEM 9.1 MiB/9.1 MiB. Determinism:
  upstream and crane final derivation `--check` rebuilds retained identical
  paths/NAR hashes; normalized Cix vendoring made warm and `--cold` converge
  on `/nix/store/fniw9p1i9k9xchyzak5clj66vpxn8vgy-cix-item-gitsitter`.
  Two caveats are receipts, not wins: crane's internal 154 MiB cargo-artifact
  archive failed `--check` although its shipped result reproduced; and the Cix
  ITEM reports zero references despite `ldd` naming store-linked glibc,
  libgit2, OpenSSL, and libgcc, so its apparent closure-size win is a missing-
  reference/distribution gap. The stratum-3 local-serve flow itself passed:
  single-member `-t measured` needed no namespace, `serve --with-store`, and a
  separate consumer index `pull --as` all succeeded; a second local Nix store
  attempt failed in pull's path-info JSON parser, so the doc must describe the
  shared-store receipt exactly. Next: write `docs/nix-build.md` around these
  outputs and limitations, audit every claim/command against committed files,
  then run the prescribed workspace and all-three-build gate.

- 2026-07-31T21:42:00Z — The three pinned baseline routes are focused-green
  for gitsitter `29c8a2d` with nixpkgs `9cf7092`. Inspection proved that the
  upstream baseline uses `rustPlatform.buildRustPackage` with Cargo.lock,
  pkg-config/git, and openssl/libgit2/sqlite rather than a low-level handwritten
  derivation. The new idiomatic crane fixture uses `buildDepsOnly` plus
  `buildPackage`; its first attempt exposed that gitsitter's workflow test needs
  `git` at check time, matching upstream's declared native input, and passed
  after adding it. Exact focused repros now pass: `devenv shell -- nix build
  github:mathijshenquet/gitsitter/29c8a2dede19b5e7d1bd7e65f81829fa0ac66ecd
  --no-link -L`; `devenv shell -- nix build
  path:./examples/compare/gitsitter/crane --no-link -L`; and `devenv shell --
  target/debug/cix build examples/compare/gitsitter/cix#gitsitter`, which
  produced `/nix/store/l7r5d2d4jc5jx7wf6rjk6q1pj30xm7q4-cix-item-gitsitter`.
  The Cixfile uses the required remote source binder, a pinned FETCH, offline
  Cargo RUN, and a D68 ITEM. Next: validate locks/output shape, add the
  reproducible one-line patch fixture, then run the controlled measurement
  matrix and distribution flow.

- 2026-07-31T21:34:18Z — Started `.dev/specs/track-nixcompare.md` on
  `track/nixcompare` at `4193e20`. Read AGENTS.md, the current session and
  Cixfile journals, authoritative D62/D65/D67/D68 in full, and the complete
  track spec. Scope is documentation and committed comparison fixtures only:
  upstream gitsitter flake versus an idiomatic crane flake versus a stratum-2
  manifest-less Cixfile ITEM, with dated reproducible authoring/timing/closure/
  determinism receipts and an honest stratum-3 distribution walkthrough. The
  branch-local environment probe `devenv shell -- true` passed. Next: inspect
  the resolved upstream source/flake and establish a reproducible benchmark
  protocol before implementing the two local routes.

- 2026-07-31T21:57:00Z — Committed the green D68 implementation on
  `track/itemrevive` (`Implement D68 manifest-less ITEM trees`). No open items
  remain for this track.

- 2026-07-31T21:55:00Z — Final D68 gate is green. Exact prescribed repros:
  `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo clippy
  --workspace --all-targets -- -D warnings`; `devenv shell -- cargo test
  --workspace`; `devenv shell -- cargo test -p cix --test tour -- --ignored
  generate_tour`; `git add docs/tour && git diff --exit-code -- docs/tour`;
  `devenv shell -- cargo test -p cix --test tour
  tour_matches_committed_document -- --exact`; and `devenv shell -- cargo test
  -p cix --test tour generated_tour_is_deterministic -- --exact` (passed
  twice). The dogfood VM repro `devenv shell -- nix build
  .#checks.x86_64-linux.vm-dogfood --no-link -L` passed under normal TCG
  fallback after KVM denial; receipt confirmed with `nix path-info
  /nix/store/c0fiy75v3ysgdpag3s5cnbp83fqi9yh9-vm-test-run-vm-dogfood`.
  Standalone example repro: `devenv shell -- target/debug/cix build
  examples/build/item#welcome-assets` produced
  `/nix/store/6gj8yjw457z35sr1l7rk2pv3x47qgn5d-cix-item-welcome-assets`;
  `test -f` on its declared files plus `test ! -e` on
  `cix-manifest.json` passed. `git diff --check` and `git diff --cached
  --check` pass. Cleanup repro: `sudo -n systemctl stop 'cix-*' >/dev/null
  2>&1 || true; sudo -n systemctl reset-failed 'cix-*' >/dev/null 2>&1 ||
  true; sudo -n systemctl daemon-reload; systemctl --user stop 'cix-*'
  >/dev/null 2>&1 || true; systemctl --user reset-failed 'cix-*' >/dev/null
  2>&1 || true; systemctl --user stop cix-run.slice >/dev/null 2>&1 || true;
  ! sudo -n systemctl list-units 'cix-*' --all --no-legend --plain | grep -q
  .; ! systemctl --user list-units 'cix-*' --all --no-legend --plain | grep
  -q .` passed. Removed only the untracked devenv-generated `devenv.lock`.
  Next: stage the scoped D68 diff and commit it on `track/itemrevive`.

- 2026-07-31T21:35:00Z — D68 implementation and focused proof are green.
  `ArtifactKind::Item` reuses named assembly, selection, tagging, and D65 source
  binders, but writes neither a spec nor `cix-manifest.json`; runtime vocabulary
  receives one D68 seam diagnostic. `Spec::load` detects manifest-less paths so
  `cix run` and `cix debug` surface the same actionable boundary before any
  runtime work. The real-Nix regression is ITEM producer → tag → lock-pinned
  FROM → COPY consumer and also proves the producer has no manifest. Added
  `examples/build/item`, reference/migration table rows, and a tour build/tag/
  consume/run-refusal transcript. Exact focused repros: `devenv shell -- cargo
  fmt --all`; `devenv shell -- cargo test -p cix-cixfile --lib`; `devenv shell
  -- cargo test -p cix-run --lib`; `devenv shell -- cargo test -p cix-cixfile
  --test lock_nix cix_item_from_copies_a_lock_pinned_tag_and_rejects_a_bad_nar_hash
  -- --nocapture`; and `devenv shell -- cargo test -p cix --test tour --
  --ignored generate_tour` (all passed). Next: build the standalone example,
  review scope, then run the prescribed workspace/tour/determinism/VM gate and
  record exact results plus cleanup here.

- 2026-07-31T21:05:00Z — Started `.dev/specs/track-itemrevive.md` on
  `track/itemrevive` at `4c822d9`. Read AGENTS.md, the session and Cixfile
  journals, authoritative D67/D68 in full, and the complete track spec.
  Scope: restore named ITEM as a D68 manifest-less pure store tree; fence
  stratum-1a directives with the seam diagnostic; reject run/exec/debug on
  those trees; prove producer → tag → FROM-consume → COPY via real Nix; add
  a small example, docs, and the tour. The branch-local devenv probe passed:
  `devenv shell -- true`. Next: map the current parser, codegen, runner, and
  test seams before applying the smallest compatible representation.

- 2026-07-31T20:00:00Z — Started `.dev/specs/track-famref.md` on
  `track/famref`. Read AGENTS.md, the current session and Cixfile journals,
  authoritative D65 in full, and the complete track spec. Scope: classify
  `FROM` inputs as known flakerefs or explicit-tag index refs; resolve and
  narHash-pin item refs as source-only artifact binders; add real-Nix coverage,
  a small example, reference/migration/tour documentation, then run the full
  Rust/tour/determinism/VM gate with exact commands and cleanup recorded here.
  Next: map the parser, lock, index resolution, and real-Nix test seams before
  designing the smallest compatible representation.

- 2026-07-31T20:20:00Z — D65(a3) implementation and focused proof are green.
  `FROM` now classifies only known flakeref spellings as flakerefs; every other
  token must be an explicit-tag `cix_common::Ref`, which becomes an artifact
  source binder. `LockFile.artifacts` is keyed by ref and pins `{storePath,
  narHash}`; resolution uses the index, verifies local/remote output hashes,
  and a reused lock verifies its exact pinned store path without re-resolving a
  moved tag. Artifact interpolation is path-only; `${binder.attr}` cites
  D65(c), and builder IMPORT cites deferred D65(d). Exact focused repros:
  `devenv shell -- cargo fmt --all`; `devenv shell -- cargo test -p
  cix-cixfile --lib`; `devenv shell -- cargo test -p cix-cixfile --test
  lock_nix cix_item_from_copies_a_lock_pinned_tag_and_rejects_a_bad_nar_hash --
  --nocapture`; and `devenv shell -- cargo test -p cix --test tour -- --ignored
  generate_tour`. The real-Nix test builds and tags a producer, builds a
  consumer that COPYs it, proves tag-move stability until `--update-lock
  source`, proves the update, missing-tag `pull it or tag it first` guidance,
  and a hard forged-narHash failure. Next: inspect the full diff, run the
  prescribed workspace/tour/VM gate, record exact results and cleanup, then
  commit.

- 2026-07-31T20:35:00Z — Final D65(a3) gate is green. Exact prescribed
  repros: `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo
  clippy --workspace --all-targets -- -D warnings`; `devenv shell -- cargo
  test --workspace`; `devenv shell -- cargo test -p cix --test tour --
  --ignored generate_tour`; `git add docs/tour && git diff --exit-code --
  docs/tour`; `devenv shell -- cargo test -p cix --test tour
  tour_matches_committed_document -- --exact`; and `devenv shell -- cargo test
  -p cix --test tour generated_tour_is_deterministic -- --exact` (passed
  twice). The dogfood VM repro `devenv shell -- nix build
  .#checks.x86_64-linux.vm-dogfood --no-link -L` passed under normal TCG
  fallback after KVM denial; receipt confirmed with `devenv shell -- nix
  path-info .#checks.x86_64-linux.vm-dogfood`:
  `/nix/store/0wak3k68m5kmggmqmww6lbxp51qplfvm-vm-test-run-vm-dogfood`.
  `git diff --check` and `git diff --cached --check` pass. Cleanup repro:
  `sudo -n systemctl stop 'cix-*' >/dev/null 2>&1 || true; sudo -n systemctl
  reset-failed 'cix-*' >/dev/null 2>&1 || true; sudo -n systemctl
  daemon-reload; systemctl --user stop 'cix-*' >/dev/null 2>&1 || true;
  systemctl --user reset-failed 'cix-*' >/dev/null 2>&1 || true; systemctl
  --user stop cix-run.slice >/dev/null 2>&1 || true; ! sudo -n systemctl
  list-units 'cix-*' --all --no-legend --plain | grep -q .; ! systemctl
  --user list-units 'cix-*' --all --no-legend --plain | grep -q .` passed.
  Removed only the untracked devenv-generated `devenv.lock`. Next: stage the
  complete D65(a3) diff and commit it on `track/famref`.

- 2026-07-31T20:40:00Z — Committed the green D65(a3) implementation on
  `track/famref` (`Implement D65 FROM item binders`). No open items remain for
  this track.

- 2026-07-31T19:25:00Z — Final D58 `/usr/bin/env` gate is green. Exact
  prescribed repros: `devenv shell -- cargo fmt --all --check`; `devenv shell
  -- cargo clippy --workspace --all-targets -- -D warnings`; `devenv shell --
  cargo test --workspace`; `devenv shell -- cargo test -p cix --test tour --
  --ignored generate_tour`; `git add docs/tour && git diff --exit-code --
  docs/tour`; `devenv shell -- cargo test -p cix --test tour
  tour_matches_committed_document -- --exact`; and `devenv shell -- cargo test
  -p cix --test tour generated_tour_is_deterministic -- --exact` (twice). The
  dogfood VM repro `devenv shell -- nix build
  .#checks.x86_64-linux.vm-dogfood --no-link -L` passed under normal TCG
  fallback after KVM denial; receipt confirmed with `devenv shell -- nix
  path-info .#checks.x86_64-linux.vm-dogfood`:
  `/nix/store/dpnkmcgs7vhshm9ypabfk6i8ka8f35s9-vm-test-run-vm-dogfood`.
  `git diff --check` and `git diff --cached --check` pass. Cleanup repro:
  `sudo -n systemctl stop 'cix-*' >/dev/null 2>&1 || true; sudo -n systemctl
  reset-failed 'cix-*' >/dev/null 2>&1 || true; sudo -n systemctl daemon-reload;
  systemctl --user stop 'cix-*' >/dev/null 2>&1 || true; systemctl --user
  reset-failed 'cix-*' >/dev/null 2>&1 || true; systemctl --user stop
  cix-run.slice >/dev/null 2>&1 || true; ! sudo -n systemctl list-units
  'cix-*' --all --no-legend --plain | grep -q .; ! systemctl --user
  list-units 'cix-*' --all --no-legend --plain | grep -q .` passed. Next:
  final scope audit and commit the green track.

- 2026-07-31T19:12:00Z — D58 `/usr/bin/env` focused implementation and corpus
  proof are green. Bubblewrap now creates only `/usr/bin/env → /bin/env` in
  addition to the existing union; `/usr` otherwise remains empty. The alias
  dangles without an imported `env`, and a failed command then names the alias
  and suggests `IMPORT ${pkgs.coreutils}`. Chain-key decision: the fixed
  skeleton is a versioned serialized step-key input (`v1:/usr/bin/env->/bin/env`),
  so this semantic change invalidates prior memo keys and retains D57's
  disposable-workspace guarantee; a repeated successful build memo-hits rather
  than flapping. Exact focused repros: `devenv shell -- cargo fmt --all`;
  `devenv shell -- cargo test -p cix-cixfile --test lock_nix
  usr_bin_env_shebang_requires_an_imported_env -- --nocapture`; `devenv shell
  -- cargo test -p cix-cixfile --lib`; `devenv shell -- cargo build -p cix`;
  `cd corpus/migrate && ./fetch.sh echo-server`; and `cd
  corpus/migrate/echo-server && ../../../target/debug/cix build --update-lock
  build .#echo-server && ./check.sh cix`. The regression proves both a literal
  shebang success with bash/coreutils imports and the loud missing-coreutils
  failure. Echo Server now reaches and passes webpack plus its HTTP check;
  receipt and corpus journal contain the full honest result. Next: review the
  track diff and run the prescribed full Rust/tour/VM gate.

- 2026-07-31T19:00:00Z — Started `.dev/specs/track-usrbinenv.md` on
  `track/usrbinenv`. Read AGENTS.md, the session journal, D58’s complete
  2026-07-31 `/usr/bin/env` addendum, the full track spec, and this crate
  journal. Scope: add exactly the `/usr/bin/env → /bin/env` builder-skeleton
  alias, preserve D57 memo correctness, cover present/absent IMPORT behaviour,
  update builder docs, and re-check echo-server honestly. Next: inspect the
  union/sandbox and real-Nix test seams, then implement the minimal skeleton.

- 2026-07-31T18:00:00Z — Started `.dev/specs/track-absdest.md` on
  `track/absdest`. Read AGENTS.md, the session journal, authoritative D66 in
  full, the complete track spec, and this crate journal. D66 makes SERVICE/APP
  destinations runtime-world absolute while retaining item-relative storage;
  BUILDER COPY stays workdir-relative. Next: fetch every corpus context,
  normalize parser storage after validating absolute spellings, migrate live
  Cixfiles/docs/tour, prove manifest equivalence, then run the prescribed
  Rust/tour/corpus/docs/VM gates and record exact repros here.

- 2026-07-31T18:40:00Z — D66 compiler/surface milestone is focused-green.
  SERVICE/APP COPY/FILE/LINK now require an item-world absolute spelling and
  normalize it to the pre-existing item-relative model; `/` normalizes to the
  existing COPY-root `.`. Relative EXEC/SETUP paths with a slash give the same
  migration-grade absolute spelling, while bare commands remain D64 commands.
  BUILDER COPY remains relative and now explicitly rejects an absolute
  destination. Codegen keeps stored files byte-identical and derives assembly
  mounts from their normalized runtime paths; a manifest test compares the
  D66 parse result with the pre-D66 v5 shape. Exact focused repros passed:
  `devenv shell -- cargo fmt --all`; `devenv shell -- cargo test -p
  cix-cixfile --lib`; and `devenv shell -- cargo test -p cix-cixfile --test
  lock_nix -- --nocapture` (43 unit and 16 real-Nix tests).
  `cd corpus/migrate && ./fetch.sh --all` fetched all pinned contexts, and
  the nine previously green checks (adminer, caddy, memcached, nats, nginx,
  phpmyadmin, redis, traefik, whoami) pass via `./check.sh cix`; exact corpus
  results are appended to `corpus/migrate/LOG.md`. Both complete migrate.md
  samples were copied verbatim to ignored `.dev/scratch/absdest/` fixtures and
  built with `devenv shell -- target/debug/cix build .dev/scratch/absdest/{dissolve,fetch}`;
  the sample and README/reference verbatim diffs pass. Tour regeneration via
  `devenv shell -- cargo test -p cix --test tour -- --ignored generate_tour`
  passed. Next: review generated docs, run the full prescribed Rust/tour/VM
  gate, cleanup units, then commit.

- 2026-07-31T18:55:00Z — Final D66 gate is green. Exact prescribed repros:
  `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo clippy
  --workspace --all-targets -- -D warnings`; `devenv shell -- cargo test
  --workspace`; `devenv shell -- cargo test -p cix --test tour -- --ignored
  generate_tour`; `git add docs/tour && git diff --exit-code -- docs/tour`;
  `devenv shell -- cargo test -p cix --test tour
  tour_matches_committed_document -- --exact`; and `devenv shell -- cargo
  test -p cix --test tour generated_tour_is_deterministic -- --exact` (passed
  twice). The dogfood gate `devenv shell -- nix build
  .#checks.x86_64-linux.vm-dogfood --no-link -L` passed under normal TCG
  fallback after KVM denial; receipt independently confirmed with `devenv
  shell -- nix path-info .#checks.x86_64-linux.vm-dogfood`:
  `/nix/store/xn34wazhykc9xvp1fjn9f73jymixha7l-vm-test-run-vm-dogfood`.
  `git diff --check` and `git diff --cached --check` pass. Unit cleanup repro:
  `sudo -n systemctl stop 'cix-*' >/dev/null 2>&1 || true; sudo -n systemctl
  reset-failed 'cix-*' >/dev/null 2>&1 || true; sudo -n systemctl daemon-reload;
  systemctl --user stop 'cix-*' >/dev/null 2>&1 || true; systemctl --user
  reset-failed 'cix-*' >/dev/null 2>&1 || true; systemctl --user stop
  cix-run.slice >/dev/null 2>&1 || true; ! sudo -n systemctl list-units
  'cix-*' --all --no-legend --plain | grep -q .; ! systemctl --user list-units
  'cix-*' --all --no-legend --plain | grep -q .` passed. Removed only the
  untracked devenv-generated `devenv.lock`. Next: stage all track files and
  commit this green D66 implementation.

- 2026-07-31T17:15:24Z — Committed the green D64 implementation on
  `track/selfbin` as `Implement D64 implicit self-bin runtime PATH`. No open
  items remain for this track.

- 2026-07-31T17:15:24Z — Final D64 gate is green. Exact prescribed repros:
  `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo clippy
  --workspace --all-targets -- -D warnings`; `devenv shell -- cargo test
  --workspace`; `devenv shell -- cargo test -p cix --test tour -- --ignored
  generate_tour`; `git diff --exit-code -- docs/tour`; `devenv shell -- cargo
  test -p cix --test tour tour_matches_committed_document -- --exact`; and
  `devenv shell -- cargo test -p cix --test tour generated_tour_is_deterministic
  -- --exact` (passed twice). The dogfood gate `devenv shell -- nix build
  .#checks.x86_64-linux.vm-dogfood --no-link -L` passed under normal TCG
  fallback after expected KVM denial; receipt confirmed with `devenv shell --
  nix path-info .#checks.x86_64-linux.vm-dogfood`:
  `/nix/store/q5yysxm4n1j2mb9pknhchzq9q9rnw9a6-vm-test-run-vm-dogfood`.
  `git diff --check` and `git diff --cached --check` pass. Unit cleanup repro:
  `systemctl --user reset-failed 'cix-*' || true; systemctl --user stop
  cix-run.slice >/dev/null 2>&1 || true; ! systemctl --user list-units
  'cix-*' --all --no-legend --plain | grep -q .`; it left no cix user units.
  Removed only the untracked, devenv-generated `devenv.lock`. Next: stage the
  remaining track files and commit this green D64 implementation.

- 2026-07-31T17:15:00Z — Compiler/runner milestone is focused-green. Bare
  EXEC/SETUP now writes `bin/<name>` to v5 manifests and validates executable
  presence only after the item is assembled; its failure names the directive,
  line, and contents of `bin/`. Codegen emits `PATH=bin` for every SERVICE/APP
  unless explicit `ENV PATH` exists, which is emitted unchanged. No manifest
  field was added or retyped, so v5 stays v5 (D15 not triggered). Runner tests
  prove both `cix run` units and debug units project that item-relative default
  to the absolute output `bin/`; exec inherits the same generated unit
  Environment through its existing systemd inspection. Exact green repros:
  `devenv shell -- cargo fmt --all`; `devenv shell -- cargo test -p cix-run
  --lib`; `devenv shell -- cargo test -p cix-cixfile --lib`; and `devenv shell
  -- cargo test -p cix-cixfile --test lock_nix -- --nocapture` (all 16
  real-Nix tests, including default, explicit-PATH replacement, self-bin-only
  resolution, and diagnostic listing). Next: regenerate/review the tour and
  execute the prescribed full gate.

- 2026-07-31T17:01:47Z — Started `.dev/specs/track-selfbin.md` on
  `track/selfbin` at `7b0d93c`. Read AGENTS.md, authoritative D64 in full, the
  complete track spec, session journal, and this crate journal. D64 replaces
  D31's explicit external runtime PATH resolution: SERVICE/APP will always
  generate `PATH=bin` unless an explicit `ENV PATH = …` replaces it; bare
  EXEC/SETUP resolves only against the assembled item's own `bin/`, while
  explicit `bin/x` and store paths stay literal. This changes generated values
  in the existing v5 `env`/`exec` fields only, so no manifest field or version
  change is expected. Next: update compiler and runner coverage, sweep active
  examples/docs/tour, then run the prescribed full gate and record exact
  commands here.

- 2026-07-31T16:55:00Z — Final demofix gate is green. Exact demo invocations
  (each passed) were `devenv shell -- env CIX_BIN=/home/mathijs/composix/.worktrees/demofix/target/debug/cix bash examples/pack/nginx/demo.sh`;
  `devenv shell -- env CIX_BIN=/home/mathijs/composix/.worktrees/demofix/target/debug/cix bash examples/pack/caddy/demo.sh`;
  `devenv shell -- env CIX_BIN=/home/mathijs/composix/.worktrees/demofix/target/debug/cix bash examples/pack/node-app/demo.sh`;
  `devenv shell -- env CIX_BIN=/home/mathijs/composix/.worktrees/demofix/target/debug/cix bash examples/pack/postgres/demo.sh`;
  `devenv shell -- env CIX_BIN=/home/mathijs/composix/.worktrees/demofix/target/debug/cix bash examples/build/projB/demo.sh`;
  `devenv shell -- env CIX_BIN=/home/mathijs/composix/.worktrees/demofix/target/debug/cix bash examples/build/projB-chef/demo.sh`;
  and `devenv shell -- env CIX_BIN=/home/mathijs/composix/.worktrees/demofix/target/debug/cix bash examples/compose/stack/demo.sh`.
  Exact prescribed gate repros: `devenv shell -- cargo fmt --all --check`;
  `devenv shell -- cargo clippy --workspace --all-targets -- -D warnings`;
  `devenv shell -- cargo test --workspace`; `devenv shell -- cargo test -p cix
  --test tour -- --ignored generate_tour`; `git diff --exit-code -- docs/tour`;
  `devenv shell -- cargo test -p cix --test tour
  tour_matches_committed_document -- --exact`; `devenv shell -- cargo test -p
  cix --test tour generated_tour_is_deterministic -- --exact` (twice); and
  `devenv shell -- nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`
  (passed under TCG after expected KVM denial; output
  `/nix/store/d79lrmlyfiqvwpkkz6qx538pxdi7vraa-vm-test-run-vm-dogfood`). Final
  cleanup reproduced the all-empty system and user `systemctl list-units
  'cix-*'` scans; `git diff --check` passes. `devenv.lock` was generated during
  the gate and removed, since it was absent on the clean starting tree. The
  scoped green track is committed on `track/demofix`.

- 2026-07-31T16:48:00Z — Every touched demo is e2e-green with the rebuilt
  binary, using these exact invocations (all ran from the repository root):
  `devenv shell -- env CIX_BIN=/home/mathijs/composix/.worktrees/demofix/target/debug/cix bash examples/pack/nginx/demo.sh`
  (served the nginx page through `my-nginx`); the identical prefix with
  `examples/pack/caddy/demo.sh` (served the Caddy page and asserted
  `cap_net_bind_service`); `examples/pack/node-app/demo.sh` (the no-JIT V8
  control failed as expected, then the `GRANT jit` service served its page);
  `examples/pack/postgres/demo.sh` (SQL `SELECT 1` passed);
  `examples/build/projB/demo.sh` (served `hello from RUN v0`);
  `examples/build/projB-chef/demo.sh` (served `hello from the chef chain`);
  and `examples/compose/stack/demo.sh` (fd-only web→backend→Redis, selective
  update, rollback, and `down` all passed). The nginx initial PrivatePIDs
  attempt and compose web unit used the expected D36 degraded fallback in this
  host environment; neither affected the assertions. Cleanup repro:
  `sudo systemctl stop 'cix-*' >/dev/null 2>&1 || true; sudo systemctl
  reset-failed 'cix-*' >/dev/null 2>&1 || true; sudo systemctl daemon-reload;
  ! sudo systemctl list-units 'cix-*' --all --no-legend --plain | grep -q .;
  systemctl --user stop 'cix-*' >/dev/null 2>&1 || true; systemctl --user
  reset-failed 'cix-*' >/dev/null 2>&1 || true; ! systemctl --user list-units
  'cix-*' --all --no-legend --plain | grep -q .` passed: both lists are clean.
  The D62 selector audit finds exactly ten selected builds across seven demos;
  `sed -n '28,38p' README.md | diff -u examples/pack/nginx/Cixfile -` passed,
  proving the sample directives and source are byte-identical. Next: full gate.

- 2026-07-31T16:20:00Z — Started `.dev/specs/track-demofix.md` on clean
  `track/demofix`. Read AGENTS.md, the session and crate journals, authoritative
  D62, and the complete spec. Scope: repair all ten stale `cix build <dir>`
  demo captures with D62 member selectors (including the three compose-stack
  builds), rename the pack nginx SERVICE to `my-nginx`, and make the README
  sample caption/source verbatim again. Structural gap recorded for follow-up:
  demos claim e2e verification, but no automated gate executes the demo scripts;
  a scenario VM tier is the candidate future home. Next: patch selectors and
  source/caption, then run every touched demo as root and the full prescribed
  Rust/tour/VM gate with exact receipts here.

- 2026-07-31T16:02:00Z — Final prompt-refresh gate is green. Exact smoke repro:
  `devenv shell -- cargo test --workspace`; it passed the complete workspace,
  including 41 Cixfile unit tests, 16 real-Nix Cixfile tests, the proj1 warm/cold
  proof, tour drift/determinism, runner system/user projection, compose, index,
  and doc tests. Final sample/source audit repros:
  `awk '/^## Complete sample: dissolve/{section=1; next} section &&
  /^```dockerfile$/{code=1; next} code && /^```$/{exit} code{print}'
  docs/migrate.md | diff -u -
  .dev/scratch/promptrefresh/dissolve/Cixfile` and the identical command with
  `fetch` in both places; both passed with no diff. Final scope repros:
  `git diff --check`; `test -z "$(git diff --name-only | rg -v
  '^(docs/migrate.md|crates/cix-cixfile/LOG.md)$')"`; both pass, and the only
  changed paths are `docs/migrate.md` plus this required journal. Cleanup repro:
  `systemctl --user reset-failed 'cix-*'; systemctl --user stop cix-run.slice;
  systemctl --user list-units --all 'cix-*' --no-legend`; no cix user units
  remain. Removed only the untracked `devenv.lock` generated by this track's
  `devenv shell` commands. No open items; next: commit the green two-file diff.

- 2026-07-31T15:48:00Z — The rewritten migration prompt and both complete
  examples are real-binary green. It now teaches migration as dissolve/build/gap,
  D47 binders and blocks, D56 EXPECT, D58 IMPORT, D59 builder ENV and quoted argv,
  D60 grants, the complete D52 role-dir family, D62's build/select/tag flow, the
  bare-builder-command invariant, and an explicit implemented-vs-designed gap
  ledger including D48 health and Docker-socket refusal. Exact repros:
  `git check-ignore -v .dev/scratch/promptrefresh` reports the local
  `.git/info/exclude` rule; `devenv shell -- cargo build` passed;
  `target/debug/cix build .dev/scratch/promptrefresh/dissolve` returned
  `{"web":"/nix/store/xzhzcfh1ynpgxq7vrvlwhvgbjaw2vl87-cix-item-web"}`;
  `target/debug/cix build .dev/scratch/promptrefresh/fetch` executed the declared
  5.2 KiB EXPECT fetch and offline heredoc, then returned
  `{"readme":"/nix/store/axg74d3c1d7hzcji5ahjffj4f7fwc9bw-cix-item-readme"}`.
  Two `awk ... | diff -u - .dev/scratch/promptrefresh/{dissolve,fetch}/Cixfile`
  comparisons prove the verified fixtures are byte-for-byte the complete samples
  in `docs/migrate.md`; all other Cixfile snippets are visibly marked Fragment.
  `git diff --check` passes, and `git diff --name-only` names only the prompt and
  this required journal. Next: pedagogical/diff review, workspace smoke, final
  scope audit, and commit.

- 2026-07-31T15:30:00Z — Started `.dev/specs/track-promptrefresh.md` on clean
  `track/promptrefresh` at `5ffc89e`. Read AGENTS.md, the current session and
  crate journals, authoritative D47–D62, and the live Cixfile reference. The
  branch-local direnv/devenv is active. Scope is the full `docs/migrate.md`
  teaching-prompt rewrite plus this journal; sample fixtures will live under the
  locally ignored `.dev/scratch/promptrefresh/`. Next: write the current-language
  prompt, build the real `cix`, verify every complete sample, then run the
  workspace smoke and scope gates.

- 2026-07-31T00:45:00Z — Final D62 gate is green. Exact repros: `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo clippy --workspace --all-targets -- -D warnings`; `devenv shell -- cargo test --workspace`; `devenv shell -- cargo test -p cix --test tour -- --ignored generate_tour`; `devenv shell -- cargo test -p cix --test tour tour_matches_committed_document -- --exact`; `devenv shell -- cargo test -p cix --test tour generated_tour_is_deterministic -- --exact` (twice); `devenv shell -- nix build .#checks.x86_64-linux.vm-dogfood --no-link -L` (passed under QEMU TCG after KVM was denied; receipt `/nix/store/bi8p1gkac44b5a5zdqs51s020mkbyxd8-vm-test-run-vm-dogfood`, independently confirmed with `nix path-info .#checks.x86_64-linux.vm-dogfood`). `git diff --check` passes. Cleanup repro: `systemctl --user reset-failed 'cix-*'; systemctl --user stop cix-run.slice; systemctl --user list-units --all 'cix-*' --no-legend`; no cix user units remained. The existing untracked `devenv.lock` remains intentionally untouched. Next: stage only track files and commit the green implementation.

- 2026-07-31T00:20:00Z — D62 compiler/index/tour milestone is focused-green. `cix build .` now writes exactly one JSON member map to stdout (builder command/log output is forwarded to stderr); `.#member` builds the member's backward FETCH/BUILDER slice and prints a bare path; `-t` is repeatable tag-only family sugar with `--namespace` required for multi-member Cixfiles. The real-Nix selector regression gives an unrelated BUILDER `RUN exit 42` and proves `.#api` succeeds without executing it. Shared refs now require explicit tags with the `:latest is not a thing here` diagnostic; the index HTTP name route accepts untagged URL names internally while run rejects docker-shaped untagged refs before Nix fallback. The tour shows `proj1 --namespace proj1 -t v1` and runs `proj1/proj1-api:v1` through the existing slashed-name table. Exact focused repros: `devenv shell -- cargo test -p cix-cixfile selected_member_executes_only_its_backward_builder_slice -- --nocapture`; `devenv shell -- cargo test -p cix-common -p cix-index --lib`; `devenv shell -- cargo test -p cix-index --test pull -- --nocapture`; `devenv shell -- cargo test -p cix-cixfile --test lock_nix -- --nocapture`; `devenv shell -- cargo test -p cix --test tour -- --ignored generate_tour`. Codegen naming audit: `rg -n 'family|NAMESPACE' crates/cix-cixfile/src/codegen.rs` has no matches, so family naming is absent from generated output. Next: run tour drift/determinism, review the final diff, then execute the prescribed workspace/VM gate and commit.

- 2026-07-31T00:00:00Z — Started D62 round-one family tags on `track/famtags` at `0c2bc63`. Read AGENTS.md, `.dev/LOG.md`, D62 including its NAMESPACE/YAGNI amendment, this crate journal, and `.dev/specs/track-famtags.md`; branch devenv is available. Scope: Cixfile build selector/JSON/tag-only CLI, shared explicit-tag ref parsing, tag call sites, active docs/examples/tour; do not change cix-index table schema. The pre-existing untracked `devenv.lock` is left untouched. Next: implement and focused-test selector/tag/ref semantics, then migrate surfaces and run the prescribed gate.

- 2026-07-31T13:48:09Z — Final D52 directory-spelling gate is green. Exact repros: `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo clippy --workspace --all-targets -- -D warnings`; `devenv shell -- cargo test --workspace`; `devenv shell -- cargo test -p cix --test tour -- --ignored generate_tour`; `git diff --exit-code -- docs/tour`; `devenv shell -- cargo test -p cix --test tour tour_matches_committed_document -- --exact`; `devenv shell -- cargo test -p cix --test tour generated_tour_is_deterministic -- --exact` (twice); `devenv shell -- nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`. The VM gate passed under its normal TCG fallback after the host denied KVM. Cleanup repro: `systemctl --user reset-failed 'cix-*'; systemctl --user stop cix-run.slice; systemctl --user list-units --all 'cix-*' --no-legend`; it left no `cix-*` user units. Final `git diff --check` and generated-tour drift checks are clean. The untracked `devenv.lock` is environment-generated and intentionally left outside this commit. Next: commit the D52 micro-round.

- 2026-07-31T13:43:28Z — D52 directory-spelling compiler/docs milestone is focused-green. `LOGSDIR` and `CONFIGDIR` now validate and populate the unchanged `dirs.logs`/`dirs.config` manifest roles; `LOGS` and `CONFIG` are hard, line-numbered D52 migrations that name their replacement. The golden Cixfile, parser coverage (including service-only fences), and active Cixfile/migration docs use the new spellings. No active example Cixfile, `docs/docker.md`, or generated tour page used either old spelling, so tour regeneration is intentionally zero-diff. Exact green repros: `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo test -p cix-cixfile --lib`; `devenv shell -- cargo test -p cix --test tour -- --ignored generate_tour`; `devenv shell -- cargo test -p cix --test tour tour_matches_committed_document -- --exact`; `devenv shell -- cargo test -p cix --test tour generated_tour_is_deterministic -- --exact` (twice); `git diff --exit-code -- docs/tour`; `git diff --check`. Active-surface residue scan leaves `LOGS`/`CONFIG` only in the migration parser arms and their assertions. Next: run the full prescribed workspace and VM gate, clean test-created user units, record exact results, and commit.

- 2026-07-31T13:41:21Z — Started `.dev/specs/track-dirnames.md`, authoritative D52 addendum, on `track/dirnames`. Read AGENTS.md, the session and crate logs, the complete track spec, and D52’s resolved 2026-07-31 directive-family decision. Scope: hard `LOGS`→`LOGSDIR` and `CONFIG`→`CONFIGDIR` directive spellings only; manifest role keys remain `dirs.logs`/`dirs.config`. The branch-local devenv is active (Rust 1.97). Next: audit parser/model/test and active docs/example/tour surfaces, then add migration-grade diagnostics and run the prescribed gate.

- 2026-07-30T22:53:37Z — D56 EXPECT compiler/executor milestone is focused-green. Both `FETCH <name> EXPECT <sri-hash> <cmd…>` and builder-local `FETCH EXPECT <sri-hash> <cmd…>` carry the declaration through the typed model, replace any stale TOFU lock pin with the declared hash, verify memo hits and executions against it, and report declared versus fetched hashes without an update-lock hint. Targeted or all-fetch `--update-lock` refuses EXPECT with “change the EXPECT value instead.” Exact green repro: `cargo fmt --all && cargo test -p cix-cixfile --lib && cargo test -p cix-cixfile --test lock_nix fetch_expect -- --nocapture` (34 unit tests plus real sandbox match/mismatch coverage passed). Next: commit D56 separately, then replace snapshot-keyed memo execution with D57 chain keys, consumed-path records, and persistent disposable workspaces.

- 2026-07-30T23:12:42Z — Resumed track/keys after the authoritative D58 rewrite. Re-read AGENTS.md, `.dev/specs/track-keys.md`, and design D56/D57/D58; `bfde201` is already present through the branch merge and remains the focused EXPECT commit. Audited the existing uncommitted D57 partial: it has pure step/chain key requests with COPY source hashes and FETCH pins, path-indexed memo records, upper-preserving fresh-input staging, `--cold` plus deprecated `--no-cache`, and exact COPY-line cold attribution, but still uses the removed PATH surface and has no IMPORT union. Baseline is green: `cargo fmt --all --check && cargo test -p cix-cixfile --lib` (33 tests). Next: finish and test D57 engine semantics, then implement D58 IMPORT root unions and migrate every non-corpus/non-scenario surface.

- 2026-07-30T23:29:28Z — D57/D58 engine milestone is focused-green. Builder final keys are pure predecessor chains over directive arguments, ordered IMPORT roots, offered closure, fixed environment, COPY source hashes, and FETCH pins; lock memos now record each statically consumed path as its own NAR hash/store object, with whole-tree reads represented by `"."`. Persistent per-Cixfile/builder workspaces refresh declared inputs with deletion propagation while preserving build writes; `--cold` compares each recorded path and attributes a mismatch to the exact COPY line; CACHE is a hard D57 migration error. IMPORT accepts only whole package/binder refs, merges `bin`/`etc`/`share` recursively with earlier declarations winning, mounts them read-only at the sandbox root, and fixes bare-command lookup at `/bin`; service PATH is now explicit `ENV PATH`. The real git-over-HTTPS test proves failure without cacert and success with explicit `${pkgs.cacert}`; because locked nixpkgs OpenSSL does not discover `ca-bundle.crt` by directory alone, the fixed sandbox environment points `SSL_CERT_FILE` at the conventional imported path without importing any CA implicitly. Exact green repros: `cargo test -p cix-cixfile --lib`; `cargo test -p cix-cixfile --test lock_nix imported_cacert_enables_bare_git_over_https -- --nocapture`; `cargo test -p cix-cixfile --test lock_nix newly_consumed_path_reruns_the_chain_and_extends_its_record -- --nocapture`; `cargo test -p cix-cixfile --test proj1 -- --nocapture`. Next: commit the engine/language/examples slice, regenerate and verify the rewritten RUN/proj1 tour, then run the complete track gate.

- 2026-07-30T23:43:59Z — Persistent-workspace prefix reuse and migrated build examples are focused-green. Each workspace now stores its step-key vector: source edits after an unchanged pinned FETCH reuse that command prefix in the warm upper, while any changed suffix containing FETCH deliberately replays from step zero in a clean workspace before replacing the disposable persistent tree. Fresh declaration staging overrides an earlier step's colliding output on first application, then preserves later build writes across subsequent builds; this is what lets the cargo-chef recipe overwrite its generated placeholder manifest once and keep Cargo's incremental state thereafter. Ingredient now demonstrates `IMPORT bash curl cacert` plus EXPECT; projB uses a tiny manifest-only fetch target before staging its real source; both projB locks and the chef lock use D57 path records only. Exact green repros: `cargo test -p cix-cixfile --lib`; `cargo test -p cix-cixfile --test lock_nix -- --nocapture`; `cargo test -p cix-cixfile --test proj1 -- --nocapture`; `cargo test -p cix-cixfile --test lock_nix changed_step_before_fetch_replays_command_prefix_in_clean_workspace -- --nocapture`. Next: commit this workspace/example follow-up, then commit the rewritten reference/migration prose and generated tour before starting the full gate.

- 2026-07-30T23:46:21Z — The D56–D58 reference, migration guide, and executable tour are focused-green. The reference documents EXPECT, ordered `bin`/`etc`/`share` IMPORT unions, pure predecessor chain keys, path-indexed lock records, whole-tree cost, fresh staging with persistent writes, disposable workspaces, `--cold`, CACHE removal, and explicit service `ENV PATH`. Tour chapters 4 and 5 visibly prove cold → warm → cold artifact equivalence, narrow proj1 binaries, selective worker rebuilding, and safe workspace deletion; generated workspace receipts are normalized without hiding whether persistence was active. Exact green repros: `cargo fmt --all --check && cargo test -p cix --test tour -- --ignored generate_tour --nocapture`; `cargo test -p cix --test tour -- --nocapture`; `git diff --check`. Next: commit docs/tour, then run the full track gate from committed code.

- 2026-07-30T23:58:05Z — Final D56–D58 gate is green on committed implementation `bfde201` + `4f0cadd` + `e84e31a` + `89594e9` + `ebf25e2`. Exact full repro: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`; `cargo test -p cix-cixfile --test proj1 -- --nocapture && cargo test -p cix --test tour -- --ignored generate_tour --nocapture && git diff --exit-code -- docs/tour && cargo test -p cix --test tour tour_matches_committed_document -- --exact && cargo test -p cix --test tour generated_tour_is_deterministic -- --exact && cargo test -p cix --test tour generated_tour_is_deterministic -- --exact`; `nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`; `nix build .#checks.x86_64-linux.compose-fallback-vm --no-link -L`; `nix build .#checks.x86_64-linux.scenario-lifecycle --no-link -L`. This covers both EXPECT forms and lock behavior, 38 compiler unit tests, 15 real-Nix engine tests including real git-over-HTTPS with and without imported cacert, proj1 memo/selective warm rebuild/byte-identical cold/workspace wipe, tour zero drift and determinism twice, dogfood, the exact systemd-261 fallback receipt, and lifecycle update/rollback/down. Final audits found no touched `corpus/**` or `nix/scenarios/**`, live example PATH/CACHE directives, or legacy snapshot lock fields. Cleanup repro: `systemctl --user reset-failed 'cix-*' && systemctl --user stop cix-run.slice`; the user manager then listed zero `cix-*` units. Next: final log-only commit; branch is ready for independent verification and merge.

- 2026-07-30T22:49:09Z — Started `.dev/specs/track-keys.md` on clean `track/keys` at `4cd96e7`. Read AGENTS.md, the repository/session log, the complete track spec, D39/D40/D47/D48(a)/D56–D58, and the prior Cixfile track history; the branch-local devenv is active. Scope fences: do not touch `corpus/**` or `nix/scenarios/**`. The implementation must preserve all five D57 invariants: pure chain keys, consumed-path store records, disposable persistent workspaces, hard CACHE removal, and exact per-COPY warm/cold attribution. Next: inventory the parser/model/lock/executor and existing real-Nix/proj1/tour seams, then land EXPECT separately.

- 2026-07-30T22:36:15Z — Final D55 gate is green on committed implementation `03301e2` + `e262883`. Exact single-command repro: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo test -p cix --test tour -- --ignored generate_tour && git diff --exit-code -- docs/tour && cargo test -p cix --test tour tour_matches_committed_document -- --exact && cargo test -p cix --test tour generated_tour_is_deterministic -- --exact && cargo test -p cix --test tour generated_tour_is_deterministic -- --exact && nix build .#checks.x86_64-linux.vm-dogfood --no-link`; every stage passed from the start. This covers the exact D55 diagnostic, the 32-test compiler suite, all nine real-Nix Cixfile tests, workspace integration, explicit tour regeneration with zero drift and determinism twice, and dogfood. Final active-tree scans found no `SCRIPT` directive in example Cixfiles or generated tour pages and no script model/codegen residue; historical logs/specs and D55/migration text remain intentionally. Cleanup repro: `systemctl --user reset-failed 'cix-*' && systemctl --user stop cix-run.slice; sudo -n systemctl reset-failed 'cix-*' && sudo -n systemctl stop cix-run.slice`; both managers then listed zero `cix-*` units. The pre-existing untracked `devenv.lock` remains untouched. Next: final log-only commit; branch is ready for independent verification.

- 2026-07-30T22:30:08Z — D55 examples/tour/docs milestone is green. The live compose backend and web now keep `backend.py`/`start` as checked-in files, COPY them, and use `${pkgs.bash}/bin/sh` with system-projected `/bin/start`; their full system lifecycle passed v1, selective v2 replacement, rollback, and clean down. Tour chapters 3 and 6 now create, list, cat, and COPY real script files; rootless chapter 3 invokes the same locked local-source file because degraded user mode deliberately cannot project `/bin`, while the copied artifact and manifest mounts remain visible. Regenerated only pages 03/06 and replaced stale directive claims in `docs/cixfile.md`, `docs/docker.md`, and the superseded Part 4 prose; D55 and the migration note remain intentional. Exact green repros: `cargo run -q -p cix -- build examples/compose/stack/backend && cargo run -q -p cix -- build examples/compose/stack/web`; `cargo build -p cix && CIX_BIN="$PWD/target/debug/cix" examples/compose/stack/demo.sh`; `cargo test -p cix --test tour -- --ignored generate_tour --nocapture`; `cargo test -p cix --test tour -- --nocapture`; `git diff --check`. Next: commit this surface milestone, run explicit tour regeneration/drift/determinism twice, then the requested full Rust and dogfood VM gate.

- 2026-07-30T22:21:10Z — D55 compiler milestone is focused-green. Removed `Assembly::Script`, all script-specific Nix bindings/shebang/executable installation, and the real-Nix executable-script assertion; `FILE` still emits interpolated mode-0644 content. `SCRIPT` is intercepted before heredoc parsing and returns the exact requested line/source-bearing D55 rewrite with no compatibility alias. Exact green repro: `cargo fmt --all && cargo test -p cix-cixfile --lib && cargo test -p cix-cixfile --test lock_nix real_nix_build_assembles_files_links_and_spec -- --nocapture` (32 unit tests and the focused real-Nix test passed). Residue scan `rg -n 'Assembly::Script|cixfile-script|install -m 0755|"FILE" \| "SCRIPT"' crates/cix-cixfile` is empty. Next: commit this language milestone, migrate live compose fixtures and tour chapters 3/6, then regenerate.

- 2026-07-30T22:19:56Z — Started `.dev/specs/track-noscript.md` for D55 on clean `track/noscript` at `c7d5e30`; the only pre-existing worktree residue is an untracked `devenv.lock`, which is preserved. Read AGENTS.md, the current repository/session entry, D55 and the superseded directive prose, the complete crate history, and the parser/model/codegen/test/tour/docs surfaces. The branch-local devenv is active (Cargo 1.97, Nix 2.34). Scope is a hard `SCRIPT` removal with the exact D55 migration error, live compose-example migration, tour chapters 3/6 using visible real scripts plus explicit nixpkgs sh, active-doc sweep, and the full requested Rust/tour/VM gate. Next: remove the model/parser/codegen path and focused-test the migration diagnostic.

- 2026-07-30T21:44:35Z — Final tourbook gate is green on committed implementation `63073b8` + `7a6be82`. Exact single-command repro: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo test -p cix --test tour -- --ignored generate_tour && git diff --exit-code -- docs/tour && cargo test -p cix --test tour tour_matches_committed_document -- --exact && cargo test -p cix --test tour generated_tour_is_deterministic -- --exact && cargo test -p cix --test tour generated_tour_is_deterministic -- --exact && nix build .#checks.x86_64-linux.vm-dogfood --no-link -L && nix build .#checks.x86_64-linux.compose-fallback-vm --no-link -L && nix build .#checks.x86_64-linux.scenario-lifecycle --no-link -L`; every stage passed from the start. This covers D50 ITEM removal, D51 directory COPY/continuations/RUN heredocs, D52 CACHEDIR/LINK order, D53 comments, all workspace and real-Nix tests, regenerated-tour zero drift and determinism twice, dogfood, old-systemd compose fallback, and update/rollback/down lifecycle behavior. Final scans found no `ArtifactKind::Item`/`ManifestKind::Item`, legacy ITEM declarations, pack/compose no-op BUILDER blocks, result links, or lingering system/user `cix-*` units. Next: final log-only commit; branch is ready for independent verification.

- 2026-07-30T21:08:00Z — Tourbook and directive-reference milestone is focused-green. Fourteen isolated pages are now six chapter stories: one tag flows through tag/inspect/move/untag; one publisher/consumer pair serves, pulls, and refreshes; one Cixfile-built TAG flows through manifest inspection, run, and debug; RUN shows its complete directory before a heredoc miss/hit; proj1 shows its tree and D51 directory-COPY/heredoc cache proof; advanced cats the listener fixture and builds both compose revisions from a visible Cixfile. The TAG review requirement exposed that cix-run only sent non-store inputs to `nix build`; it now resolves local/qualified cix refs through cix-index first, with flake-installable fallback preserved. Rewrote the active Cixfile reference for D50–D53, updated the Docker/index vocabulary, and recorded the corpus receipt/version-parity limit. Exact green repros: `cargo test -p cix --test tour -- --ignored generate_tour --nocapture`; `cargo fmt --all && cargo test -p cix-cixfile --lib && cargo test -p cix-run --lib && cargo test -p cix --test tour -- --nocapture`. The latter includes committed-page drift and two-render determinism. Next: commit the prose/tour milestone, run explicit regeneration + drift + determinism twice, then execute every Rust and Nix gate from the track spec before the final log commit.

- 2026-07-30T20:32:00Z — D50–D53 compiler/runtime milestone is focused-green. Removed Cixfile ITEM and v4 `kind: "item"` support; added physical directive continuations, builder-shell RUN heredocs, D52 CACHEDIR/LINK hard flips with migration diagnostics, and formal D53 full-line comment coverage (including literal shell comments in RUN bodies). Migrated all real Cixfiles: proj1 is one directory COPY plus a readable RUN heredoc, deliberate manifest-first examples explain their memo boundary, every COPY-only builder is gone from pack and compose, and corpus comments explain non-obvious network/offline splits. Exact repros: `cargo fmt --all && cargo test -p cix-cixfile -- --nocapture`; `cargo test -p cix-run --lib`; `cargo test -p cix --test artifact_kinds --no-run`; `cargo build -p cix && for example in examples/pack/caddy examples/pack/nginx examples/pack/node-app examples/pack/postgres examples/pack/redis examples/compose/stack/backend examples/compose/stack/web; do target/debug/cix build "$example"; done`. All passed; the seven pure-assembly builds produced valid store items without BUILDER blocks. Next: commit the language/example milestone, then rewrite the directive reference and regroup the executable tour into chapter stories.

- 2026-07-30T20:05:00Z — Started `.dev/specs/track-tourbook.md` on clean `track/tourbook` at `9b20114`, extended by Mathijs to implement D53 in the same parser round. Read AGENTS.md, the repository/session log, D50–D53, the complete track spec, and the prior Cixfile history; the branch-local devenv is active. Required surface: D50 ITEM removal, D51 continuations/RUN heredocs/directory COPY, D52 CACHEDIR/LINK hard flips, D53 full-line comments, chapter-quality tour regrouping, manifest vocabulary, and no-op BUILDER removal. Next: inventory parser/model/runtime/tests and current tour/example sources, then implement migration-grade grammar changes with focused tests.

- 2026-07-29T15:54:55Z — Started D39 RUN v0. Required context and spike groundwork are read; the branch is clean and the sandbox dependencies are available. The intended seam is an ordered COPY/FETCH/RUN model plus an external build-chain executor that supplies the final `${build}` snapshot to the existing Nix item generator. Next: inspect parser ordering, lock compatibility, and code generation before changing the model.

- 2026-07-29T16:10:04Z — Parser, lock, memo, sandbox executor, and `${build}` codegen now work together. The real sandbox integration test passes; a clean `projB` build produced `/nix/store/ig5b88h0sbg7x24m559k8r9x7s832dbl-cixfile-item`, and a second build hit both FETCH and RUN memo entries and returned the identical path. The chef chain also built successfully. Key implementation choice: old COPY-only Cixfiles keep their generated-item behavior through the same final snapshot assembly, while direct `generate_nix` test callers retain the pre-D39 code path. Next: selective chef edit and FETCH mismatch product tests, then docs/tour/full gates.

- 2026-07-29T16:13:00Z — Chef source-edit proof passed: recipe FETCH and cook were memo hits, the changed source COPY produced a new snapshot, and only the final RUN missed. Restored the checked-in source; the lock retains both final memo realizations by design. Exact paths/transcript are in `.dev/specs/track-runv0.LOG.md`.

- 2026-07-29T16:16:13Z — Focused cix-cixfile unit + real-Nix tests and warning-denied clippy pass. Added deterministic generated-tour coverage of a real RUN miss/hit, completed D39 docs, wrapped the Nix package with its bwrap/nix runtime tools, and passed the live projB build/run/curl/stop demo. Existing nginx/PostgreSQL Cixfiles still build with COPY's 0644 semantics through the chain. Next: full workspace/VM gate, cleanup, and commit.

- 2026-07-29T16:22:03Z — Full workspace fmt/build/clippy/test and the NixOS VM dogfood gate pass. The initial VM command omitted the untracked new module by Git-flake semantics; staging the complete tree fixed source visibility and the unchanged check passed. Memo reuse now also realizes a missing substitutable store path and verifies a present path’s NAR hash. Final chef rebuild was all hits; exact gate commands and transcripts are in the track LOG. Units and temporary artifacts are clean. Next: short post-hardening recheck and commit.

- 2026-07-29T16:27:00Z — Exact staged-source recheck passed: full workspace gate, explicit tour regeneration/determinism, and NixOS VM dogfood. Ready to commit.

- 2026-07-29T03:25:00Z — Started `.dev/specs/track-fromdecl.md` (D32 amendment). Replaced ambient `pkgs` with required `FROM <flakeref> AS <name>` bindings and namespaced interpolation. The parser rejects missing/AS-less/duplicate bindings, accepts `nixpkgs`, GitHub refs, and HTTPS tarballs, and names declared namespaces in unknown-namespace errors. Locks are now `inputs.<name> = {url, rev, narHash}`, migrate the previous single-input shape on read/write, and `--update-lock [name]` refreshes one or all inputs. Generated Nix imports each locked universe independently. Added focused parser/lock/real-Nix coverage, migrated all eight example Cixfiles and their locks, and rebuilt them successfully. Chose not to add a second node-app universe: a contrived unused pin would obscure the namespace mechanism, which the worked docs demonstrate plainly. Next: document the truthful FROM meaning, run full gates/demos/VM check, and commit each verified milestone.

- 2026-07-29T03:40:00Z — Committed the implementation as `e49c071`. `cargo fmt --all --check` and warning-denied workspace all-target clippy passed twice; focused cix-cixfile unit and real-Nix tests pass. nginx, PostgreSQL, and compose-stack sudo demos passed with the built binary; compose covered fd-only web→backend→db, selective update, rollback, and clean down. `nix build .#checks.x86_64-linux.vm-dogfood --no-link` passed. The required `cargo test --workspace` gate was run twice but cannot pass without changing excluded territory: `crates/cix/tests/tour.rs` writes a Cixfile beginning with `FILE`, and now correctly receives the required-FROM diagnostic; its other tests then poison their shared renderer lock. The track explicitly forbids changes to that test and `docs/tour/`, so neither was touched. Stopped/reset system `cix-*` units; no lock temp files or result links remain. Next: commit this verification record and hand off the out-of-scope gate conflict.

- 2026-07-29T02:10:00Z — Started `.dev/specs/track-nopkg.md` (D32). Read the D32 contract, current compiler/parser, real-Nix coverage, every example Cixfile, and the Cixfile/Docker docs. The worktree is clean. Next: remove the `PKG` declaration model and parse arbitrary `${pkgs.<attrpath>}` references directly, with focused diagnostic and real-Nix regression coverage.
- 2026-07-29T02:20:00Z — Removed `PKG` from the typed model and parser. Every `${pkgs.<attrpath>}` now resolves directly from the locked nixpkgs expression; arbitrary nested attrpaths are accepted, and generated Nix wraps each lookup in an error context with the originating Cixfile line. Bare `${name}` suggests `${pkgs.name}`; a `PKG` line explains the D32 rewrite. Focused parser coverage and real-Nix tests cover nested resolution, the bare-name and PKG diagnostics, an unknown-attribute eval error with line number, and unchanged package assembly/spec behavior. Focused lib and real-Nix integration tests pass. Next: migrate and rebuild every example Cixfile.
- 2026-07-29T02:35:00Z — Migrated every example Cixfile, including all three compose-stack items, by deleting `PKG` lines and changing every package reference to `${pkgs.<attrpath>}`. All eight Cixfiles build successfully with the current local `cix` binary. The nginx and PostgreSQL system demos passed; the compose-stack sudo demo passed its fd-only web→backend→db probe, selective update, rollback, and clean down. Updated the directive reference, worked nginx example, interpolation/closure explanation, and Docker `FROM` row for direct `pkgs.*` references. Next: run the full required gates twice, VM check, and cleanup audit.
- 2026-07-29T02:50:00Z — Final D32 gate: `cargo fmt --all --check`, warning-denied workspace all-target clippy, and `cargo test --workspace` each passed twice in devenv. The VM gate `nix build .#checks.x86_64-linux.vm-dogfood --no-link` passed. Stopped and reset the `cix-*` system and user units (including stale failed listener probes), then verified both managers list no Cix units; no temporary locks or result links remain. Next: commit this final verification entry and confirm a clean worktree.

- 2026-07-28T21:50:00Z — Started `.dev/specs/track-cixfile.md`; read both authoritative design sections, the directive reference, cix-spec v2 validation, existing CLI/index APIs, and both hand-written examples. Added the `cix-cixfile` workspace crate scaffold. The branch-local `.envrc` changed and was re-approved; Rust 1.97, Nix 2.34, and devenv 2.1.2 are active. Contract discrepancy to preserve and test: Part 4/D22 says heredoc contents are verbatim, while this track explicitly requires `${…}` interpolation and `$${…}` escaping in FILE/SCRIPT bodies. The track requirement wins for the implementation; examples will still use D22-style `/app` paths and LINKs. Next: implement the parser and its complete error coverage.
- 2026-07-28T21:55:50Z — Implemented the complete v1 parser and typed model: all item/service directives, ordered service blocks, package and runtime interpolation validation, heredoc escaping, D11 role-path constraints, safe item paths, duplicate detection, and deferred ENV/PORT reference validation. Every parse/semantic error retains the exact line number and quoted offending source line, including heredoc-body failures. Focused tests exercise every directive, both port forms, all specified interpolation behavior, unsafe/duplicate paths, and representative syntax/semantic failures; tests and warning-denied clippy pass. Boring syntax choice: directive arguments are whitespace-delimited and ENV defaults are one token because v1 documents no quoting grammar. Next: generate deterministic Nix and cix-spec v2 JSON.
- 2026-07-28T21:58:32Z — Added deterministic code generation. The generated Nix uses a rev+narHash fixed `builtins.fetchTree`, content-addresses COPY inputs, writes FILE/SCRIPT artifacts (SCRIPT prepends nixpkgs `runtimeShell` and installs executable), creates LINK symlinks, and emits sorted cix-spec v2 JSON with de-typed env declarations. Destination parents are collected in sorted order and no package tree is copied. Added a checked-in golden Cixfile→spec JSON test plus deterministic/fixed-fetch and COPY validation tests; focused tests and clippy pass. Next: implement lock create/reuse/update and mismatch behavior, then exercise the Nix expression for real.
- 2026-07-28T22:00:37Z — Implemented `Cixfile.lock` creation from `github:NixOS/nixpkgs/nixos-unstable`, strict JSON reuse, atomic `--update-lock` refresh, and field validation. Added create/reuse/update and malformed-hash lifecycle tests. Wired `cix build [dir] [-t ref] [--update-lock]` through the existing flattened CLI; successful builds print the store path and optional tags use `cix-index::tag`. Focused cix/cix-cixfile tests, CLI help, and warning-denied clippy pass. Next: add D22-style nginx/postgres Cixfiles, resolve and commit their locks, then use real builds to cover fixed-output hash mismatch behavior.
- 2026-07-28T22:04:52Z — Added nginx and PostgreSQL Cixfiles plus committed locks. Determinate Nix 2.34 omits `locked.narHash` from `flake metadata`, so first/update resolution now archives the exact resolved revision and reads its store NAR hash; both real builds then succeeded and lock reuse produced the same outputs. Added a real-Nix tampered-lock test that fails on the fixed-output hash mismatch. Nginx passed system-mode curl with the expected page; PostgreSQL initially revealed that invoking a symlinked binary changes its inferred installation prefix, so the item now LINKs `${postgresql}/share/postgresql` as well; it then passed TCP `SELECT 1`. Both transient units were stopped. Semantic diff: PostgreSQL's generated spec is byte-for-byte equal after sorted JSON normalization; nginx differs only at `services.nginx.exec[2]` (`/app/etc/nginx.conf` versus the hand-written store-path config), exactly the D22 value-form allowance. Worked-nginx deviations needing a future doc correction: this repository has no sibling `index.html`, so the Cixfile uses inline FILE; the executable is LINKed and invoked as `bin/nginx` to honor this track's LINK-for-cross-package requirement; explicit cache temp paths mirror the existing `.nix` runtime behavior. Next: tighten integration coverage and run the full workspace/final system gate.
- 2026-07-28T22:06:14Z — Verified the real `--update-lock` route (same current pin/output) and `-t` index integration (tagged, listed, then removed the test tag). Expanded real-Nix test coverage to build an item from the committed lock and inspect actual FILE package interpolation, `$${…}` literal escaping, untouched `$VAR`, SCRIPT shebang/executable mode, LINK target, and runner-validated cix-spec v2. The tampered-narHash failure test remains green. Next: run the complete workspace fmt/clippy/test gate, repeat both system demos from the final binary, and audit status/history/units.
- 2026-07-28T22:07:31Z — Final summary: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` pass. The first workspace test run correctly detected a leftover system `cix-run.slice` in the tour transcript; stopped it, reran, and the full gate passed. Repeated both final-binary system-mode demos: nginx served `<h1>hello from composix</h1>` and PostgreSQL returned `1` for `SELECT 1`; stopped both units and the system slice. Rechecked that the temporary tag is absent, no `*.lock.tmp`/result links exist, and the six implementation commits cover only the declared territory. Deviations/doc corrections remain exactly those recorded above: the track-mandated heredoc interpolation conflicts with Part 4/D22's verbatim statement, and the worked nginx example needs its absent `index.html`, LINKed executable, and cache temp paths reconciled. No leftover units or open implementation work.
- 2026-07-28T22:10:00Z — Replaced nginx's two verbatim FILE heredocs with sibling `COPY` inputs and aligned the worked documentation. PostgreSQL retains its runtime-env FILE and setup/start SCRIPT heredocs: each needs build-time package interpolation (`${coreutils}`, `${nss_wrapper}`, or `${postgresql}`), so converting them to verbatim COPY files would lose required paths rather than simplify the example. COPY already validates a local regular file and inserts it unchanged at build time; no implementation change was needed. Next: build both Cixfiles, run the system demos, then complete the workspace gate and cleanup.
- 2026-07-28T22:11:00Z — The first real COPY build found a pure-evaluation gap: generated `builtins.path` intentionally references the checked local sibling, which Nix rejects without `--impure`. Added that flag only to the Cixfile build route and its real-Nix helper, plus coverage that a COPY payload containing `${…}` and `$VAR` arrives byte-for-byte unchanged. Next: rerun focused real-Nix coverage and both example builds.
- 2026-07-28T22:12:00Z — Focused real-Nix COPY coverage, `cargo fmt --all --check`, workspace all-target warning-denied clippy, and `cargo test --workspace` all pass. Rebuilt both Cixfiles with the local CLI; the nginx and PostgreSQL sudo demos also passed, serving the expected page and returning `1`, respectively. Stopped both transient services and the system cix slice; next: verify both managers and the worktree are clean, then commit this final verification entry.
- 2026-07-28T22:13:00Z — Cleanup audit found the workspace user-run test had recreated an otherwise empty user `cix-run.slice`; stopped it and its parent. Both system and user managers now report no active `cix-*` units, and the worktree is clean apart from this append-only verification entry.
- 2026-07-29T00:00:00Z — Started `.dev/specs/track-pathdecl.md` (D31). Read the D31 contract and current Cixfile implementation/examples. Next: extend the typed parser/model and deterministic spec generation for ordered item-level PATH declarations, then add compile-time executable resolution in generated Nix.
- 2026-07-29T00:15:00Z — Implemented D31’s compiler core. PATH accepts ordered absolute package templates, rejects duplicates and explicit ENV PATH, and bare EXEC/SETUP commands now require PATH. Generated Nix resolves each bare command at evaluation time with a line-numbered searched-directory error, chooses the first matching directory, verifies executable mode in the item build, writes resolved store paths to the spec, and emits PATH as an ordinary default env declaration. Added parser, golden-spec, real-Nix found/first-wins/not-found coverage; focused tests and clippy pass. Boring validation choice: PATH paths must begin with `/` or a package interpolation, so their evaluated values are necessarily absolute without trying to impose filesystem-projection rules on store paths. Next: flip the requested examples and update docs, then run real demos and final gates.
- 2026-07-29T00:35:00Z — Converted the D31 examples and directive docs. node-app and Redis use PATH; nginx and caddy use their direct one-off store executables; listenfds has no Cixfile, so its Nix-only listener example needs no PATH change. PostgreSQL now projects scripts and the `nss_wrapper` library at `/opt/postgres`, invokes copied scripts through the direct Bash store path, and calls `id`, `rm`, `mkdir`, `initdb`, `mv`, and `postgres` through PATH. Both Cixfile and default.nix variants were aligned around that shape. Rebuilt all five Cixfiles, rebuilt nginx/postgres default.nix variants, and ran their sudo demos; all passed. The generated PostgreSQL item has no `share/postgresql` LINK and the real `${postgresql}/bin/postgres` starts successfully, confirming that the old prefix-inference workaround is unnecessary. Stopped/reset every cix system and user unit afterwards. Next: audit, repeat the full required gates, and commit the example/docs milestone.
- 2026-07-29T00:55:00Z — Final verification: `cargo fmt --all --check`, warning-denied workspace all-target clippy, and `cargo test --workspace` all passed; the workspace test suite was run twice. Repeated every touched sudo demo (node-app, Redis, caddy, nginx, PostgreSQL) and rebuilt the Cixfiles/default.nix variants as required. The VM gate `nix build .#checks.x86_64-linux.vm-dogfood --no-link` passed. The VM probe revealed that its Nix-only PostgreSQL escape-hatch test still consumes `$out/bin/psql`; restored that helper link only in `default.nix`, while the Cixfile has no executable links. Next: stop test-created units, final status audit, and commit this verification entry.

- 2026-07-30T15:12:35Z — Started `.dev/specs/track-items.md` for D40/D41 from clean `track/items` at `fe5972d`. Read AGENTS.md, the repository/session log, D31/D32/D39–D41, compose-tree §1, and the current compiler/runner/compose seams. The active devenv is allowed. Scope boundary: no changes under `crates/cix-index`; multi-item tagging will call its current `tag` API once per built item. Next: reshape the parser/model around ITEM-owned assembly/service bodies, add TAKE/CACHE/OUTBOUND, then emit one v4 item per block.

- 2026-07-30T15:34:27Z — Compiler/runner milestone is green. `ITEM` is a hard rename; item-owned TAKE/FILE/SCRIPT/LINK plus OUTBOUND emit one bare v4 manifest and content-addressed store item each. Prelude CACHE mounts persistently into RUN only, is absent from memo keys/snapshots/items, and `--no-cache` bypasses RUN memo/cache mounts. Multi-item `-t v1` resolves to `<item>:v1`; a full ref is rejected for multi-item builds without touching `cix-index`. Runner accepts 1–4 and normalizes v4 to one virtual service; legacy multi-service selection errors cite D41. Compose's selector is removed from its Rust model and schema. The real proj1 test proves exact binary-only listings (plus manifest), v4/outbound shape, unchanged-build memo hits, worker-only output invalidation, and byte-identical `--no-cache` outputs. Exact green repro: `cargo test -p cix-run -p cix-cixfile -p cix-compose`. A manual tag smoke also passed with `cargo build -p cix && CIX_STATE_DIR=/tmp/cix-items-state target/debug/cix build examples/build/proj1 -t v1`; `target/debug/cix ls -l` showed `proj1-api:v1` and `proj1-worker:v1`. Next: finish docs/tour and all example migrations, then run the full track gate.

- 2026-07-30T15:42:05Z — Completed D40's item-scoped PATH edge: prelude PATH remains absolute/package-based, while each ITEM may add normalized relative directories resolved against its own store root. Generated bare EXEC/SETUP paths and runner/debug environments now preserve that meaning; v4 store-path loading also gives run units the actual item name. Exact green repro commands: `cargo fmt --all && cargo test -p cix-cixfile --lib && cargo test -p cix-run --lib` (29 cix-cixfile and 42 cix-run tests passed). Next: finish active docs and render/review the tour.

- 2026-07-30T15:46:48Z — Active docs, Docker ledger, README, examples index, and executable tour now tell the ITEM/TAKE/CACHE/v4 story. The new tour scenario performs the real proj1 build, runs the API item through its inferred v4 item identity, curls the expected response, and stops it. The first drift run exposed a raw proj1 unit nonce; generalizing the existing unit-name normalizer fixed it. Exact green repro commands: `cargo test -p cix-cixfile --test proj1` (one real-Nix selective/cache/clean-rebuild test passed); `cargo fmt --all && cargo test --test tour -- --ignored generate_tour && cargo test --test tour` (generation plus all four non-ignored drift/determinism scenarios passed). Next: commit this docs/runtime completion milestone, then run the complete workspace/clippy/fmt/VM gate from the track.

- 2026-07-30T15:47:37Z — Added a real-Nix regression for item-relative `PATH bin`: it assembles an executable SCRIPT, resolves bare `EXEC start` to `bin/start`, and records the relative runtime PATH default. Exact green repro: `cargo fmt --all && cargo test -p cix-cixfile --test lock_nix real_nix_build_assembles_files_scripts_links_and_spec`. Next: commit the completed D40/D41 surface and start the full gate.

- 2026-07-30T15:54:16Z — Final track gate is green on commits `e25fa5f` + `58a0f51`. Exact full repro: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && nix build .#checks.x86_64-linux.vm-dogfood --no-link`. This includes the real proj1 two-item listing/manifest/selectivity/warm-cache/clean-`--no-cache` test and the deterministic tour's proj1 API build/run/curl/stop scenario. The first combined run hit a transient user-systemd journal race after repeated tour rendering; `systemctl --user reset-failed 'cix-*' && systemctl --user stop cix-run.slice && cargo test -p cix-run --test user_run -- --nocapture` passed the expected namespace-degraded path, then the entire exact full command passed from the start. Reset the test-created user-manager failures and stopped its empty slice afterward. Scope audit: `git diff --name-only fe5972d..HEAD | rg '^crates/cix-index/'` returns no paths. Next: record the orchestrator close and make the final log-only commit.

- 2026-07-30T18:09:20Z — Started `.dev/specs/track-blocks.md` for D47 on clean `track/blocks` at `19b56e6`. Read AGENTS.md, the repository/session log, D31/D32/D39–D41/D47, the complete track spec, and the prior Cixfile track history; the branch-local devenv is active. Scope fences: do not touch `nix/scenarios/**`, `crates/cix-index`, or compose semantics. Next: inventory the parser/model/build-chain/codegen and runner seams, then implement the hard block/binder grammar with migration-grade line-numbered diagnostics first.

- 2026-07-30T18:42:38Z — Implemented the D47 compiler/runner core and reviewed the interrupted model-only patch on its merits. The parser now has shared, line-tracked, backward-only FROM/FETCH/block names; BUILDER-local PATH/CACHE/chains; SERVICE/APP/ITEM directive fences; caged RUN; unified binder or bare-context COPY; source-tree versus package-universe interpolation; and explicit TAKE/`${build}` migration diagnostics. Execution now produces one snapshot per named FETCH/BUILDER, keys persistent caches by builder, gives top-level FETCH an empty networked workdir with command-only memo identity and name-keyed TOFU pin, and preserves COPY trees/modes. v4 manifests accept absent/service/app/item kind; item has no exec; apps use a hardened transient oneshot and propagate the systemd-run status. Migrated every example Cixfile and built the new pinned 5.2 KiB `examples/build/ingredient` twice (miss then hit). A focused real-Nix test caught and fixed false RUN misses from counting the local source binder in the offered command closure. Exact green repros: `cargo check -p cix-cixfile`; `cargo test -p cix-cixfile --lib`; `cargo test -p cix-run --lib` except one newly-added assertion was corrected immediately to render the anyhow chain; `cargo test -p cix-cixfile --test lock_nix run_executes_outside_nix_and_build_interpolation_reaches_the_snapshot -- --nocapture`; `cargo test -p cix-cixfile -- --nocapture` (22 unit, 8 real-Nix, and the complete proj1 selective/warm/clean rebuild passed); `cargo run -p cix -- build examples/build/ingredient` followed by `cargo run -q -p cix -- build examples/build/ingredient` (memo hit). Next: finish runner kind coverage, migrate docs/tour and all stale textual/golden fixtures, then make the compiler/runtime milestone commit.

- 2026-07-30T18:49:00Z — Rewrote the active Cixfile reference around D47 blocks/binders, migrated the README worked nginx example, Docker feature ledger (including the Bazel positioning), corpus, examples index, and executable tour sources. The docs make the amended local-context rule explicit: bare relative COPY stays legal, `FROM . AS src` is optional naming sugar, and only remote source trees require FROM. `git diff --check` passes. Next: regenerate and review every tour diff, then add live APP exit/item-refusal acceptance coverage.

- 2026-07-30T18:55:00Z — Regenerated and reviewed tour pages 08/12/14 and the index; expected diffs are named BUILDER step logs, explicit source-binder paths, and SERVICE terminology. Added a CLI-level runner acceptance test that builds real v4 app/item store fixtures: the app streams output and propagates exit 23 exactly through `systemd-run --user --wait`, while `cix run` refuses the item before any manager/root checks with the D47 asset-only diagnostic. Exact green repros: `cargo fmt --all && cargo test --test tour -- --ignored generate_tour`; `cargo test -p cix --test artifact_kinds -- --nocapture`; `cargo test -p cix --test tour -- --nocapture` (4 passed, generator ignored; includes drift and two-render determinism). Next: commit the docs/runtime acceptance milestone, then audit the full diff and start the full workspace/VM gate.

- 2026-07-30T19:02:00Z — Merged current `main` at `dc4e331` before the final gate; its corpus honesty caveat merged cleanly with D47 wording and `nix/scenarios/**` remains untouched. The first exact Rust gate reached 26 cix-cixfile unit tests, then exposed an overstrong real-Nix assertion: bare and explicitly bound local COPY outputs had identical NAR hashes/bytes but different input-addressed derivation output paths. Changed the regression to compare authoritative NAR hashes rather than derivation names. Exact green repro: `cargo fmt --all && cargo test -p cix-cixfile --test lock_nix bare_and_explicit_local_copy_contexts_are_byte_identical -- --nocapture`. Next: commit this test correction and restart the exact full gate from the beginning.

- 2026-07-30T19:03:00Z — Final merged-tree gate is green. Exact repros: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`; `cargo test -p cix --test tour -- --ignored generate_tour && cargo test -p cix --test tour`; `nix build .#checks.x86_64-linux.vm-dogfood --no-link`; `nix build .#checks.x86_64-linux.compose-fallback-vm --no-link`. The workspace run includes 26 compiler unit tests, nine real-Nix tests, proj1 selectivity/warm-cache/byte-identical `--no-cache`, 51 runner unit tests, APP exit-23 propagation, ITEM refusal, and tour run/curl. `git diff --name-only main..HEAD | rg '^nix/scenarios/'` returns no paths; `git diff --check main..HEAD` passes. Reset test-created failed user units and stopped the empty user/system `cix-run.slice`; both managers now list no `cix-*` units. Next: commit this verification record; the branch is ready for independent verification and merge.

- 2026-07-30T19:15:00Z — Main advanced twice during the VM receipts, so merged through `6e9a136` (systemd-bisect harness/corpus work and D48 design follow-ups) and repeated the complete gate on that final snapshot. Exact single-command repro: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo test -p cix --test tour -- --ignored generate_tour && cargo test -p cix --test tour && nix build .#checks.x86_64-linux.vm-dogfood --no-link && nix build .#checks.x86_64-linux.compose-fallback-vm --no-link`; all stages passed. Tour regeneration left no diff. Final cleanup again reset test-created failed units/stopped empty slices; both managers list no `cix-*` units. `git diff --name-only main..HEAD | rg '^nix/scenarios/'` is empty and `git diff --check main..HEAD` passes. Branch ready for independent verification and merge.

- 2026-07-30T19:24:58Z — Started `track/polish` at `394d04a` for the ordered D48(b) egress hard rename followed by `.dev/specs/track-tour-proj1.md`. Read AGENTS.md, `.dev/LOG.md`, all of `docs/design.md`, this crate log, and the complete tour spec; branch-local devenv is allowed and Rust 1.97 is active. Rename scope includes compiler model/parser/codegen, runner manifest parsing, examples, active docs, generated tour, and a line-numbered migration diagnostic for `OUTBOUND` with no alias. Next: implement and focused-test the rename, regenerate the tour, then commit Part 1 before changing the proj1 scenario.

- 2026-07-30T19:28:38Z — D48(b) Part 1 is implemented end to end: Cixfile model/parser/codegen use `EGRESS`, v4 manifests and runner use `egress`, proj1 and all active docs use the final spelling, and regenerated tour page 13 shows it. `OUTBOUND` is not an alias: it gets a line-numbered D48(b) replacement diagnostic; old `outbound` fields in both bare v4 and legacy-shaped manifests get an explicit migration error before serde parsing. Exact green repros: `cargo fmt --all && cargo test -p cix-cixfile --lib && cargo test -p cix-run --lib && cargo test -p cix-cixfile --test proj1 -- --nocapture`; `cargo test -p cix --test tour -- --ignored generate_tour && cargo test -p cix --test tour`; `git diff --check`. The only active-tree old-spelling hits outside this append-only log are the migration code/tests and D48(b)'s rationale. Next: commit Part 1, then begin the tour-proj1 spec without mixing it into the rename commit.

- 2026-07-30T19:34:03Z — `.dev/specs/track-tour-proj1.md` is implemented. Page 14 now cats the real D47 Cixfile, then executes and asserts the deterministic sequence RUN miss + `cache-state: cold`; worker-only edit + RUN miss + `cache-state: warm` + unchanged API item; clean `--no-cache` miss + `cache-state: cold` + byte-identical API/worker item paths, before running and curling the API. The proj1 e2e test independently reads each memo snapshot's marker and proves cold → cold/warm → cold/cold while retaining its item identity assertions. Tour normalization removes RUN wall times and Cargo build progress entirely, so the page contains no timing evidence and cannot drift on compile ordering. Exact green repros: `cargo fmt --all && cargo test -p cix-cixfile --test proj1 -- --nocapture`; `cargo test -p cix --test tour -- --ignored generate_tour`; `cargo test -p cix --test tour tour_matches_committed_document -- --exact`; `cargo test -p cix --test tour generated_tour_is_deterministic -- --exact` run twice; `rg -n '(\\b[0-9]+(?:\\.[0-9]+)?(?:ms|s)\\b|… ms|Finished \\x60release\\x60|Compiling )' docs/tour/14-running-proj1.md` returned no matches; `git diff --check`. Next: commit Part 2, then run the complete final gate from the resulting committed tree.

- 2026-07-30T19:41:29Z — Final gate passed on committed implementation `603766d` (with D48(b) commit `55fdde9`). Exact single-command repro: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo test -p cix --test tour -- --ignored generate_tour && git diff --exit-code -- docs/tour && cargo test -p cix --test tour tour_matches_committed_document -- --exact && cargo test -p cix --test tour generated_tour_is_deterministic -- --exact && cargo test -p cix --test tour generated_tour_is_deterministic -- --exact && nix build .#checks.x86_64-linux.vm-dogfood --no-link && nix build .#checks.x86_64-linux.compose-fallback-vm --no-link`. This covers fmt, warning-denied clippy, all workspace tests, explicit tour regeneration with zero drift, drift test, determinism twice, the strengthened cold/warm/clean proj1 proof, dogfood VM, and systemd-261 compose fallback VM. Final residue scan leaves the old spelling only in migration code/tests, this append-only history, and D48(b)'s rationale; page 14 has no timing output. Cleanup repro: `systemctl --user reset-failed 'cix-*' && systemctl --user stop cix-run.slice && sudo -n systemctl reset-failed 'cix-*' && sudo -n systemctl stop cix-run.slice`; both managers then listed zero `cix-*` units. Next: final log-only commit and branch audit.

- 2026-07-31T13:10:00Z — Started `.dev/specs/track-argvenv.md`, authoritative D59/D60, on `track/argvenv`. Read AGENTS.md, the session and crate logs, complete track spec, and D59/D60; branch-local direnv/devenv is allowed. Scope: builder-local plain ENV with shell-time expansion and chain-key participation; quote-aware EXEC/SETUP argv; hard STATE→STATEDIR, JIT/EGRESS→GRANT migrations; v5 grants manifest plus runner validation/unit effects; active examples/docs/tour sweep. Open question for Mathijs (recorded, not decided): whether the remaining LOGS/CONFIG spellings should become LOGSDIR/CONFIGDIR. Next: inventory manifest/version validation and builder execution seams, then implement parser/model/codegen/runner tests before surface migration.

- 2026-07-31T13:25:00Z — Compiler/runner milestone implemented. Builder ENV is an ordered build step whose literal declaration enters predecessor keys and whose export prelude is parsed by every subsequent builder shell; the real-Nix RUN regression uses `ENV OUTPUT = $PWD/output` and proves it materializes `/work/output`. EXEC/SETUP now have quote-aware argv words with line-numbered unterminated-quote diagnostics. STATEDIR and GRANT (`jit`, `egress`) are hard flips, including closed-vocabulary and duplicate diagnostics. Cixfile output is bare manifest v5 with `grants`; runner reads v1–v5, keeps v1–v4 capability compatibility, enforces v5 replacement/version rules, and compiles `GRANT jit` by omitting `MemoryDenyWriteExecute=`. Exact focused repros so far: `devenv shell -- cargo fmt --all && devenv shell -- cargo test -p cix-cixfile --test lock_nix run_executes_outside_nix_and_build_interpolation_reaches_the_snapshot -- --nocapture && devenv shell -- cargo test -p cix-cixfile --test proj1 -- --nocapture`; first two passed, while the final `devenv shell -- cargo test -p cix-run --lib` exposed only a test assertion type mismatch and is being rerun after correction. Next: complete runner test, sweep generated tour/docs, then full prescribed gate.

- 2026-07-31T13:45:00Z — Focused green after correction: `devenv shell -- cargo test -p cix-run --lib` (54 tests) and `devenv shell -- cargo test -p cix-cixfile --test lock_nix -- --nocapture` (15 real-Nix tests) passed. The latter includes builder ENV shell-time `$PWD` expansion, argv/manifest v5 loading, and the existing EXPECT/import/workspace receipts. Regenerated the tour with `devenv shell -- cargo test -p cix --test tour -- --ignored generate_tour`; intended pages 03/05/06 updated (v5 manifest, GRANT egress, changed manifest hash). `devenv shell -- cargo test -p cix --test tour tour_matches_committed_document -- --exact` and one determinism run passed; a second determinism run and the full workspace/tour/VM gate remain. Updated stale OUTBOUND migration diagnostics to point at the current D60 target, `GRANT egress`. Next: format, rerun focused parser/runner checks, then every prescribed gate.

- 2026-07-31T14:05:00Z — D59/D60 track gate is green. Exact repros: `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo clippy --workspace --all-targets -- -D warnings`; `devenv shell -- cargo test --workspace`; `devenv shell -- cargo test -p cix --test tour -- --ignored generate_tour`; `devenv shell -- cargo test -p cix --test tour tour_matches_committed_document -- --exact`; `devenv shell -- cargo test -p cix --test tour generated_tour_is_deterministic -- --exact` (passed twice); `devenv shell -- nix build .#checks.x86_64-linux.vm-dogfood --no-link -L` (the daemon-owned VM completed successfully at `/nix/store/bqd2dm14hvi1ym2v9a5cn8d5xz2vypac-vm-test-run-vm-dogfood`). The workspace check covers 40 cix-cixfile parser/codegen tests, 54 cix-run tests, the 15 real-Nix compiler tests, proj1, and the rest of the workspace. Final `git diff --check` passed; active Cixfiles/docs/tour have no legacy STATE/JIT/EGRESS directives. Cleanup repro: `systemctl --user reset-failed 'cix-*' && systemctl --user stop cix-run.slice`; it leaves no cix user units. The untracked `devenv.lock` was created by `devenv shell` and is left untouched. Open with Mathijs: should LOGS/CONFIG follow the D52 naming family as LOGSDIR/CONFIGDIR? No decision taken.

- 2026-07-31T23:13:33Z — Started `.dev/specs/track-crunchy.md` on clean
  `track/crunchy` at `aa7a217`. Read AGENTS.md, the current session and Cixfile
  journals, the authoritative Cixfile decisions through the diagnostics-policy
  addendum, and the complete track spec; branch-local direnv/devenv is active.
  Scope is parser diagnostics, an adversarial 40–60-fixture snapshot corpus, and
  docs/cixfile.md anchors referenced by messages. Public diagnostics must never
  expose design-journal identifiers; forgiveness is limited to syntax variance
  that cannot change meaning. Ownership fences: do not touch `build_chain`,
  cix-index, or scenarios. Next: inventory the live parser tables, diagnostic
  strings, test structure, and documentation anchors before designing the
  fixture/grade format.

- 2026-07-31T23:25:58Z — The 60-file adversarial corpus and diagnostic
  snapshot/grade harness are focused-green. Each error fixture carries a human
  `problem/line/fix/docs` grade; the harness checks its exact committed
  snapshot, physical line, single-line/260-byte terseness budget, absence of
  public design-journal identifiers, cited text, and existence of the explicit
  public anchor. Six meaning-preserving variants are accepted (tabs/spacing,
  CRLF, blank lines, trailing whitespace, indentation, continuation); the
  other 54 reject ambiguity without aliases. Exact focused repros:
  `devenv shell -- cargo test -p cix-cixfile --lib` (45 passed) and
  `devenv shell -- cargo test -p cix-cixfile --test diagnostics` (60 fixtures,
  60 snapshots). Baseline was independently reproduced from `aa7a217`; exact
  final text lives in each committed `.snap`. Per-fixture before → after:
  - `01_typo_servic`: bare unknown → did-you-mean `SERVICE`.
  - `02_typo_improt`: bare unknown → did-you-mean `IMPORT`.
  - `03_typo_exposed`: bare unknown → Docker `EXPOSE` recognition + `PORT`/mapping anchor.
  - `04_typo_fromm`: bare unknown → did-you-mean `FROM`.
  - `05_lowercase_from`: bare unknown → uppercase-only hint + `FROM`.
  - `06_mixedcase_from`: bare unknown → uppercase-only hint + `FROM`.
  - `07_docker_from_ubuntu`: generic missing universe → Docker non-inheritance + exact universe line/mapping anchor.
  - `08_docker_run_apt_get`: internal doctrine label → `IMPORT`/BUILDER migration + mapping anchor.
  - `09_docker_workdir`: bare unknown → fixed `/work` model + delete/adjust fix.
  - `10_docker_cmd`: bare unknown → `EXEC` in SERVICE/APP.
  - `11_docker_entrypoint`: bare unknown → `EXEC` in SERVICE/APP.
  - `12_docker_expose`: bare unknown → named `PORT` mapping.
  - `13_docker_user`: bare unknown → delete `USER`, dynamic-user explanation.
  - `14_docker_copy_from`: arity error → named-binder `COPY` rewrite.
  - `15_docker_add`: bare unknown → explicit `COPY`/`FETCH` split.
  - `16_docker_volume`: bare unknown → exact role-directory family.
  - `17_docker_arg`: bare unknown → explicit inputs/builder `ENV`.
  - `18_missing_as`: generic FROM usage → reconstructed line with missing `AS` inserted.
  - `19_wrong_copy_order`: invalid relative source → exact swapped `COPY` line.
  - `20_link_old_order`: internal change label → target/link-path order + public anchor.
  - `21_copy_outside_block`: internal block label → complete legal block list.
  - `22_run_outside_builder`: internal doctrine label → add `BUILDER <name>`.
  - `23_item_exec`: internal seam label → content-only ITEM + SERVICE/APP fix/anchor.
  - `24_app_setup`: internal block label → move preparation into APP executable.
  - `25_app_port`: internal block label → move `PORT` to SERVICE.
  - `26_relative_artifact_copy`: internal path label → exact leading-slash rewrite/anchor.
  - `27_absolute_builder_copy`: generic relative-path error → exact workdir-relative spelling.
  - `28_duplicate_binder`: problem only → first line + shared namespace + rename fix.
  - `29_duplicate_exec`: problem only → remove one `EXEC`.
  - `30_empty_service`: requirement only → add exactly one `EXEC` inside named block.
  - `31_unknown_binder_typo`: misleading package guess → did-you-mean `compile`.
  - `32_unknown_namespace_typo`: namespace inventory → did-you-mean `pkgs`.
  - `33_malformed_attrpath`: generic grammar → bad suffix + exact interpolation shape.
  - `34_namespace_without_attr`: example retained unchanged (already problem/line/fix complete).
  - `35_artifact_attr_syntax`: internal rule label → source-tree spelling + inputs anchor.
  - `36_unterminated_exec_quote`: problem only → add matching quote.
  - `37_smart_quotes`: silently accepted → reject with ASCII quote replacement.
  - `38_dangling_continuation`: problem only → remove backslash or add next line.
  - `39_run_heredoc_missing_close`: expected token only → exact unindented closing line.
  - `40_run_heredoc_quoted`: misleading unterminated error → unquoted delimiter rewrite.
  - `41_file_heredoc_missing_delimiter`: retained (already names required delimiter fix).
  - `42_file_heredoc_indented_close`: expected token only → exact unindented closing line.
  - `43_stray_equals`: bare unknown → remove unexpected `=`.
  - `44_stray_colon_directive`: bare unknown → did-you-mean `SERVICE`.
  - `45_inline_comment_extra_args`: usage only → usage + full-line-comment rule.
  - `46_legacy_state`: internal rename label → `STATEDIR` + role-dir anchor.
  - `47_legacy_jit`: internal replacement label → `GRANT jit` + grants anchor.
  - `48_legacy_pkg`: internal removal label → exact `${pkgs.hello}` rewrite + inputs anchor.
  - `49_legacy_take`: internal removal label → exact binder `COPY` rewrite + copy anchor.
  - `50_legacy_cache`: internal removal label → delete line + persistent-workspace anchor.
  - `51_accept_whitespace_tabs`: accepted → accepted.
  - `52_accept_crlf`: accepted → accepted (fixture has real CRLF terminators).
  - `53_accept_blank_lines`: accepted → accepted.
  - `54_accept_trailing_whitespace`: accepted → accepted.
  - `55_accept_indentation`: accepted → accepted.
  - `56_accept_continuation`: accepted → accepted.
  - `57_legacy_path`: internal replacement label → builder `IMPORT` + builders anchor.
  - `58_legacy_script`: internal removal label → copied-script/explicit-shell rewrite + copy anchor.
  - `59_legacy_logs`: internal rename label → `LOGSDIR` + role-dir anchor.
  - `60_legacy_outbound`: internal replacement label → `GRANT egress` + grants anchor.
  Parser/codegen residue scan now finds design identifiers only in the migration
  table's deliberately internal `decision` fields; no snapshot contains one.
  Next: run workspace regressions, audit the full Cixfile diagnostic surface and
  scope, then execute the complete tour/determinism/VM gate.

- 2026-07-31T23:34:48Z — Final staged-snapshot gate is green. Exact repros:
  `devenv shell -- cargo fmt --all --check`; `devenv shell -- cargo clippy
  --workspace --all-targets -- -D warnings`; `devenv shell -- cargo test
  --workspace`; `devenv shell -- cargo test -p cix --test tour -- --ignored
  generate_tour`; `git diff --exit-code -- docs/tour`; `devenv shell -- cargo
  test -p cix --test tour tour_matches_committed_document -- --exact`;
  `devenv shell -- cargo test -p cix --test tour
  generated_tour_is_deterministic -- --exact` (passed twice); and `devenv
  shell -- nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`. The
  first VM attempt correctly proved Nix's tracked-source boundary by refusing
  the then-untracked new parser module; after staging the audited intended
  source set, the exact VM command passed under normal KVM-denied TCG fallback,
  confirmed by `devenv shell -- nix path-info
  .#checks.x86_64-linux.vm-dogfood` at
  `/nix/store/frczhmdmqc7ywwxm54ryv47di3ay9rnh-vm-test-run-vm-dogfood`.
  `git diff --cached --check` passes with fixture-local whitespace attributes
  documenting the intentional CRLF and trailing-whitespace cases. Scope audit
  finds no `build_chain`, cix-index, or scenario paths. Test-created user/system
  cix units were reset/stopped and both managers list none; the generated
  untracked `devenv.lock` and the exact temporary baseline checkout were
  removed. Next: re-stage this final record, review the cached diff summary,
  commit on `track/crunchy`, and verify the committed tree is clean.

- 2026-07-31T23:35:57Z — Committed the green adversarial-diagnostics
  implementation on `track/crunchy` as `804fab5` (`Improve Cixfile diagnostics
  under adversarial syntax`). No open items remain for this track; this final
  journal-only close records the completed commit and leaves the branch ready
  for independent verification.

- 2026-08-01T03:00:00Z — Started `track/fmt` after reading `AGENTS.md`,
  `.dev/specs/track-fmt.md`, the current journals, and D53/D74. Scope: a
  separate lossless Cixfile formatter, parse-gated CLI/discovery/check mode,
  golden+torture+CLI coverage, repo adoption, docs, gate wiring, lock-stability
  proof, the complete prescribed gate, and a scoped commit. `devenv shell --
  true` confirms the development environment is active. Next: map parser
  lexical rules and implement the formatter without changing the semantic
  parser.

- 2026-08-01T03:25:00Z — Implemented the separate `cix_cixfile::fmt` lossless
  physical-line scanner and the `cix fmt` command. It calls the real parser
  before scanning, tracks parser-compatible continuations and RUN/FILE
  heredocs, preserves comments and heredoc body/terminator text, and normalizes
  the v1 layout outside them. CLI coverage proves unified `--check` diffs,
  stdin, rejected-input no-write behavior, .gitignore discovery, explicit-file
  behavior, and unchanged mtime; golden/CRLF tests plus the complete torture
  sweep prove idempotence and semantic equivalence excluding diagnostic
  provenance. Focused repros currently green: `devenv shell -- cargo test -p
  cix-cixfile --test fmt` and `devenv shell -- cargo test -p cix --test fmt`.
  Next: documentation/gate wiring review, commit the implementation, format
  examples in its required separate commit, then lock proof and full gate.

- 2026-08-01T05:25:00Z — STOPPED at the D74 lock-stability gate, before
  committing the separately reformatted examples or claiming the full gate.
  `cix fmt examples` and the required ignored tour regeneration succeeded, but
  the regenerated proj1 transcript changed builder memo keys solely from the
  new Cixfile indentation. The concrete leak is
  `crates/cix-build/src/build_chain.rs:763`: COPY step key material includes
  `copy.source`, the physical directive text, rather than the parsed COPY
  semantics. An isolated reformatted proj1 refresh (`CIX_BUILD_WORKSPACE_DIR=
  /dev/shm/cix-fmt-lock.fgIocj/workspaces TMPDIR=/dev/shm target/debug/cix build
  --update-lock build /dev/shm/cix-fmt-lock.fgIocj/proj1`) produced a changed
  lock (original SHA-256 `1226eb…61519`, refreshed `3b7b3b…ea2ae`) with the new
  memo key `d505b7…f6d8b0`. This is exactly the keying leak the track says to
  surface rather than paper over. Implementation commit `7f99aa7` is complete;
  unstaged adoption/gate changes remain for a follow-up after a decision on
  fixing the build-key provenance dependency.

- 2026-08-01T05:27:28Z — Merged `origin/main` as required by the D74
  addendum (`21d5993`), preserving the already-uncommitted formatter adoption
  work. The addendum confirms this is a D48a/D69 bug, not a design decision.
  Root-cause work resumed at `builder_chain_keys`: the COPY arguments presently
  hash `copy.source` verbatim beside `copy.dst`, while the source content is
  separately nar-hashed. Next: make the semantic COPY key explicit, prove the
  exact before/after key material, add the formatting/lock regressions, then
  finish adoption and the prescribed full gate.

- 2026-08-01T05:32:00Z — Root cause pinned down and fixed. The old first
  proj1 COPY key arguments were `COPY ${src}/rust/ .\0.`; formatting changed
  only that first component to `  COPY ${src}/rust/ .\0.`. The parsed `src`
  template (`Binder(src) + Literal(/rust/)`), parsed destination `.`, declared
  source nar hash, imports, environment, and predecessor are otherwise the
  same. `builder_chain_keys` now serializes only the semantic template parts
  (without diagnostic line numbers) and destination, plus the pre-existing nar
  hash. The codegen fingerprint is bumped from `d69-v1` to `d74-v1`, so old
  memo entries are deliberately orphaned for one alpha cold rebuild. Repros
  green: `devenv shell -- cargo test -p cix-build
  copy_key_arguments_exclude_physical_directive_provenance --lib` and
  `devenv shell -- cargo test -p cix-cixfile --test fmt
  formatting_preserves_builder_keys_and_clean_update_lock -- --exact`.
  The latter runs two isolated clean `--update-lock build` builds of an
  unformatted/formatted fixture and proves byte-identical locks (hence builder
  memo key). Next: re-prove D69 pinkeys after workspace wipes, then complete
  adoption and the full gate.

- 2026-08-01T05:36:00Z — Committed the required standalone repository adoption
  as `d9077bc` (`Apply Cixfile format canon to examples`): 14 live example
  Cixfiles only. The unrelated generated untracked `devenv.lock` was removed.
  Remaining uncommitted scope is the COPY-key correction/regression, tour
  regeneration, CI formatting check, and this append-only log. Next: rerun
  the tour for the D74 fingerprint keys and execute the D69 workspace-wipe
  acceptance before the full gate.

- 2026-08-01T05:39:00Z — Re-proved the D69 pinkeys acceptance with a fresh,
  disposable automatic-FETCH fixture: clean `--update-lock build` under
  `d69-workspace-a`, moved that workspace aside (the wipe), reset to the clean
  input lock, then repeated under fresh `d69-workspace-b`. Both generated
  `Cixfile.lock` files are byte-identical (`d430fb09…a5a7b7`), despite distinct
  timestamped `.npm/_logs` volatile probe facts, and `cmp -s` passed. Next:
  regenerate the tour with the new D74 fingerprint, then run the full gate.

- 2026-08-01T05:40:00Z — Full track gate is green. Passed: `devenv shell --
  cargo fmt --all --check`; `devenv shell -- cargo clippy --workspace
  --all-targets -- -D warnings`; `devenv shell -- cargo test --workspace`;
  `devenv shell -- cargo run -- fmt --check examples`; tour regeneration,
  committed-doc matching, and deterministic generation twice; and the required
  full `TMPDIR=/dev/shm devenv shell -- nix flake check -L` (69 checks,
  including VM scenarios; the immediate repeat reported all checks previously
  built). The regen updates the RUN and proj1 transcript key receipts to the
  deliberate `d74-v1` namespace. `git diff --check` and cached diff checks
  pass. Next: remove generated scratch artifacts, stage this remaining scoped
  correction, review, and commit.

- 2026-08-01T05:41:06Z — Committed the scoped COPY-key correction, regression,
  CI formatter check, regenerated tour receipts, and journal as `862975b`
  (`Preserve COPY memo keys across formatting`). Together with standalone
  adoption `d9077bc` and formatter implementation `7f99aa7`, track/fmt is
  complete. Worktree is clean; the temporary `/dev/shm/cixfmt-key.eCexDX`
  acceptance workspace and generated `devenv.lock` have been removed.
- 2026-08-02T08:17:41Z — Started `.dev/specs/track-corpusweb.md` on
  `track/corpusweb`. Read the current project journal, authoritative design
  registry, complete track spec, and this crate journal. Scope is the corpus
  fold, a sharpened open-gaps ledger, and a deterministic self-contained HTML
  corpus browser generated beside (but independent of) the tour harness. The
  concurrency fence excludes `docs/tour/` and `crates/cix-run`; both will stay
  untouched. `crates/cix-cixfile/LOG.md` is tracked in this worktree despite
  the shared ignored-log convention, so this task journal will remain
  uncommitted. Next: inventory the corpus/ledger shape and existing generator
  test conventions, then implement the fold and browser model.

- 2026-08-02T08:43:00Z — Folded the isolated Renovate regrade into the
  living `corpus/migrate/renovate` layout and removed `corpus/regrade/`.
  Added SOURCE notes pinned to renovatebot/helm-charts revision
  `f953571cd7d10fd301799192dbaf18c55bd1dad0`, the verbatim upstream CronJob
  template, and a bounded check that covers the cix build, native calendar
  parse, and compose validation without host activation; the existing timer,
  run, and indexed-log receipt remains the stronger runtime proof. Added the
  independent `crates/cix/tests/corpus.rs` generator and generated 21
  self-contained pages (index + 20 cases), including Wallos's D4 `default.nix`
  escape-hatch case. The first run caught an ambiguous receipt link in the new
  four-column gap table; restricting ribbon extraction to the six-column
  survey evidence cell fixed it. Exact focused pass: `devenv shell -- cargo
  test -p cix --test corpus -- --ignored generate_corpus_browser --nocapture`.
  Next: inspect the rendered artifacts and link/ribbon projections, run normal
  drift/determinism plus the Renovate static check, then begin the full gate.

- 2026-08-02T09:02:00Z — Focused review and all pre-flake gates are green.
  Added a six-column living-corpus ledger for all 20 cases after the first HTML
  review showed that 17 receipt-bearing migrations had no explicit wild-survey
  ribbon; the generator now hard-fails when any cix-bearing case lacks a ledger
  ribbon/evidence row. Case pages project that row's empirical summary as the
  requested receipt-status line. Exact passes: Renovate `check.sh` (same locked
  item, calendar parse, compose validation); `devenv shell -- cargo fmt --all
  --check`; `devenv shell -- cargo run -- fmt --check examples`; warning-denied
  workspace/all-target clippy after correcting one `write!` newline lint;
  `devenv shell -- cargo test --workspace`; corpus regeneration plus normal
  drift/determinism; tour regeneration, zero `docs/tour` diff, committed-tour
  match, and deterministic-tour test. The staged fence contains neither
  `docs/tour/` nor `crates/cix-run/`; only this LOG remains unstaged. Next: run
  the required full `devenv shell -- nix flake check -L`, then final audit and
  commit.

- 2026-08-02T08:38:05Z — Required final gate and scope audit are green.
  `devenv shell -- nix flake check -L` exited 0 across its complete 64-check
  matrix, including `scenario-update-repin`, `scenario-observability`,
  `scenario-devices`, `scenario-gc-survival`, `scenario-side-by-side`,
  `compose-fallback-vm`, `scenario-lifecycle`, and `vm-dogfood`. Expected TCG
  fallback and D36 PrivatePIDs probe messages did not fail the checks. Final
  `git diff --cached --check`, `git diff --check`, and the committed-tour diff
  are clean; no live `corpus/regrade` reference remains outside historical
  logs/specs, and the staged fence still excludes `docs/tour/` and
  `crates/cix-run/`. Next: commit the staged track deliverables while leaving
  this required task journal unstaged.

- 2026-08-02T08:38:30Z — Committed the complete corpusweb track as `2d1794e`
  (`docs: add migration corpus browser`): Renovate's migration fold and proof,
  the complete status/gap ledger, the independent deterministic corpus browser
  generator and 21 generated pages, and documentation entry points. This
  append-only task journal is the sole remaining worktree modification and is
  intentionally uncommitted per the track instructions.

- 2026-08-02T12:31:29Z — Began `track/readset` at `81586c5`. Read the track
  spec, adopted CIP-87, current project journal, design registry, assigned
  journal, builder executor/lock schema, hermetic ergo fixture, and affected
  docs/example. The prerequisite `--stats`, output receipts, persistent warm
  underlay, COPY staging, and cold FETCH replay are present. Chosen trace
  mechanism: host `strace -f` around the existing bubblewrap boundary, limited
  to filesystem/process syscalls and decoded into workspace-relative reads.
  This captures regular reads, readdir calls, and failed lookups across child
  processes without weakening the sandbox; fanotify was rejected because it
  cannot meet the negative-lookup requirement. Inputs will be hashed from a
  pre-step workspace snapshot so later writes cannot corrupt the observed
  input value. A step memo must also retain and replay its filesystem delta;
  merely skipping a hit would be unsound when an earlier executed step touched
  the same output. Known failure modes to guard and document: strace must be on
  the runtime PATH; path decoding and cwd/dirfd tracking must cover child
  processes; tracing only `/work` deliberately relies on the static key for the
  immutable offered closure and fixed skeleton; ptrace-denying hosts must fail
  closed rather than run untraced; snapshot/diff cost favors completeness over
  speed. Next: add the trace/delta lock model and executor plumbing with unit
  coverage, then extend the hermetic acceptance fixture.

- 2026-08-02T12:52:19Z — Implemented the constructive-trace engine and passed
  its first acceptance round. `strace -f -yy` now wraps bubblewrap without
  weakening its namespaces/seccomp; the parser follows child/cwd/dirfd paths,
  records `/work` regular reads, true `getdents64` listings, positive directory
  existence, and ENOENT probes, while filtering write-only destination probes.
  Each static step identity excludes the predecessor/COPY bytes and owns one
  latest `stepMemo` read set plus a replayable precise filesystem delta. Traced
  writes (including atomic rename destinations) keep deltas reconstructive even
  when a repeated command writes byte-identical output. COPY staging and the
  old chain remain the persistent-workspace lineage; the builder fingerprint is
  now `d87-v1` and also participates in completed-output receipts, honestly
  orphaning old memos. Cold RUN executes and compares the actually observed
  read set; cold FETCH validates its recorded inputs and replays only its delta,
  preserving fresh copy-everything inputs and never refetching. Cold no longer
  mutates/revalidates FETCH pins against later RUN output. `--stats` now reports
  actual per-step outcomes from the executor rather than labeling the whole
  invocation uniformly. Extended the local-HTTP fixture: manifest edit executes
  FETCH; source-only edit hits FETCH and executes RUN; creating a probed absent
  file and adding a listed-directory entry each execute RUN; repeat is zero-Nix
  all-hit; cold byte-converges. Exact focused test and trace unit tests pass;
  workspace all-target check, Rust fmt, examples fmt, and diff check pass.
  Flake/devenv runtime PATH now includes strace. Docs teach copy-everything and
  the Docker/corpus ledgers are regraded. Next: exercise the full existing
  builder suite, regenerate the gitsitter lock, then obtain dated no-op/edit
  receipts and update docs/nix-build.md.

- 2026-08-02T13:01:55Z — The full cix-build/cix-cixfile test pass exposed the
  intended new cold-audit boundary in the older proj1 fixture. Its RUN
  deliberately probes warm-only Cargo state and `target/.cix-warm`; outputs
  happen to converge, but CIP-87 requires cold to reject the differing actual
  read set at the RUN source line. Updated the integration assertion and the
  executable tour source to teach that diagnostic, while the new hermetic
  read-set fixture remains the acceptance proof for normal/repeat/cold byte
  convergence. Earlier broad failures in delta permissions, newly consumed
  outputs, append tracing, and update-pin volatile paths were repaired; their
  focused regressions all pass. Next: rerun the broad suite, regenerate and
  drift-check the tour, then remeasure gitsitter and finish the prescribed
  gates.

- 2026-08-02T13:35:24Z — The required gitsitter receipt found and closed two
  completeness issues hidden by the tiny fixture. Successful `stat` probes
  were initially promoted to content reads, so Cargo metadata invalidated
  FETCH on source bytes it only tested for existence; the lock model now has
  explicit file/directory existence markers while opened files remain content
  hashes. Cargo also inspected its warm `target/` during `cargo vendor`; the
  copy-everything fixture now points that command's target directory at the
  sandbox's ephemeral `/tmp`, preserving the intended manifest/lock boundary.
  The measurement harness copies the complete upstream tree, substitutes the
  pinned revision for the local binder's unavailable `${src.rev}`, and forces
  two untimed primes so the single latest trace represents the warm underlay.
  Synchronous 2026-08-02 receipts: no-op 0.34 s, zero Nix subprocesses, every
  directive `memo-hit`; patched `src/main.rs` 84.83 s, FETCH `memo-hit`, RUN
  executed, 11 Nix subprocesses. The completeness-first trace is honestly much
  slower than the pre-CIP receipt. Added repeated-delta root retention so later
  traces preserve replayable roots instead of enumerating every touched child;
  focused FETCH/cold and complete proj1 tests pass. The generated real-world
  step memo was deliberately not committed into the fixture lock because it is
  host-local memo state and made the example lock hundreds of thousands of
  lines; the committed input/fetch pin ledger remains unchanged. Next: update
  and drift-check the executable tour, run all prescribed Rust gates, then the
  full flake check.

- 2026-08-02T13:49:21Z — Final semantic audit corrected the memo index from a
  key-derived collection to one stable owner slot per FETCH/RUN, with the
  static key stored in that slot. A changed directive now replaces its prior
  trace instead of retaining stale alternatives, satisfying CIP-87's exactly
  one-latest invariant. Trace paths now use reversible hex encoding when they
  are not UTF-8, and the syscall filter/parser also covers `getdents` and
  `creat`. Formatting, cix-build unit tests (20/20), the hermetic read-set
  acceptance fixture, example formatting, and diff hygiene pass from this
  final code state. Next: rerun clippy, all workspace tests, tour drift and
  determinism, then the full flake gate.

- 2026-08-02T13:58:40Z — Track gate is green from the final staged source
  set. Exact synchronous repros: `devenv shell -- cargo fmt --all --check`;
  `devenv shell -- cargo run -- fmt --check examples`; `devenv shell -- cargo
  clippy --workspace --all-targets -- -D warnings`; `devenv shell -- cargo
  test --workspace`; `devenv shell -- cargo test -p cix --test tour
  generate_tour -- --ignored --exact`; the exact
  `tour_matches_committed_document` and `generated_tour_is_deterministic`
  tests; and `devenv shell -- nix flake check -L`. All exited 0, with the
  flake runner reporting all 70 checks and every VM scenario green. The first
  flake attempt correctly failed because the new trace module was untracked
  and therefore absent from Nix's Git source; staging the implementation
  while leaving this LOG unstaged made the second run test the actual commit
  source and pass. `git diff --cached --check` also exits 0. All CIP-87 scope,
  acceptance coverage, ledgers, tour output, and the dated gitsitter no-op and
  one-line-edit receipts are complete. Next: commit `track/readset`.

- 2026-08-02T13:59:11Z — Committed the completed track as
  `7e7aadf28273885506029782c9ba357123940f9f` (`build: key steps by traced read
  sets`). The worktree now differs from the commit only by this intentionally
  unstaged LOG. No implementation or documentation work remains for the
  track.

- 2026-08-02T14:10:00Z — Resolved the in-progress merge of main's D70 overlay
  universes and CIP-81 secrets into CIP-87 read-set memoization. The merged
  executor keeps the read-set `(outputs, ExecutedStep stats)` result while
  loading one host-local consent manager from `--allow-secret` and passing it
  only to FETCH execution/probes; RUN remains credential-free. Both chain keys
  and constructive step-memo static keys now contain the resolved base pin +
  ordered overlay-hash universe identities, and the vendored dev-environment
  cache retains that same identity. Credential names/paths remain runtime-only:
  they are injected directly into the sandbox, never into the declared
  environment or either key, and the trace parser persists only `/work` reads;
  its regression now explicitly ignores `/run/cix-credentials/private`.
  Bumped the combined fingerprint to `d87-v2` so neither parent track's memos
  can cross the changed semantics. Took the read-set tour side temporarily,
  then regenerated all tour output from the merged binary. The first requested
  package-test attempt synchronously exposed one readset fixture constructor
  missing main's new `allow_secret: false`; fixed it and reran. Green exact
  receipts: `devenv shell -- cargo test -p cix-build -p cix-cixfile` (25
  cix-build unit tests plus every cix-cixfile unit/integration/doc test),
  `devenv shell -- cargo test -p cix --test tour generate_tour -- --ignored
  --exact`, and the exact `tour_matches_committed_document` and
  `generated_tour_is_deterministic` drift tests. Next: final index audit and
  merge commit.

- 2026-08-02T14:11:15Z — Committed the resolved two-parent merge as
  `568205f9a1bbc07b65f04eafdbbd5bf74cb9ae34` (`Merge main into
  track/readset`). Parents are readset `f9a870c` and main `857d185`; no merge
  state or implementation work remains. Per AGENTS.md, this task LOG and
  main's cix-run task LOG remain intentionally unstaged.

- 2026-08-02T14:59:42Z — Started `track/corpusstyle` at `c3821fe`. Read the
  complete track spec, current project journal, authoritative D70/D74 and
  Cixfile design context, the unadopted `cips/draft/file-from.md`, assigned
  journal, migration/cixfile teaching docs, Wallos case, and corpus browser
  generator. The devenv environment is active. Scope is the file-first and
  readable-RUN authoring canon, a mechanical corpus sweep with a full Wallos
  exemplar rewrite, generation-time syntax highlighting and tighter laptop
  typography, all regenerated corpus pages, and synchronous checks for every
  touched corpus case. Fences: no `crates/cix-run`, `nix/scenarios/lib.nix`,
  or closed-root/vmslim scenario work. Next: inventory all corpus heredocs and
  RUN chains, then edit only mechanically safe cases before adding lexer tests.

- 2026-08-02T15:28:00Z — Completed the corpus smell inventory and first
  implementation round. Wallos was the only case with FILE heredocs and the
  only RUN exceeding the two-command chain limit; echo-server and phpmyadmin's
  two-command RUNs already satisfy the canon. Wallos now has six readable
  read-set-keyed RUNs, real sibling setup/start scripts copied into the item,
  and an explicit comment retaining its paired config heredocs for package
  interpolation until the unadopted FILE … FROM draft lands. Migration and
  Cixfile docs teach the same file-first and RUN-shape rules. The corpus
  generator now has generation-time Cixfile/Dockerfile/Nix/JSON/YAML lexers,
  escaped inline spans, restrained light/dark palettes, 13px code, tighter
  chrome, laptop-safe two-column scrolling, and includes supporting .nix,
  config, and script files. Rust formatting, Wallos Cixfile formatting, diff
  hygiene, lexer tests, deterministic rendering, and explicit regeneration all
  pass; the pre-regeneration drift failure was expected and observed. Next:
  run `corpus/migrate/wallos/check.sh cix` synchronously, update its receipt and
  stale health ledger wording, then prove committed browser drift green.

- 2026-08-02T15:10:16Z — Focused behavior and browser receipts are green. The
  first Wallos check stopped before build because its ignored context fixture
  was absent; the pinned `../fetch.sh wallos` restored revision `3a7f965…`.
  The next attempt executed all six rewritten RUNs but failed in a generated
  manifest derivation because `/` and `/nix/store` had zero available bytes.
  I moved one exact 974 MiB disposable Nix Git-fetch cache directory, without
  deleting it, to `/tmp/composix-corpusstyle-nix-gitv3-backup`; this exposed
  25 GiB of reclaimable sparse allocation and the rerun passed synchronously.
  Exact receipt: `CIX=…/target/debug/cix ./check.sh cix` exited 0, reused
  `/nix/store/rds1fgd05lf8hh46i7h67inc2kxyw28c-cix-item-wallos`, started
  `cix-run-wallos-18c8059869b7e40c0.service` under the expected D36 fallback,
  and returned `PASS cix` after `/health.php`. Updated the receipt and stale
  corpus ribbon to say CIP-79 READINESS is available but undeclared. Fresh
  browser generation and the complete corpus test binary pass: four normal
  tests (lexer safety, deterministic output, committed drift) green, one
  generator ignored. Next: prescribed fmt/examples/clippy/workspace/tour gates.

- 2026-08-02T15:42:00Z — Prescribed broad gates are green from the final
  generator implementation. `devenv shell -- cargo fmt --all --check` and
  `devenv shell -- cargo run -- fmt --check examples` exited 0. The first
  warning-denied clippy pass caught one redundant `trim_start` before
  `split_whitespace`; removing it is output-neutral, and the exact rerun of
  `cargo clippy --workspace --all-targets -- -D warnings` exited 0. Full
  `devenv shell -- cargo test --workspace` exited 0, including corpus drift,
  lexer safety, deterministic corpus output, tour drift/determinism, real-Nix
  builder coverage, and all docs tests. Fresh ignored tour regeneration exited
  0 and `git diff --exit-code -- docs/tour` proved zero drift; exact
  `tour_matches_committed_document` and `generated_tour_is_deterministic` both
  passed serially. One attempted cargo invocation with two positional test
  filters was rejected by cargo before running; each intended exact test was
  then run separately and passed. Per amended AGENTS.md, the full flake matrix
  belongs to the orchestrator's independent pre-merge gate, not this agent
  tier. Next: final corpus/style/fence diff audit, stage all deliverables except
  this journal, review the candidate, and commit on `track/corpusstyle`.

- 2026-08-02T15:48:00Z — Final staged-scope audit passed. The only retained
  FILE heredocs in the corpus are Wallos's two explicitly commented config
  blocks, and no corpus RUN contains more than two `&&` joins. All 21 pages
  contain inline CSS/spans only: no scripts, external stylesheets, imports, or
  CSS URLs. The staged fence contains no `crates/cix-run`,
  `nix/scenarios/lib.nix`, or `docs/tour` changes; `git diff --cached --check`
  and the unstaged diff check pass. Wallos's lock retains the new semantic
  builder memo and output source hash, but its generated `stepMemo` was removed
  before staging because those replay snapshots are host-local memo state (the
  established CIP-87 fixture convention); `jq empty` confirms valid JSON.
  Exactly 30 deliverable paths are staged, while this assigned journal remains
  the sole unstaged tracked path. Next: commit the complete green candidate.

- 2026-08-02T15:50:00Z — Committed the complete track as `4e76da0` (`docs:
  establish Cixfile corpus style`): authoring canon, Wallos exemplar and lock/
  receipt, generation-time five-language highlighting, responsive self-contained
  styling, lexer regressions, ledger correction, and all 21 regenerated pages.
  Restored the temporarily relocated Nix Git-fetch cache to its exact original
  path; `/` retains 22 GiB available. Post-commit `git diff --check`, zero
  `docs/tour` drift, and commit-scope review pass. The worktree differs from
  `4e76da0` only by this required unstaged journal. The branch was not manually
  pushed; `git ls-remote origin refs/heads/track/corpusstyle` was empty at the
  audit instant. No implementation, documentation, generation, focused receipt,
  or agent-side gate work remains.

- 2026-08-02T19:34:42Z — Fix round started after the orchestrator's closed-root
  inventory assertion exposed Mastodon as the 21st unclassified corpus case.
  The audit's current Renovate treatment proves that its corpus contracts are
  reproducibly reconstructed rather than rebuilt through Cixfile, but it does
  not exercise compose. `cix up --closed-root` is already phase-1 behavior, so
  claiming compose support is pending would be false; auditing only Mastodon's
  Redis member would also duplicate existing coverage without proving this
  migration. Chose the stronger honest classification: reconstruct all six
  checked-in item manifests and mounts, activate the actual compose topology
  under closed root, and probe Unix edges, shared-rw state, credential delivery,
  readiness, native watchdogs, and the scheduled cleanup member. Next: add that
  focused VM contract, run it synchronously, then rerun the migration check.

- 2026-08-02T19:46:05Z — First synchronous focused VM run correctly failed
  (`nix build .#checks.x86_64-linux.scenario-closedroot-audit -L`, exit 1)
  after all pre-existing unary audits passed. The full Mastodon activation
  exposed two real hidden host-root dependencies: Redis inherited the NixOS
  host locale and could not configure it inside the sealed root; PostgreSQL's
  ELF `initdb` internally invokes `postgres -V` through `/bin/sh`, which the
  closed root intentionally omits, so first-run initialization failed before
  the consumers. This was not a missing Nix closure: both executables use the
  same store dynamic loader, the item references the PostgreSQL closure, and an
  equivalent host-root init succeeded. Fixing the migration contract itself:
  declare Redis's C locale, and explicitly item-mount nixpkgs bash at `/bin/sh`
  for PostgreSQL's otherwise-hidden initdb requirement. The audit will retain
  the no-shell assertion for every other member and require PostgreSQL's shell
  to be a declared read-only item mount. Next: rebuild locks, rerun the VM, then
  run the ordinary-root migration check required by the fix round.

- 2026-08-02T19:48:33Z — The first rerun stopped early (exit 1) because a
  mechanical scenario edit accidentally added Mastodon's `/bin/sh` declaration
  to the pre-existing PostgreSQL pack manifest while placing its source only in
  the Mastodon item. This was a test-fixture wiring error, not a behavior
  result: cix correctly rejected the pack's declared-but-absent mount before
  reaching Mastodon. Restored the pack manifest and moved the mount declaration
  to the intended Mastodon manifest; next: re-evaluate and rerun synchronously.

- 2026-08-02T19:53:29Z — Third synchronous focused VM run reached the complete
  Mastodon stack: explicit `/bin/sh` made first-run PostgreSQL initialization
  succeed, the C locale made Redis ready, and the Python web/sidekiq consumers
  both authenticated over the projected Unix edges. Streaming alone failed in
  START_PRE, with the same scheduled cleanup shell command failing transiently.
  Root cause was the test fixture's secret writer: it omitted a final newline,
  while both shell consumers intentionally use `IFS= read -r` under `set -e`;
  the ordinary check writes a newline and the Python consumers tolerate either
  form. Corrected the VM fixture to reproduce the real secret byte shape. Next:
  rerun the full focused scenario synchronously and require exit 0.

- 2026-08-02T19:59:45Z — Fourth synchronous focused VM run brought the entire
  six-member Mastodon compose up successfully under closed roots: PostgreSQL
  initialized, Redis became ready, web and sidekiq authenticated over both
  projected Unix-socket edges, streaming served through nginx, and the cleanup
  timer fired. The test then failed only because it looked for the declared
  `/bin/sh` mount in the host-visible staging directory. Item mounts are
  systemd namespace bind mounts, so their targets need not exist in that host
  view. Retaining the real contract assertion on the unit's
  `BindReadOnlyPaths=...:/bin/sh`; the service's successful `initdb` execution
  is the behavioral proof that the mount was usable inside the namespace.
  Next: rerun the focused scenario synchronously and require exit 0.

- 2026-08-02T20:05:45Z — Fifth synchronous focused VM run again brought all six
  members up and passed every closed-root boundary assertion, including the
  PostgreSQL unit's declared `/bin/sh` bind. It then failed on an assertion that
  expected `systemctl show --property=LoadCredential` to expose the directive;
  systemd does not publish that setting as a runtime show property here. The
  web process had already authenticated with the credential, but to retain
  structural coverage as well, changed the assertion to inspect the installed
  unit with `systemctl cat` for the exact `LoadCredential=` directive. Next:
  rerun the focused scenario synchronously and require exit 0.

- 2026-08-02T20:12:29Z — Sixth synchronous focused VM run passed (exit 0):
  `devenv shell -- nix build
  .#checks.x86_64-linux.scenario-closedroot-audit -L`. Mastodon is honestly in
  `auditedCorpus`: the VM ran all six members in sealed roots and verified the
  declared PostgreSQL shell bind, absence of an ambient shell elsewhere,
  credential directive plus behavioral use, both Unix edges, shared state,
  readiness and watchdog survival, scheduled cleanup, member-scoped logs,
  purge, and removal of every root. The required ordinary `check.sh cix` then
  failed synchronously (exit 1) because the locked streaming item still
  existed but its manifest's external nginx store path had been collected;
  systemd reported `203/EXEC`. This is the already-recorded zero-reference/
  distribution gap, and the cached build did not re-realize the missing
  dependency. Changed the integration harness to explicitly `--update-lock`
  every member build so its runtime paths are realized before activation.
  Next: rerun `check.sh cix` synchronously and require exit 0.

- 2026-08-02T20:13:37Z — The immediate harness rerun failed before building
  (exit 1): clap interpreted the positional member path as the optional value
  of a bare `--update-lock`, leaving the repository root as the default build
  directory. Made the intended pin refresh unambiguous with
  `--update-lock=pkgs`; next: rerun `check.sh cix` synchronously.

- 2026-08-02T20:14:52Z — Final ordinary-root receipt passed synchronously
  (exit 0): `devenv shell -- ./corpus/migrate/mastodon/check.sh cix`, including
  its EXIT teardown. All six members built, the two-edge compose activated,
  readiness delay, credential, shared-rw markers, watchdog survival, timer,
  scoped logs, and purge passed. Explicit stability follow-up refreshed all six
  members again with `cix build --update-lock=pkgs`; exit 0 and checksum diff
  showed no byte change in any lock. Updated the receipt and corpus ledger to
  record Mastodon as the 21st migration and as a full-compose CIP-84 audited
  contract. Next: regenerate/drift-check corpus docs, inspect changes, and
  commit every task artifact except this LOG.

- 2026-08-02T20:18:38Z — Final verification/commit audit complete. Corpus
  browser regeneration passed and the non-ignored corpus suite passed 4/4,
  including deterministic generation and committed-page drift. `cargo fmt
  --all -- --check`, focused `cix fmt --check`, `bash -n check.sh`,
  `git diff --check`, and evaluation of the final closed-root check derivation
  all exited 0. The available pinned nixfmt wanted to reformat hundreds of
  pre-existing scenario lines; discarded that unrelated churn and retained the
  exact green-tested source snapshot, whose existing style is unchanged outside
  the new audit. Next: stage the ten task artifacts explicitly without this
  LOG, inspect the index, and commit on `track/mastodon`.

- 2026-08-02T20:19:08Z — Committed the fix round as `cd4f9b1` (`test: audit
  Mastodon under closed roots`). The commit contains exactly ten task
  artifacts; this append-only LOG remains the only unstaged worktree change as
  required. No open implementation items remain for this fix round.
- 2026-08-04T06:48:56Z — Started `track/cip91` from a clean branch with
  direnv/devenv active. Read the assigned spec, adopted CIP-91 contract,
  project journal/design context, and this crate journal. The track is
  explicitly spike-first: I am mapping the existing parser and Nix assembly,
  then will prototype the tomcat later-write, directus role-dir descendant,
  and linked-tree realpath shapes. No phase-2 behavior will be implemented
  until the spike verdict is recorded here; a sour verdict will stop at the
  contract's universal-IMPORT fallback. Next: identify the smallest assembly
  representation that keeps link/materialize mode text-derivable and both
  materialization triggers statically decidable.

## 2026-08-04 — track/cip91

- 2026-08-04T06:57:30Z — **Phase-1 spike verdict: clean; proceed to phase 2.**
  Store/package, BUILDER/FETCH, and cix-item binder sources are statically
  distinguishable from local/remote source contexts in the parsed Cixfile.
  A final text-only pass also decides both exceptions: any role-dir/`DIR` at
  or below the COPY destination, and any later COPY/FILE/LINK write below it.
  Chose materialize-on-later-write over refusal: it preserves ordinary ordered
  assembly (including Tomcat overlays) without new syntax. The first real-Nix
  run exposed that `cp -a` retains store directory read-only modes; making the
  private copy owner-writable during assembly fixed the later-write case
  without weakening the final immutable store result. Synchronous focused
  receipts passed: the Tomcat tree retained its base plus later patch;
  Directus's `${build}/dist -> /app` became a real directory solely because
  `STATEDIR /app/database`/`DIR /app/uploads` descend from it; and the realpath
  probe observed the accepted semantic difference exactly. Linked:
  `/nix/store/mnz7…-.tmpKEOKzA/tree/lib/probe.js`; materialized:
  `/nix/store/g58z…-cix-item-materialized-realpath/app/lib/probe.js`.
  Exact repros: `cargo test -p cix-cixfile --test parser
  d47_tests::store_copy_mode_and_structural_materialization_are_static --
  --exact` and `cargo test -p cix-cixfile --test lock_nix
  store_aware_copy_spike_covers_tomcat_directus_and_realpath -- --exact
  --nocapture`, both exit 0. The realpath hazard is the narrow, already
  accepted store-path visibility change, not a broader resolution failure.
  Next: phase 2 universal artifact IMPORT and LINK's deprecated COPY alias.

- 2026-08-04T07:08:25Z — Phase-2 language/assembly implementation is complete
  and focused-green. SERVICE/APP/ITEM now retain ordered IMPORTs, recursively
  union `bin`/`etc`/`share` with earlier-wins collisions, enumerate D22-safe
  sparse manifest mounts, and validate bare START/START_PRE command words even
  when argv follows. ITEM remains manifest-free. LINK now emits a deprecation
  warning and enters the exact COPY model path. Store-aware COPY links package,
  builder/FETCH, and cix-item sources, while context sources materialize; the
  two spike exceptions materialize automatically. Preserved D57/proj1 member
  selectivity by content-address-normalizing a narrow consumed builder/FETCH
  subpath before linking, so an unrelated sibling output cannot leak the
  changing aggregate view path into an item. Focused real-Nix union, spike,
  bare-command, and proj1 receipts all pass; complete `cargo test -p
  cix-cixfile` and `cargo test -p cix-build` also exited 0. Next: audit/stage
  only implementation and tests (leave this journal unstaged), commit the
  logical unit, then update docs/tour/ledgers.

- 2026-08-04T07:11:55Z — Committed the complete implementation/test unit as
  `2a67d44` (`build: implement CIP-91 artifact imports`). Rewrote
  docs/migrate.md's artifact canon to block-local IMPORT + bare argv, explained
  store-aware COPY and its structural materialization rules, replaced every
  interpolated START and LINK in its examples, and made native-path role dirs
  explicit. Re-graded the Dockerfile/COPY/multi-stage rows in docs/docker.md.
  The executable tour contains no LINK demonstration, so its sources/generated
  pages need no behavioral edit. Grepped every migration GAPS ledger: all 15
  artifact-import exhibitors now say `Status: stale — regenerate with CIP-91`
  and link the accepted CIP; no corpus Cixfile or generated browser page was
  touched. Next: commit this documentation/ledger unit, then run the prescribed
  standard tier and a focused closed-root VM consuming linked artifact content.

- 2026-08-04T07:23:14Z — CIP-91 is implementation-complete and green at the
  track gate. Committed documentation/ledgers as `5218e07` (`docs: teach
  CIP-91 artifact assembly`). The first standard-tier pass caught one Clippy
  `match_single_binding` and the expected generated corpus-browser drift after
  marking GAPS stale; fixed the former in `4c5e98a` and regenerated/committed
  the 15 affected browser pages in `6d1e0f1`, without changing any corpus
  Cixfile. Final synchronous exit-0 receipts on the committed source snapshot:
  `cargo fmt --all -- --check`; `cargo run -p cix -- fmt --check examples`
  (only the expected LINK deprecation diagnostics); `cargo clippy --workspace
  --all-targets -- -D warnings`; `cargo test --workspace -j 6`; tour generator,
  determinism, and committed-document drift tests; corpus-browser generator,
  determinism, and committed-page drift tests; and `nice -n 10 nix build
  .#checks.x86_64-linux.scenario-closedroot-audit -L --no-link --max-jobs 6
  --cores 4`. The focused VM fell back from unavailable KVM to TCG, completed
  its full linked-artifact/closed-root suite in 317.56 seconds, and the owning
  Nix build synchronously exited 0. `git diff --check` also exited 0 before the
  final commits. Next: re-run the short formatting/diff checks after journal
  bookkeeping, verify only this required uncommitted LOG remains, and hand off.
