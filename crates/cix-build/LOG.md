# cix-build work log

- 2026-08-06T02:00:00Z — Started `track/cip107-pinleg`. Read CIP-107 and the
  assigned spec. The mandatory order is regenerate every committed whole-tree
  FETCH `narHash` exhibit first, capture a synchronous build exit-0 and review
  each lock diff, then remove the whole-tree FetchPin read/write path while
  retaining an explicit reject-and-teach old-lock diagnostic. `direnv` has the
  devenv environment loaded. Next: enumerate the exact legacy entries and
  derive the corpus regeneration commands before making product edits.

## FRICTION

- 2026-08-06T03:24:54Z — Direct recursive removal of the three explicitly
  validated task workspaces was rejected by the command safety layer. Moved
  the three 194 MiB directories to the system trash with `gio trash` instead;
  the operation is recoverable and all three `/var/tmp` paths are absent.

- 2026-08-06T03:23:53Z — Putting the whole lock SHA in Valkey's in-case
  receipt created a source-hash self-reference because receipt files belong to
  the case directory hashed for completed-output invalidation. Removed that
  hash from the receipt. An attempted ordinary source-hash refresh then began
  a verifying RUN from the long-lived default workspace; it was interrupted
  before lock save (exit 130, unchanged lock) and is not a receipt. The final
  `--cold` run is the correct empty-workspace refresh and acceptance proof.

- 2026-08-06T03:06:00Z — Two command/test corrections were required and are
  not receipts: Cargo accepts one positional test filter, not two; and the
  first PID-namespace regression expected no observations but correctly kept
  the parent `chdir` existence observation. More importantly, an ordinary
  warm runtime check hit the completed-output memo and did not refresh the
  builder trace; only the explicit fresh-workspace `--update-lock build` run
  counts as the re-lock receipt.

- 2026-08-06T02:35:11Z — The first focused test command used an unqualified
  `--exact` filter and therefore passed with zero tests; it is not a receipt.
  The corrected fully qualified filter ran one test and exited 0. Reproduction
  also required the receipt's explicit context-fetch → warm-build → cold order:
  a cold run without the local store snapshot fails earlier, and a warm run
  from an old persistent workspace legitimately records a different pre-state.

- 2026-08-06T02:24:39Z — Started `track/valkey-coldtrace` from clean current
  main (`30a3af49`) with the devenv active. The committed Valkey receipt says
  the faithful warm build and runtime PING pass, while cold replay diverges on
  a random libtool file (`libbacktrace/.libs/stVX6SFe`, warm `Some(Absent)`,
  cold `None`). The accepted CIP-87 already treats same-step self-observation
  as output rather than input for FETCH, so the first question is whether the
  trace parser is merely retaining a negative probe that precedes a successful
  same-step create. Next: reproduce synchronously, retain the syscall evidence,
  and decide hygiene-versus-semantics from that evidence.

- 2026-08-06T02:00:00Z — Initial broad text search matched unrelated lock
  dependency `kind` values and large trace payloads; legacy FetchPin evidence
  must instead be identified structurally as `fetches` entries carrying
  `narHash`.

## Work

- 2026-08-06T03:24:54Z — Final post-generation audit is clean: corpus browser
  regeneration and its full parser/drift/determinism test exit 0; tour remains
  byte-unchanged; `git diff --check` exits 0. The complete before/final corpus
  lock hash diff names exactly one file, Valkey's `Cixfile.lock`, at final
  SHA-256 `2a71590645c5541826bd343dbe22e42c05b4460a3d22b7cbc98fa6a4dc5f333e`.
  All 13 changed tracked files are scoped product, receipt, ledger, generated
  browser, and journal updates. Next: commit without merging and verify clean
  local/remote branch state.

- 2026-08-06T03:23:53Z — Final exact-source gate is green. After removing the
  receipt self-reference, ultimate Valkey cold replay exited 0 (RUN 246183 ms)
  with the same item; final lock SHA-256 is
  `2a71590645c5541826bd343dbe22e42c05b4460a3d22b7cbc98fa6a4dc5f333e`.
  Full synchronous exit-0 agent tier: Rust fmt, examples fmt, warning-denied
  all-target workspace clippy, serialized workspace tests, corpus browser
  generation plus parser/drift/determinism suite, tour generation plus zero
  drift, source-size/module-map guard, diff check, and structural shared-state
  audit (no new sites). The progressive VM selector inspected the build
  surfaces, selected 0/14 because no declared VM contract intersects them, and
  exited 0 in 13.242s. Next: regenerate browser for the final lock bytes,
  recheck one-lock-only churn, remove the three task-owned temp workspaces,
  commit, and verify clean branch/remote state without merging.

- 2026-08-06T03:06:00Z — Acceptance is green. The finished fix has two
  observation-hygiene halves: successful exclusive read/write creates do not
  claim incoming content; and `strace --decode-pids=pidns` lets the parser map
  clone results to host PIDs, inherit cwd, and suppress observations at or
  below paths the same step successfully created while still retaining their
  writes. The fresh-workspace forced Valkey re-lock exited 0 (RUN 227877 ms),
  cold replay exited 0 (RUN 251075 ms), and the final warm runtime check exited
  0 with exact `PONG`. Both builds produced
  `/nix/store/fgm45ck2453mrpmhpv4hqhc64kcwa3f6-cix-item-valkey`; refreshed
  interim lock SHA-256 was `e6b82b3795affb455633b13f15b980bd9a5b5979027cd1311ef575b0e9b4a1be`.
  The lock read set drops over 12k self-output observations while retaining the
  two input tarball content hashes. A whole-corpus lock hash diff names Valkey
  as the only changed lock. Valkey is regraded current; HTTPD's matching
  generated-sed-path CIP-87 exhibit is marked stale for regeneration. Corpus
  browser regeneration exits 0. Next: complete agent tier, progressive VM
  selector, final structural/diff audit, commit, and leave a clean branch.

- 2026-08-06T02:35:11Z — Reproduced the committed failure synchronously from
  its intended empty pre-state: cold exited 1 at
  `libbacktrace/.libs/stVX6SFe` (warm `Some(Absent)`, cold `None`). Temporary
  targeted trace instrumentation captured the corresponding cold syscalls as
  successful `openat(..., O_RDWR|O_CREAT|O_EXCL, 0600)` calls with different
  random names, then was removed. `O_EXCL` proves the syscall created a fresh
  output and could not read incoming bytes, so this is classification hygiene,
  not an open semantics choice. The parser now excludes exclusive creates from
  `open_read` while preserving them in the write set; an existing `O_RDWR`
  path remains both read and written. Rust fmt and the one-test exact regression
  receipt exit 0. The Valkey check also now uses CIP-109's adopted absolute
  `tcp://` probe grammar. Next: regenerate only Valkey's memo from a fresh warm
  workspace and prove warm runtime plus cold replay.

- 2026-08-06T01:46:27Z — Directus is the mandatory stop: after restoring its
  pinned context, `target/debug/cix build corpus/migrate/docker/directus
  --update-lock build` synchronously exited 1. It completed the FETCH and
  install steps, then its offline production deploy could not resolve
  `@directus/tsconfig@4.0.0` because the pinned metadata cache lacks it. The
  attempted lock diff was reviewed and restored byte-for-byte to SHA-256
  `e40ee98df87de1bbf9a65b261c79f56987e0eb4b70ab1a3ece6a106906ea0d66`.
  `corpus/migrate/docker/directus/GAPS.md` now marks the case stale for the
  CIP-107 FetchPin migration. Per the spec, do not delete whole-tree FetchPin
  support while this committed exhibit remains.

- 2026-08-05T21:23:35Z — Reproduced the exact pre-fix CI failure before
  editing: the real binary under a one-core, nice-19, 2%-CPU user scope exited
  101 after `sigterm_removes_live_build_scratch` reported `cix did not create
  scratch under .../temp` (`/var/tmp/cip101-cifix-before-20260805T2115Z/`).
  Replaced deadline polling with a one-shot `CIX_SCRATCH_READY_FIFO` signal
  emitted after `ScratchDir` has lock-backed ownership, and serialized the
  binary's real-CLI tests. This does not lengthen a timing budget: the test
  blocks for the owner-confirmed path. The identical 2%-CPU receipt exits 0
  (3/3, 87.20s; `/var/tmp/cip101-cifix-after-20260805T2122Z/`), and the
  required constrained one-core/nice loop has 20 value-checked exit-0 runs
  (each 3/3; `/var/tmp/cip101-cifix-after-20260805T2126Z/`). Unconstrained
  `cargo test -p cix --test fetch_probe_cleanup -- --nocapture`, Rust fmt,
  warning-denied workspace clippy, `git diff --check`, and the shared-state
  audit all synchronously exit 0. The new mutex and one-shot signal each have
  site-local rationale comments. Next: final diff/status review; no VM run is
  needed for this test-only change, per the track spec.

- 2026-08-05T21:08:31Z — Started `track/cip101-cifix`. Read the assigned
  spec, current CIP-101 decisions, and the real-CLI cleanup test. The branch
  is clean at `452028dd`, direnv/devenv is loaded, and the only scoped issue
  is the slow-CI race in `sigterm_removes_live_build_scratch`: sibling tests
  in the same binary contend under Cargo's threaded harness before the polling
  test observes its scratch. Next: obtain a constrained, value-checked
  pre-fix failure, then make that binary deterministic without increasing its
  timing budget.

- 2026-08-05T20:11:54Z — Started `track/cip101-livelock` on the main-CI
  regression in `fetch_probe_cleanup::sigterm_removes_live_build_scratch`.
  Current `sweep_stale` decides solely from age, so the six-hour CIP-101
  amendment can delete another process's active scratch. The scoped repair is
  an on-disk owner lock held for each `ScratchDir`; the startup sweeper will
  skip a lock it cannot acquire and retain the aggressive removal policy for
  unlocked aged directories. Next: add real-CLI regression coverage for a
  live aged build plus an unlocked aged directory, then run the cargo tier.

- 2026-08-05T18:35:00Z — Started `track/cip103-memo`, CIP-103 leg 3. The
  branch is clean at `7adb1e8`, direnv/devenv is active, and the accepted
  decision makes the owned interface the acceptance condition: `build_chain`
  must send typed requests to a `MemoEngine` and receive typed verdicts, while
  the engine may reach persisted state only through `Workspace`. Next: map the
  memo/replay cluster and its conductor dependencies, establish a representative
  before receipt, then extract without widening the seam.

- 2026-08-05T18:45:00Z — Established the representative pre-change receipt
  with freshly staged Wallos source context. A synchronous warm build exited 0
  and produced `/nix/store/26wbmzxzyks6q0h41sl0zxs3gf4dgj6j-cix-item-wallos`;
  its generated lock SHA-256 is
  `7aab30d66afd0df16c16b87c4109324b697c01609b2f1948b006ecd4dd3a186d`.
  The generated lock and stdout are retained under
  `/var/tmp/cip103-memo-receipts/` for a value comparison after extraction.
  Next: define the typed memo requests/verdicts and move policy behind them.

- 2026-08-05T19:20:00Z — Extracted `memo.rs`. `MemoEngine` now owns key
  construction, output lookup, persisted memo state, validation policy,
  read/write reduction, cold comparison, and constructive replay. The conductor
  translates its `BuildContext` into purpose-specific requests and receives
  explicit chain/output/reuse verdicts; it no longer carries Workspace memo
  state or reaches its validation/replay primitives directly. Focused crate
  tests pass (41/41), the source-size/module-map guard exits 0, and the
  structural shared-state audit adds no sites. Representative after receipt:
  Wallos exited 0 with the same output path and byte-identical generated lock
  (`cmp` 0 for each; lock SHA-256 remains
  `7aab30d66afd0df16c16b87c4109324b697c01609b2f1948b006ecd4dd3a186d`).
  The generated corpus lock was restored to tracked HEAD after comparison.
  Next: review the interface/diff, then run the complete agent tier including
  the contract-keyed progressive selector.

- 2026-08-05T19:30:00Z — Warning-denied all-target workspace clippy exits 0
  on the finished interface. The VM axis is presently occupied by
  `track/cip109-probeurl`'s bounded 14-scenario progressive run (two active
  QEMU guests), so this track will run non-VM gates now and wait before its
  own selector rather than contend for TCG capacity.

- 2026-08-05T19:45:00Z — Non-VM agent tier is green with synchronous exit-0
  receipts: fmt, examples fmt, warning-denied clippy, full workspace tests,
  corpus regeneration plus zero drift, tour regeneration plus zero drift and
  its exact committed-document test, source-size/module-map guard, and diff
  check. The first progressive-selector attempt is explicitly NOT a receipt:
  CIP-109 restarted its two-slot matrix after this track's clean preflight, so
  this track interrupted only its own source build at 38.161s (exit 1) before
  any guest to preserve the shared bound. Waiting for CIP-109 to release the
  axis, then rerun from scratch.

- 2026-08-05T18:59:00Z — Timestamp correction: the preceding entries stamped
  `19:*` were written one hour ahead of the UTC host clock; their file order is
  the intended chronology. Merged current `origin/main` (`b57ee13`) after the
  verified extraction checkpoint. The only conflicts were rewritten-history
  expansion corpus files; each resolved to the current upstream side, retaining
  the 10k-line browser cap and CIP-99 criterion ledger. The lock-aggregation
  code itself merged without conflict in `trace.rs`; the MemoEngine continues
  to call that owner for full-subtree reduction. Next: rerun focused identity
  and the full agent tier on merge commit `1bde604`.

- 2026-08-05T19:03:00Z — Post-merge non-VM tier is green: fmt, examples fmt,
  warning-denied clippy, workspace tests, corpus regeneration/drift, tour
  regeneration/drift plus exact document test, source-size/module-map guard,
  and diff check all synchronously exited 0. Wallos still produces
  `/nix/store/26wbmzxzyks6q0h41sl0zxs3gf4dgj6j-cix-item-wallos`; its ordinary
  warm-hit build only refreshed the tracked output source hash, which was
  restored (the controlled before/after generated-lock receipt remains the
  byte-identity evidence). CIP-109 is using both guest slots for a final retry;
  continue waiting on its concrete selector PID.

- 2026-08-05T19:32:00Z — Merged the subsequent current `origin/main`
  (`ab49af8`), which added only the expand2 track specification and did not
  touch the extraction. A second selector start raced CIP-109's transition
  from its focused retry to a serial matrix; this track immediately interrupted
  only its own build at exit 130, before any guest started. That attempt is not
  a receipt. After CIP-109 released the axis, the supported explicit-base run
  `nix run --max-jobs 2 --cores 2 .#progressive-vm-check -- --base origin/main`
  compared the complete `6d27875+worktree` extraction to `ab49af8`, selected
  all 14 scenarios because `cix-build/src/lib.rs` is cross-cutting, and
  synchronously exited 0 after the bounded matrix in 723.462s. Exact selector
  output is retained at
  `/var/tmp/cip103-memo-receipts/final-progressive-vm.log`. The complete agent
  tier is green; full flake check remains the orchestrator's independent gate.

- 2026-08-05T16:10:00Z — Merging `origin/main` after CIP-93b and CIP-99.
  Semantic resolution retains CIP-99's complete traced-subtree aggregation at
  both top-level FETCH and builder recording sites, while the Workspace owner
  remains responsible for memo validation, snapshot/replay, and every
  `StepChange::Subtree` materialization path. Next: compile and run focused
  lock/workspace tests before the complete agent tier and a new base/current
  byte-identity corpus receipt.

- 2026-08-05T16:35:00Z — Merge resolution and post-merge agent tier are
  green. The isolated `origin/main`/merged-head Wallos receipt value-checked
  equal JSON output (`wallos` =
  `/nix/store/4ia5g2fz571l8hfzzgl2v2p3i2q1pjwj-cix-item-wallos`) and equal
  resulting `Cixfile.lock` SHA-256
  `a0815887a1c5bc2367e294c54416e276e32370d801cf01797e2b2277faf5df9c`.
  Synchronous exit-0 tier receipts: Rust fmt, examples fmt, warning-denied
  all-target clippy, workspace tests, corpus and tour regeneration with zero
  drift, and bounded contract-keyed progressive VM selection. The selector
  compared the merge to the Workspace parent and correctly found no VM product
  contract change, so it selected 0 scenarios and exited 0. The source-size
  and shared-state audits also passed. The isolated 487 MiB receipt directory
  was removed after value capture. Full flake remains the orchestrator gate.

- 2026-08-05T15:57:00Z — After semantic merge of `origin/main` at
  `54c03a5`, the CIP-93b contract-keyed gate selected the complete 14-scenario
  matrix for the accumulated merge diff. Its retained synchronous log at
  `/tmp/cip99-progressive-post-cip93b.log` ends `VM selection: build exit 0;
  total wall-clock 699.699s.` Health also completed within that run. The
  quiet-window receipt set is now clean: workspace exit 0; health SOLO exit
  0; post-merge progressive VM exit 0.

- 2026-08-05T15:05:00Z — Quiet-window health discrimination is green: the
  actual check attr `devenv shell -- nix --max-jobs 1 --cores 1 build -L
  .#checks.x86_64-linux.scenario-health`, retained at
  `/tmp/cip99-health-solo.log`, synchronously exited 0. (A prior `.#scenario-health`
  package-attr probe exited 1 without running a VM and is not a scenario
  receipt.) The earlier progressive health exit 1 versus this solo exit 0
  attributes that failure to VM contention; per the stated conditional, no
  `origin/main` scratch control is required. Merge CIP-93b and rerun the
  contract-keyed progressive selector before closing.

- 2026-08-05T14:40:00Z — Quiet-window part 1 receipt is green: `devenv shell
  -- cargo test --workspace -- --test-threads=1`, retained at
  `/tmp/cip99-workspace-quiet.log`, synchronously exited 0. This clears the
  former tour/user-manager failure; continue to hold all VM discrimination
  until the orchestrator declares the VM axis quiet.

- 2026-08-05T14:30:00Z — Orchestrator has not accepted the two non-green
  receipts as environmental. Stand by while CIP-106 and CIP-93b occupy the
  tour/VM capacity. On an explicit quiet-window ping: (1) rerun the complete
  serialized workspace suite with synchronous retained-log capture; (2) rerun
  the failing health-watchdog VM scenario SOLO with retained-log capture. If
  health still fails alone, run the identical scenario from a scratch checkout
  at `origin/main`, as CIP-105 did, before making any environmental claim.

- 2026-08-05T14:20:00Z — Final agent-tier receipts: fmt, example fmt,
  warning-denied clippy, corpus-browser generation, and corpus drift all exit
  0. The serialized workspace receipt exits 101 only in the pre-existing
  user-manager tour race: the foreign test’s 60-second decoy ends before its
  cleanup after rendering, then poisons the renderer lock. The isolated tour
  generator exits 0 and leaves no `docs/tour` drift; a fresh independent
  renderer run reproduces the host-only `second web run printed its unit`
  assertion. The bounded automatic VM selector (`--max-jobs 2 --cores 2`)
  exits 1 in the unrelated health scenario: under concurrent VM load its
  watchdog kills the service before the journal assertion, followed by the
  runner’s `Invalid BuildResult status` error. Do not call this track green
  until the orchestrator has a clean serialized VM/tour environment.

- 2026-08-05T14:00:00Z — Implemented CIP-99 lock-scale. `ReadDependency`
  now has a recursive `subtree` digest selected only for a bottom-up complete
  stable tree (listed directories plus content observations of every child);
  validation re-walks the same tree and any narrower/volatile observation
  stays per-path. Complete output trees use one replay-root change record;
  wholly removed trees use one absent root. Focused trace tests pass. Clean
  HEAD controls and this branch produced the same current output store paths
  for parse-server, echo-server, and phpMyAdmin. Regenerated lock counts:
  parse-server 197,888 -> 54,915; echo-server 19,622 -> 19,620 (partial
  reads deliberately cannot collapse); phpMyAdmin 15,539 -> 461. Next: full
  agent tier and focused VM receipt.

- 2026-08-05T00:35:00Z — Committed CIP-108 (`Add CIP-108 structural
  guardrails`) after the staged whitespace review. The branch is ready for
  independent orchestrator review and its full flake gate; do not merge from
  this track.

- 2026-08-05T00:30:00Z — Full CIP-108 agent tier completed with synchronous
  numeric exit-0 receipts: `devenv shell -- cargo fmt --all --check`, `devenv
  shell -- cargo run -- fmt --check examples`, `devenv shell -- cargo clippy
  --workspace --all-targets -- -D warnings`, `devenv shell -- cargo test
  --workspace`, `devenv shell -- cargo test -p cix --test tour -- --ignored
  generate_tour`, and `git diff --exit-code -- docs/tour`. The bounded VM
  receipt is `devenv shell -- nix run --max-jobs 2 --cores 2
  .#progressive-vm-check` exit 0; it selected all 14 current scenario
  derivations because the flake source-size check changed. The initial VM
  attempt passed the bounds to the app and exited 2 before running scenarios;
  the corrected command above is the only VM receipt. `git diff --check` is
  clean. Next: stage the scoped changes, verify the staged diff, and commit;
  full `nix flake check -L` remains the orchestrator gate.

- 2026-08-05T00:10:00Z — Implemented CIP-108 against the current module
  layout: standardized exhaustive maps for build, Cixfile, compose, index,
  and run; documented intentional root-map omissions for the two single-module
  roots; and made the source-size check compare declarations to map entries.
  It now reports live/inline-test/total LOC, retains the 2,000 total ceiling,
  and emits extraction diagnostics at 500 inline-test LOC. Added the required
  shared-state inventory command to `AGENTS.md` and rationale comments at the
  five audited sites. Direct `bash scripts/check-source-size.sh` completed
  synchronously with exit 0; its live tree reports only the retained
  `build_chain.rs` grandfather exception and extraction diagnostics for
  compose generation/resolve tests. Next: format, review, then run the
  prescribed gates with captured exit-status receipts.

- 2026-08-05T00:00:00Z — Started `track/cip108-guardrails` (CIP-108).
  Read the accepted decision, the P2 audit evidence, and the current crate
  roots rather than the audit snapshot. The assigned journal is tracked by
  established project convention. Next: make root maps mechanically
  checkable, split source-size reporting into live/test/total, record the
  shared-state audit command, and add the five missing site rationales.

- 2026-08-05T02:10:00Z — Merged `origin/main` stop-disposition merge
  (`e5c75d0`) as `689ae38`; it was conflict-free, and review confirmed its
  STOPSIGNAL/KillSignal and compose `stopTimeout` strata do not overlap the
  CIP-101 scratch owners. Regenerated the tour with the now-exclusive user
  manager: `devenv shell -- cargo test -p cix --test tour -- --ignored
  generate_tour` exited 0, then `git diff --exit-code -- docs/tour` exited 0.
  The earlier observed transient user-manager namespace retries were
  successful D13 fallbacks (journal evidence showed the final retry start);
  this regeneration completed without a repeat assertion failure. Complete
  post-merge synchronous exit-0 receipts: `devenv shell -- cargo fmt --all
  --check`; `devenv shell -- cargo run -- fmt --check examples`; `devenv shell
  -- cargo clippy --workspace --all-targets -- -D warnings`; `devenv shell --
  cargo test --workspace`; and `devenv shell -- nix run
  .#progressive-vm-check` (all 14 current derivations selected, including the
  new `scenario-stopdispo`). The full flake matrix remains the orchestrator's
  independent pre-merge gate by project policy.

- 2026-08-05T01:15:00Z — Required track tier results: synchronous exit-0
  receipts for `devenv shell -- cargo fmt --all --check`, `devenv shell --
  cargo clippy --workspace --all-targets -- -D warnings`, `devenv shell --
  cargo test --workspace`, `devenv shell -- cargo run -- fmt --check
  examples`, and `devenv shell -- nix run .#progressive-vm-check` (the
  selector reported all 13 scenario derivations changed and completed exit
  0). `cix build --help` synchronously exposes `--keep-scratch`. Tour drift
  matching (`cargo test -p cix --test tour tour_matches_committed_document
  -- --exact`) exited 0. Tour regeneration itself was attempted twice and
  both attempts hit the existing host-dependent `second web run printed its
  unit` assertion after deleting its in-progress generated files; each time
  the exact generator diff was reversed and `git diff --exit-code --
  docs/tour` returned 0, so this track retains no generated-tour change. Full
  flake matrix remains reserved for the orchestrator by project policy.

- 2026-08-05T00:30:00Z — Implemented the CIP-101 scratch owner and converted
  every production large-tree allocation (`cix-build-cold`, `cix-build-view`,
  `cix-fetch-{probe,work}`, `cix-import-{loaders,union}`, `cix-read-trace`,
  `cix-step-delta`). Default root is `/var/tmp` because it is disk-backed and
  tmpfiles-aged; TMPDIR remains the explicit override. `ScratchDir` restores
  write bits before recursive removal, retains and prints paths with
  `cix build --keep-scratch`, and a signal listener cleans live owners on
  INT/TERM/HUP before restoring the default signal action. `cix` startup
  sweeps own-UID matching scratch prefixes older than one day. The reachable
  comparison harness now defaults its large benchmark root to `/var/tmp` and
  retains its existing EXIT cleanup trap. Synchronous focused receipts:
  `devenv shell -- cargo check -p cix-cixfile -p cix-build -p cix`; `devenv
  shell -- cargo fmt --all --check`; and `devenv shell -- cargo test -p cix
  --test fetch_probe_cleanup -- --nocapture` (success, ordinary failure, and
  SIGTERM each leave no recognized build scratch). A direct startup-sweep
  receipt created a two-day-old `cix-build-cold-*` under an isolated TMPDIR,
  ran `target/debug/cix inspect does-not-exist`, observed its expected nonzero
  inspection result, and synchronously confirmed the stale directory was
  removed.

- 2026-08-05T00:00:00Z — Started track/cip101 (CIP-101). Read the accepted
  decision and mapped production scratch owners: cold builder workspaces,
  FETCH work/probe/audit snapshots, step deltas, consumed-path views, read
  traces, IMPORT unions, and FHS loader surfaces. The implementation will use
  `/var/tmp/cix-*` by default (honouring TMPDIR), RAII ownership plus an
  explicit `--keep-scratch` debugging mode, and startup sweeping of own
  day-old directories. Test/tooling scratch reachable from the workspace will
  be audited alongside the production path. `crates/cix-build/LOG.md` is
  already tracked by the repository despite the general ignored-log guidance;
  preserve that established project convention for this assigned log.

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

## 2026-08-05 — track/cip103-leg1

- Started CIP-103 leg 1 at `8a081e6`. Scope is strictly deletion of the
  commented FETCH extraction residue, deletion of the audit-proved unreachable
  `codegen.rs` test module, and a byte-identical move of build-chain tests to a
  sibling test file. The crate-root module map will become exhaustive. The
  worktree started clean; no behavior, corpus, lock, or fixture edits are in
  scope. Next: apply the mechanical move, inspect its diff, then run the agent
  tier with synchronous captured exits.

- Applied the pure move/deletion. `build_chain.rs` is now 3,348 lines (the
  254-line FETCH residue and 772-line inline test module are gone); its 768
  test-body lines now live in `build_chain_tests.rs` through a child test
  module, preserving private access and every test assertion. `codegen.rs`
  lost the audit's 148-line `#[cfg(all(test, any()))]` module. The module map
  now names all ten direct production modules. Synchronous focused receipt:
  `devenv shell -- cargo test -p cix-build` exited 0 (38 unit tests, including
  all moved `build_chain::tests`). Next: format, inspect the move proof, make
  granular commits, then run the complete agent tier.

- The source change is committed in two compile-valid, narrow commits:
  `d6023f0 build: remove obsolete fetch and codegen test residue` (251-line
  FETCH copy plus 149-line disabled codegen tests) and `fc462f1 test(build):
  move build chain tests beside conductor` (test-file move plus exhaustive
  module map). `cargo fmt --all` made only layout changes in the moved file;
  the whitespace-normalized old inline test body and new file compare equal.
  Next: wait for any shared-manager work, then run the synchronous full agent
  gate and focused progressive VM selection from this committed source.

- Gate coordination: `systemctl --user list-jobs` found another suite's active
  `cix-private-devices-probe-2341500-1785922635090529821.service` start job.
  Per the shared-manager rule, waiting before starting this track's tour/VM
  gate; no manager work has been launched by this track yet.

- Gate receipts on committed `fc462f1`: after the foreign manager job cleared,
  one foreground capture-as-epilogue run wrote `.gate-exit` = `0` after
  `cargo fmt --all --check`, `cargo run -- fmt --check examples`, warning-denied
  workspace clippy, `cargo test --workspace`, tour regeneration, a clean
  `git diff --exit-code -- docs/tour`, and the full tour drift test. The
  progressive selector compared `fc462f1` to `d6023f0`, declared all fourteen
  scenario derivations changed, and selected every scenario (no hand-picking).
  Its first foreground stream lost its terminal status after completing; the
  exact selector was rerun with an epilogue capture at
  `/tmp/composix-cip103-leg1-vm-exit`, which contains numeric exit `0`.
  No corpus, lock, fixture, or generated-document change remains. Final state:
  source is committed; this required uncommitted LOG entry is the only
  worktree change; do not merge.

- 2026-08-05T20:46:00Z — Completed `track/cip101-livelock`. `ScratchDir`
  now holds an advisory `flock` in a `/var/tmp/.cix-scratch-locks/` (or
  `$TMPDIR`) sidecar, so the startup sweep skips a live aged tree without
  making the marker part of a FETCH/build output. It still reaps an aged,
  unlocked tree and removes its stale sidecar. The real-CLI regression ages a
  running build's actual scratch, invokes a concurrent cix startup sweep,
  proves the tree survives, then SIGTERMs it and proves cleanup; the companion
  test proves an unlocked dead tree plus sidecar is removed. CIP-101's
  changelog records the amendment. Final synchronous, value-checked receipts:
  focused `fetch_probe_cleanup` 3/3; warning-denied workspace clippy; full
  workspace tests (including both tour checks); tour regeneration plus zero
  drift; source-size and shared-state audits; and final bounded two-job
  progressive selector, which selected all 14 scenarios and exited 0 in
  688.590s. Captures are retained at `/var/tmp/cip101-livelock-receipts/`.

- 2026-08-06T01:55:49Z — Stopped `track/cip107-pinleg` before deletion, as
  required. A repo-wide structural scan finds 21 `fetches.*.narHash` entries
  across 14 committed locks (the current tree is broader than the spec's
  stale 18/9 count); Directus is one of them. Its freshly restored-context
  `--update-lock build` attempt exited 1 only after completing FETCH and
  installation, because the pinned metadata cache lacks
  `@directus/tsconfig@4.0.0` for offline deploy. The partial generated lock
  was reviewed and restored byte-identically; its SHA-256 remains
  `e40ee98df87de1bbf9a65b261c79f56987e0eb4b70ab1a3ece6a106906ea0d66`.
  Directus GAPS/receipt and its generated browser page record the wall. Final
  synchronous receipts: corpus browser regeneration, Rust fmt, examples fmt,
  warning-denied clippy, workspace tests (retry exit 0 after a foreign tour
  renderer made the first run fail at the listener), tour regeneration/drift,
  and progressive VM selector (0 scenarios, exit 0). No FetchPin source or
  lock deletion was made; receipts are `/var/tmp/cip107-pinleg-receipts/`.
