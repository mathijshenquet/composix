# litdoc work log

- 2026-08-06T10:20:00Z — Finished `track/expand-postgres-registry`. The two
  fetched contexts, faithful Cixfiles, dissolved twins, locks, checks, receipts,
  GAPS files, and generated corpus pages are present. Synchronous receipts:
  `cargo test -p cix --test corpus` exited 0 (7 passed, 1 ignored), browser
  generation exited 0, faithful and dissolved PostgreSQL builds plus both cold
  builds exited 0, and the Registry dissolved normal/cold builds plus faithful
  runtime check exited 0 with exact `GET /v2/` value `{}`. The faithful Registry
  cold build is a recorded read-set wall at `FETCH go mod download`; the
  PostgreSQL runtime is a recorded wall before `pg_isready` because the item
  cannot provide the package `lib` path and the state-role setup cannot chmod
  `/var/run/postgresql`. No Rust source changed, so workspace Rust/VM gates were
  outside this corpus-only scope. Next: hand off without committing or merging.

  FRICTION: Cix FETCH read-set replay distinguishes a warm Go module cache from
  a cold absent cache, so the faithful Registry case cannot honestly claim cold
  compatibility. PostgreSQL's split Nix `lib` output and arbitrary-path role
  realization remain product walls; both are preserved in receipts/GAPS rather
  than hidden behind a green probe claim.

- 2026-08-06T00:00:00Z — Started `track/expand-postgres-registry` from a
  clean worktree after reading the supplied spec, `docs/corpus.md`, the
  candidate rows, and recent ntfy/filebrowser anatomy. This is corpus-only:
  add faithful postgres and distribution registry cases, prestage all vendor
  inputs through pinned FETCHes, and preserve synchronous receipts and honest
  walls. Next: pin the upstream revisions, fetch contexts, and inspect the
  exact Dockerfiles before authoring translations.

  FRICTION: The requested `crates/cix/LOG.md` is a tracked append-only journal
  shared by prior tracks; no new language form has been reached for yet.

- 2026-08-06T00:20:00Z — The first registry receipt could not start because
  this clean worktree had no `target/debug/cix`; the prerequisite
  `devenv shell -- cargo build -p cix` exited 0 synchronously in 12.70s.
  No case build was counted from the missing-binary attempt. Next: rerun the
  registry build with the freshly built binary, then exercise postgres.

  FRICTION: The documented corpus command assumes an existing debug binary;
  a clean worktree needs the explicit package build first. `devenv shell` was
  active and healthy once invoked.

- 2026-08-06T00:35:00Z — The registry Cixfile parsed and its first corrected
  build reached the large Go dependency compile after moving `GOPROXY=off`
  from the network FETCH to the offline build. The initial ordering failure
  was a synchronous nonzero receipt and was fixed before counting the build.
  Added the two corpus rows and updated the outside-closed-root count to 32
  cases / 15 outside the roster. Next: collect the registry build exit and
  then build the PostgreSQL case.

  FRICTION: `--update-lock` has an optional selector argument; writing
  `--update-lock corpus/...#registry` consumed the directory as the selector
  and made cix read the repository-root Cixfile. The working form is
  `--update-lock=build corpus/...#registry`. Go's module cache must remain
  network-enabled during FETCH and only become `GOPROXY=off` in RUN.

- 2026-08-06T01:00:00Z — Faithful and dissolved PostgreSQL builds exit 0.
  Its synchronous runtime attempt is a wall: the upstream entrypoint reaches
  the existing state directory, but the service cannot chmod
  `/var/run/postgresql`, and PostgreSQL then reports its package `lib` path is
  unavailable in the isolated item. This is recorded with the known
  arbitrary-path state-role defect rather than reclassified as a green probe.
  Registry initially failed its upstream debug listener on undeclared port
  5001; adding the declared second port preserved the config, and the rerun
  passed `GET /v2/` with exact value `{}` (one transient readiness refusal
  preceded the successful probe). Next: run cold build receipts, dissolved
  registry build, and write both full receipts.

  FRICTION: A package with a separate Nix `lib` output is not made available
  merely by `IMPORT`; Cix's runtime import surface is bin/etc/share, while
  PostgreSQL discovers its compiled package lib path. Direct `/lib` COPY is a
  reserved runtime path, so this remains an honest package/sandbox wall.

- 2026-08-06T00:00:00Z — Started `track/expand-ntfy-filebrowser` from the
  supplied spec. This is corpus-only work: add faithful ntfy and filebrowser
  migration cases, with their upstream release artifacts prestaged through
  pinned `FETCH` inputs rather than letting an installer use the network during
  a build. Read the active corpus contract, fetch/staging conventions, and
  recent green case anatomy. Next: identify the exact upstream Dockerfiles,
  source revisions, release artifacts, and probe contracts; record walls
  rather than fabricate green evidence.

- 2026-08-06T00:00:00Z — Started track/ch7gcroot from the supplied spec.
  Scope is the Chapter 7 CI-only gcroot cleanup drift. Read the repository
  journal, D13 dev-mode context, and the scenario before changing anything.
  First investigate the failing root's ownership/lifecycle in the GitHub
  runner's old user-manager environment; do not normalize the symptom unless
  the documented cleanup is sound and host-environmental. Next: trace the
  run/cleanup implementation and reproduce the chapter command sequence.

- 2026-08-06T00:15:00Z — Cause confirmed at the product lifecycle layer:
  the CI user manager retains `PrivateUsers=yes`, making the injected
  unprefixed `ExecStopPost=rm` unable to unlink the host runtime-dir root;
  beast's D13 fallback drops it. Changed the cleanup command to use systemd's
  `+` prefix, which is still the user manager's UID but escapes that sandbox.
  A focused construction test, `cargo fmt --all --check`, and an actual
  `systemd-run --user` ExecStopPost probe each returned synchronous exit 0.
  Next: regenerate Chapter 7 and run the track gates.

- 2026-08-06T00:30:00Z — Final track receipts are value-checked: fmt,
  examples fmt, warning-denied workspace/all-target Clippy, serial workspace
  tests, explicit tour generation plus no `docs/tour` drift, and regular tour
  drift/determinism tests each exited 0. The derived progressive VM matrix
  selected all 14 runtime-core scenarios for the lifecycle change and exited 0
  after 624.335s. An initial terminal bridge detached overlapping processes;
  those were terminated and discarded, then the serial sessions above captured
  concrete exit values. Next: clean final state checks; do not commit.

- 2026-08-04T16:47:31Z — Semantic jsonpretty merge resolution is fully green
  with synchronous exit-0 receipts. Explicit ignored regeneration ran twice;
  the second run against staged pages left `git diff --exit-code -- docs/tour`
  clean. The exact `generated_tour_is_deterministic` test then passed three
  consecutive foreground runs in 58.14 s, 58.26 s, and 58.62 s. The bounded
  agent tier passed `cargo fmt --all --check`, `cix fmt --check examples`,
  warning-denied workspace clippy, and serialized full workspace tests. Both
  unstaged and staged `git diff --check` passed, the recursive START canon scan
  found no interpolation, no merge conflicts remain, and only cix-prefixed
  stale failed user-unit records were reset before verifying none remained.
  The df guard started with 37 GiB root space and 181905 `/tmp` inodes free and
  ended with 42 GiB and 181580 free. No focused VM scenario applies to this
  tour-only conflict resolution; the orchestrator retains the independent full
  flake-matrix gate and final prose pass. Next: commit the merge and stop.

- 2026-08-04T16:38:47Z — Began semantic resolution of the in-progress
  `origin/main` merge after jsonpretty landed. The tour4 chapter structure and
  prose remain authoritative. Reapplied the landed JSON intent at the harness
  boundary: hand-made manifests use `jq -n`, pretty-spacing assertions follow
  the new output, multiline build maps are parsed as complete JSON values, and
  the built listener manifest is displayed from its pretty on-disk file. Kept
  Chapter 1's selected `jq` view because it teaches the runtime-contract fields
  without dumping the entire first manifest. Both append-only journal histories
  are retained; generated pages and fixture locks will be regenerated rather
  than hand-merged. Next: regenerate, run three determinism receipts and the
  complete bounded agent gate, then commit the merge.

- 2026-08-04T16:29:14Z — Final bounded agent tier is green with synchronous
  exit-0 receipts: `devenv shell -- cargo fmt --all --check`, `devenv shell --
  cargo run -p cix -- fmt --check examples`, `devenv shell -- cargo clippy
  --workspace --all-targets -- -D warnings`, and `devenv shell -- cargo test
  --workspace -- --test-threads=1`. Explicit ignored tour regeneration exited
  0 and `git diff --exit-code -- docs/tour` proved no drift. The exact
  `generated_tour_is_deterministic` test passed three consecutive foreground
  runs (50.59 s, 51.41 s, 53.06 s). Recursive `grep` found no `START` line
  containing `${`, `git diff --check` passed, and no active user `cix-*` unit
  remained. The df guard started with 40 GiB root space and 175454 `/tmp`
  inodes free and ended with 36 GiB and 181923 free. No focused VM scenario is
  applicable because this track changes only the executed tour harness and
  generated guide; the orchestrator retains the full flake matrix gate. Track
  is complete and ready for the requested orchestrator prose pass.

- 2026-08-04T22:10:00Z — Chapter 7 and the cross-chapter cold-reader audit
  regenerate successfully. Watch now captures its PID, executes SIGINT cleanup,
  states default workspace/memo behavior, and gives the exact privileged
  compose-watch boundary. A pinned two-platform flake executes a real
  `nix develop` tool receipt. Faithful and dissolved runnable APPs now derive
  from the same displayed five-line Dockerfile, both print `Hello, world!`, and
  their independent nixpkgs locks and `--file` resolution/update commands are
  explicit. The local migration bridge defines source context, receipts, and
  gaps. The final bullet audit also added index prerequisites, Chapter 1 role
  backing/service-kind context, correct Cargo.lock highlighting, explicit
  compose readiness/baseline/shared-role rules, and a real Chapter 5 watchdog
  restart plus debug PATH receipt. Next: commit, then run df guard and the full
  bounded agent gate including three deterministic renders and canon scan.

- 2026-08-04T21:25:00Z — Chapter 6 explicit regeneration exits 0. Named
  listener commands now capture the real item and unit. New executable Python
  fixtures create and connect to a real AF_UNIX socket, while compose projects
  the producer directory at the consumer's explicit `/run/upstream` path. The
  document shows track and pin policies, shared backing ownership/lifecycle,
  honest lock-writing and root-activation boundaries, the root profile and
  generation-list command, precise rollback scope, unary flag mappings, pod
  and journal JSON locations, and exact log fields. Producer v1/v2 item names
  survive store-hash normalization, and assertions prove the diff lines are
  visibly different. Next: commit Chapter 6 and rebuild Chapter 7 around a
  real five-line Dockerfile plus fully executable faithful/dissolved twins.

- 2026-08-04T20:45:00Z — Chapter 5 explicit regeneration exits 0 with a real
  rootless web lifecycle: readiness succeeds, the generated manager properties
  are inspected, state is written and read after restart, and the tour restores
  any pre-existing user-state value afterward. The chapter now validates the
  implemented compose secret source and gives the exact privileged supply
  commands, defines the debug delimiter and observability columns/selectors,
  scopes closed-root as an opt-in audit, and creates, inspects, removes, and
  unloads a scheduled APP timer pair. Host-specific manager warnings and live
  accounting remain declared normalizations. Next: commit Chapter 5, then make
  Chapter 6's Unix producer/consumer edge and compose update receipts concrete.

- 2026-08-04T20:05:00Z — Chapter 4 explicit regeneration passes. The hand-made
  item is now honestly labelled taggable-but-not-runnable, `nix store add`, GC
  roots, tag argument order, platform metadata, and the inspect `artifact`
  label are explained in plain terms. V2 and v3 directories are each created,
  shown, added, captured, and only then tagged. Families are scoped to their
  actual namespace/list behavior. Distribution now establishes separate
  publisher/consumer indexes, gives the qualified-ref grammar before use,
  defines closure/download behavior, and distinguishes the unsigned loopback
  receipt from TLS + signed-cache + trusted-public-key production setup. Next:
  commit Chapter 4 and replace Chapter 5's parse-only web story with a live
  readiness/state/restart lifecycle.

- 2026-08-04T19:40:00Z — Chapter 3 regeneration passes with the original real
  warm/cold/FHS/proj1 builds intact. The prose now labels FETCH grammar and
  trust modes, defines memo/build-view/lock/dev-env at first use, states the
  exact workspace and snapshot-cache locations, and gives the complete
  update-lock→cold transition including fresh-machine and GC refusal behavior.
  The local fixture server is explicit. Proj1 now shows Cargo manifests,
  lock, source tree, resulting refs, captured path variables, visibly changed
  worker output, and a byte-identical API receipt. Lock files render as JSON.
  Exact explicit tour generation exited 0; next: commit Chapter 3, then build
  all three naming-demo items before rewriting Chapter 4 distribution prose.

- 2026-08-04T19:10:00Z — Chapters 1 and 2 execute cleanly in explicit tour
  regeneration. Chapter 1 now checks Nix/flakes/user-systemd, teaches captured
  item paths, runs the nginx item through the real user manager, probes HTTP,
  and stops the printed unit; a checked-in launcher uses native projections in
  production and `CIX_APP` only on degraded user managers. Host-specific
  degradation blocks normalize to the declared single marker. Chapter 2 now
  defines every language noun, puts five Dockerfile/Cixfile lines side by side,
  proves a real store symlink target and CIP-91 materialization, and supplies
  the missing runtime/operator grammar. Chapter 1 plus the verbatim
  preamble/index committed as `2b80163`; next commit is the independently
  generated Chapter 2 source/page.

- 2026-08-04T18:30:00Z — Started `track/tour4` from clean `310022e` after
  reading the track spec, both cold-reader `CONFUSIONS.md` reports in full,
  the repository/design journals, the existing seven-chapter generator, and
  the runtime/compose/build implementation seams needed for honest commands.
  Every report bullet is acceptance scope. Direct `cix run` has no credential
  source flag: implemented runtime credentials are compose-level
  `secrets` consumed by root-owned `cix run --compose`/`cix up`; compose
  activation likewise requires root. The tour will execute the portable
  rootless service, persistence, timer, listener, and Unix-probe receipts,
  while labeling exact credential/activation commands as non-executed instead
  of fabricating privilege. Planned commits are index/infrastructure followed
  by one commit per chapter; final gate is the standard agent tier, three
  consecutive deterministic renders, the `${` START canon scan, and df guard.
- 2026-08-04T16:14:29Z — Committed the resolved `origin/main` merge as
  `f298152` (`Merge origin/main into track/jsonpretty`). Both journal histories
  are retained; main's `show_file` tour structure is authoritative and this
  track's pretty JSON fixture/generated-manifest displays are adapted onto it.
  Regenerated tour pages have zero drift, all merge receipts above are
  synchronous, and the worktree was clean immediately after the merge. The
  orchestrator retains the independent full flake-matrix gate.

- 2026-08-04T16:13:32Z — Merge verification is green with synchronous
  receipts: `cargo fmt --all --check`, `cix fmt --check examples`,
  warning-denied workspace clippy, and serialized workspace tests. The first
  explicit deterministic run encountered stale failed listener-demo units in
  the user manager; after resetting those exact inactive unit states, three
  fresh foreground runs passed consecutively in 35.27s, 32.90s, and 34.02s.
  The resolved tour keeps main's typed `show_file` output and displays the
  pretty hand-assembled and generated manifests. Next: stage regenerated
  pages, prove zero regeneration drift, review the merge, and commit it.

- 2026-08-04T16:06:41Z — Began resolving the in-progress `origin/main`
  merge. Kept both append-only journal histories. The semantic tour merge
  retains main's `Doc::show_file` harness and its canonical Cixfile-built
  listener, while reapplying this track's pretty JSON displays: the
  hand-assembled manifest is generated with `jq -n`, raw-item fixture bytes
  are indented, and the generated listener manifest is shown through
  `show_file`. Generated tour pages are intentionally deferred to regeneration
  after code resolution. Next: resolve/stage all generated page conflicts,
  regenerate, then run the complete agent tier and three deterministic receipts.

- 2026-08-04T16:20:00Z — Final `track/tourpolish` agent tier is green with
  synchronous exit-0 receipts: `devenv shell -- cargo fmt --all --check`;
  `devenv shell -- cargo run -p cix -- fmt --check examples`; `devenv shell
  -- cargo clippy --workspace --all-targets -- -D warnings`; and `devenv shell
  -- cargo test --workspace -- --test-threads=1`. Explicit `devenv shell --
  cargo test -p cix --test tour -- --ignored generate_tour` followed by `git
  diff --exit-code -- docs/tour` proved zero regeneration drift. One foreground
  `bash -c` loop with `set -e` then ran the exact
  `generated_tour_is_deterministic` test three consecutive times; all passed
  synchronously in 36.50s, 36.99s, and 35.13s. Final audits find a clean
  worktree before this receipt entry, zero generated `$ cat` prompts, no raw
  listener-manifest fixture, and no abstract Chapter 2 service/path remnants.
  This track changes only the tour harness and generated prose, so no focused
  VM scenario or Docker/corpus ledger row is affected; the orchestrator retains
  the full flake-matrix gate. Next: commit this receipt and hand off the two
  logical implementation commits for independent verification.

- 2026-08-04T15:55:00Z — Chapter polish is complete. Chapter 6 now writes a
  canonical `listener-fixture/Cixfile`, imports coreutils + Python, copies the
  executable checked-in `listenfds.py` probe, starts it by the imported bare
  name, declares `LISTENER http`, builds it with real `cix build`, asserts the
  generated manifest's listener field, and runs/probes that built item. The
  env shebang uses the runtime skeleton's documented `/usr/bin/env` route, so
  the copied script remains compatible with the rootless no-mount fallback.
  Chapter 2's service and app-native paths are now coherently named
  `guide-site`; its prose explicitly identifies `STATEDIR /opt/nginx/state` as
  a deliberate CIP-91 linked-branch materialization demonstration. Focused
  synchronous receipts pass: explicit tour generation (including the live
  built listener), fmt, committed-document match, and `git diff --check`.
  Next: commit this unit, then run the full agent tier and final three-run
  determinism receipt.

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

- 2026-08-04T15:50:16Z — Committed the amended all-JSON policy as `fb64abf`
  (`cix: pretty-print all JSON writes`): cix item manifests are formatted by
  the generated Nix build, build-state/cache and compose secret writes use
  pretty serializers, and index JSON API responses are indented. Existing
  pretty lock/compose artifact writers remain unchanged; no corpus locks were
  rewritten. The tour is committed with plain `cat` manifest receipts and all
  final agent-tier and three-run determinism receipts recorded above. The
  required post-commit journal receipt is `d5010b3`; the worktree was clean
  afterward. The orchestrator owns the independent full flake-matrix gate.

- 2026-08-04T15:49:50Z — Staged regenerated tour chapters 1, 4, and 6, then
  reran the ignored generator synchronously (exit 0) with zero unstaged tour
  drift. Final serializer audit finds the only remaining non-pretty
  `serde_json` calls construct internal hashes/cache keys, never written JSON.
  `git diff --check` is clean and neither staged nor unstaged paths include
  `corpus/`. Next: stage the scoped implementation, inspect the staged diff,
  and commit the all-JSON formatting amendment with this journal.

- 2026-08-04T15:48:46Z — The amended complete agent tier is green with
  synchronous exit-0 receipts: `cargo fmt --all --check`, `cix fmt --check
  examples`, warning-denied workspace clippy, and serialized workspace tests.
  The initially failing full suite exposed the expected compact index JSON
  golden; both HTTP list-body fixtures now assert the indented response and
  its targeted integration test passes. The amended tour determinism receipt
  passed three consecutive foreground runs (31.04s, 32.84s, 33.12s). Next:
  stage the generated tour, repeat regeneration against the staged pages for
  drift, review the all-JSON writer audit, and commit. The orchestrator still
  owns the one-per-track full flake-matrix gate.

- 2026-08-04T15:45:00Z — Focused amended-policy receipts are green:
  `cargo fmt --all --check`, the real-Nix manifest fixture
  `real_nix_build_assembles_files_links_and_spec`, and ignored tour
  regeneration all exited 0 synchronously. The real-Nix test now reads the
  assembled `cix-manifest.json` itself and requires two-space indentation plus
  its terminating newline. Regenerated chapters 1, 4, and 6 demonstrate those
  pretty on-disk manifests with plain `cat`; the item and compose lock NARs
  changed as expected from the new item bytes. No `corpus/` path is modified.
  Next: repeat the three deterministic tour receipts and the complete standard
  agent tier, then stage/recheck regeneration drift and commit the amendment.

- 2026-08-04T15:40:00Z — Mathijs superseded the prior stdout-only boundary:
  every JSON cix writes is now in scope, including item manifests, locks,
  compose artifacts/state, index API bodies, and local build caches. Alpha
  permits the resulting item hashes and memo cache identities to roll. The
  `corpus/` fence remains explicit: do not rewrite its checked-in locks; cases
  will roll forward when their owning tracks rebuild them. Audit found all
  public locks and compose artifacts already pretty, while the generated Nix
  manifest, builder workspace/cache records, compose secret state, and index
  HTTP response were compact. Updated those writes and restored plain `cat`
  manifest receipts in the tour, with its hand-assembled fixture now pretty as
  well. Next: format and focused real-Nix/tour receipts, regenerate goldens,
  then rerun the complete agent tier.

- 2026-08-04T15:34:33Z — Committed the complete JSON stdout formatting track
  as `8dba887` (`cix: pretty-print JSON stdout`), including this journal's
  prior receipts. `git status --short --branch` was clean immediately after.
  The code changes only CLI stdout rendering and its parser-facing tour
  harness; no manifest, lock, state, hash, or HTTP API serialization changed.
  The track is ready for the orchestrator's independent full flake-matrix
  gate.

- 2026-08-04T15:34:03Z — Final standard agent tier is green with synchronous
  exit-0 receipts: `devenv shell -- cargo fmt --all --check`; `devenv shell --
  cargo run -p cix -- fmt --check examples`; `devenv shell -- cargo clippy
  --workspace --all-targets -- -D warnings`; and `devenv shell -- cargo test
  --workspace -- --test-threads=1`. The exact
  `generated_tour_is_deterministic` check passed three consecutive foreground
  runs in 37.57s, 38.99s, and 44.17s. After staging generated pages, ignored
  tour regeneration exited 0 and `git diff --exit-code -- docs/tour` was
  clean. This output-only track has no focused VM scenario; the full flake
  matrix remains the orchestrator's independent pre-merge gate. Next: stage
  the scoped feature (including this required journal), review, and commit.

- 2026-08-04T15:30:00Z — Focused formatting and output receipts are green:
  the cix-cixfile pretty-render unit test and ignored tour regeneration both
  exited 0 synchronously. The tour harness now parses the complete JSON value
  before trailing build diagnostics, so its path assertions remain valid with
  multiline stdout. Regeneration changed every build-map golden to indented
  JSON and replaced all raw manifest `cat` displays with `cix inspect`; the
  remaining manifest `jq` view is already indented. An explicit serializer
  audit leaves all non-stdout JSON (manifests, locks, state, hashes, and index
  HTTP responses) on their prior code paths. Next: three consecutive tour
  determinism receipts, committed-tour drift check, then standard agent tier.

- 2026-08-04T15:23:49Z — Started `track/jsonpretty` from the current main
  head. Scope is only CLI JSON stdout formatting, its output tests, generated
  tour receipts, and docs; artifact and lock serialization bytes remain fenced.
  Audit found `cix inspect` and `cix ps --json` already use
  `to_string_pretty`; the only compact cix stdout JSON was the Cixfile build
  member map (including `--stats`). Switched those three rendering branches to
  pretty serialization with a focused indentation regression. Reworked the
  tour's raw manifest `cat` receipts to `cix inspect`, while leaving manifest
  creation/storage untouched. Updated build-output documentation to describe
  deterministic, pipe-safe indentation. Next: format, focused tests, regenerate
  the tour, inspect all generated JSON and then run the standard agent tier.

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
- 2026-08-05T00:00:00Z — Started CIP-106 doc-harness thinning: read accepted CIP-106 and audit P1. Will preserve generated tour/browser bytes while extracting shared generated-file atomic write/drift support, splitting tour/corpus tooling, consolidating integration helpers and duplicate goldens. Next: inventory current harnesses and establish exact seams.
- 2026-08-05T00:20:00Z — Added workspace crate cix-test-support: GeneratedFile, shared drift comparison, complete sibling staging plus Linux atomic directory exchange, and shared store/program/command/wait helpers. Tour chapters are now seven scenario modules; harness retains lifecycle/cleanup, command recording, normalization and page assembly. Corpus is divided into discovery/ledger parsing, highlighting, and templates modules. Six byte-identical user health fixtures were removed; both modes now assert the one common fixture. Next: compile and repair seams, then regenerate once and prove generated byte identity.
- 2026-08-05T01:05:00Z — CIP-106 implementation complete. Synchronous exit-0 receipts: atomic tour regeneration followed by zero `docs/tour` diff; atomic corpus regeneration followed by zero `docs/corpus` diff; fmt; examples fmt; warning-denied clippy; cix-test-support (2/2); corpus harness (7/7); and the ordinary single-render `tour_matches_committed_document` receipt (26.33s) with zero generated drift. The serialized workspace suite was rerun after a shared-manager port collision; it completed after the new cross-process tour lock serialized host receipts. The progressive VM selector completed its bounded 14-scenario selection (`--max-jobs 2 --cores 2`). Host cleanup left no active cix units. Next: review and commit implementation only; leave this LOG uncommitted.
- 2026-08-05T01:06:00Z — Receipt correction: the long workspace and progressive-VM child processes finished after the terminal bridge stopped streaming their parent status; do not count either as an exit-0 receipt. The only gate results recorded as green above are commands whose exit-0 result was delivered synchronously. Re-run those two commands foreground before claiming the complete agent tier.
- 2026-08-05T01:20:00Z — Final captured gate receipts: `devenv shell -- cargo test --workspace -- --test-threads=1` exited 0 (tour: 5 passed, 1 ignored; all workspace suites passed); `devenv shell -- nix --max-jobs 2 --cores 2 run .#progressive-vm-check` exited 0 after its selected 14-scenario bounded VM matrix. Final checks: docs/tour and docs/corpus remain byte-identical; no active cix user units. Next: granular commits, leaving this required LOG uncommitted.

- 2026-08-05T11:07:45Z — Started CIP-93 leg 2 from clean `9b377e2` after
  reading the accepted CIP, leg-1 selector, scenario inventory/shared harness,
  flake wiring, and current Rust crate/module seams. Chose ordered, explicit
  scenario contract surfaces: every changed product path maps to a surface,
  is deliberately outside the VM tier, or conservatively selects all; an
  unclassified/new product path also selects all. Scenario and shared-harness
  edits remain direct keys. This bounds the human risk of a wrong contract
  declaration while preventing silent gaps from new files; the orchestrator's
  full matrix remains the backstop. Crate splitting cannot refine the current
  key because every scenario consumes the same linked binary, and dynamic
  runtime read-sets cannot observe which Rust semantics were exercised. Next:
  implement the contract manifest/classifier behind the existing entry point,
  validate exhaustive source classification, and measure historical diffs.

- 2026-08-05T12:33:00Z — Implemented the leg-2 selector as
  `nix/scenario-contracts.json` plus its validating classifier, behind the
  existing progressive VM entry point. Commit `25cd6f3` preserves leg 1 as
  `--selector old`, adds historical `--target` and forced `--rebuild` modes,
  and keeps `--full`. Synchronous classifier assertions, Nix parsing/formatting,
  and `nix build --no-link .#packages.x86_64-linux.progressive-vm-check` all
  exited 0. Historical dry selection receipts were: docs-only
  `99b45fb..e436bef`, old/new 0/14 (24.402s/13.608s); build subsystem
  `aa40ffd..d6023f0`, old 14/14 versus new 0/14 (26.811s/14.546s selection);
  cross-cutting runtime `aa40ffd..a87caa4`, old/new 14/14
  (29.222s/13.575s selection).

- 2026-08-05T12:34:00Z — Completed synchronous historical build measurements.
  After pre-warming the historical outputs, exclusive forced runs with exactly
  two VM guests exited 0: build-subsystem old 14/14 in 634.809s, with its
  new-selector zero-VM run exiting 0 in 11.388s; cross-cutting old 14/14 in
  631.354s and new 14/14 in 622.024s. A first `--rebuild` attempted before its
  outputs existed exited 1, and a cross-cutting attempt overlapped by another
  worktree's full matrix exited 1; both were explicitly discarded, then
  repeated exclusively. Next: commit the amendment/measurements, run both
  selectors on this track's own diff, then execute the complete agent gate.

- 2026-08-05T13:01:55Z — Final track receipts are synchronous and green. On
  the committed track diff against `9b377e2`, leg 1 selected 14/14 and exited
  0 in 656.030s; leg 2 classified every changed path, selected 14/14 because
  its contract manifest is cross-cutting, and an independent `--rebuild`
  exited 0 in 617.644s. Both stayed exclusive with at most two guests. The
  complete agent gate then captured these per-command exits: `cargo fmt
  --all --check` 0; `cargo run -p cix -- fmt --check examples` 0;
  `cargo clippy --workspace --all-targets -- -D warnings` 0; serial `cargo
  test --workspace` 0; explicit tour generation 0; post-generation tour tests
  0; `git diff --exit-code -- docs/tour` 0; and `git diff --check` 0. The gate's
  aggregate `.gate-exit` was 0. No tour drift remained. Implementation and
  measured CIP amendment are committed as `25cd6f3` and `eee46f0`; next:
  commit this receipt-only log entry and leave the branch clean for independent
  orchestrator verification.

- 2026-08-05T13:10:50Z — Merged current `origin/main` (`d0b1f84`, CIP-107
  prune plus CIP-108 guardrails) as `6987dd8` without textual conflicts, then
  checked the overlap semantically. The new crate-root module maps remain
  covered by the contract classifier's crate-source rules; parser/directive
  changes are the build surface, while the module-map roots are intentionally
  cross-cutting. `scripts/check-source-size.sh` exited 0, including exhaustive
  module-map validation. The new shared-state audit found 14 sites; every site
  has the required local rationale. Merged-tree dry receipts against
  `origin/main`: old selector 14/14, exit 0, 23.797s selection; new selector
  14/14, exit 0, 11.915s selection, with every changed path classified and the
  contract manifest correctly forcing all scenarios. Next: independently
  execute both matrices with bounded parallelism, then repeat the complete
  agent tier on the merged tree.

- 2026-08-05T13:39:38Z — Post-merge gate is synchronously green. Against
  `origin/main` at `d0b1f84`, the old selector selected 14/14 and exited 0 in
  678.075s; the new selector classified every track path, selected 14/14, and
  an independent `--rebuild` exited 0 in 667.960s. Both runs were exclusive,
  used one Nix build client, and never exceeded two QEMU guests. The merged
  non-VM gate then captured: source-size/module-map guardrail 0; workspace fmt
  0; examples fmt 0; warning-denied workspace/all-target Clippy 0; serial full
  workspace tests 0; explicit tour generation 0; post-generation tour tests
  0; committed tour drift 0; diff check 0; aggregate `.gate-exit` 0. No tour
  drift or other worktree change remained. Next: commit this receipt entry and
  leave the merged track clean for independent verification.

- 2026-08-05T15:20:49Z — Completed the orchestrator-trimmed final gate after
  merging `origin/main` at `780be01` (CIP-106) as `8151b85`. The append-only
  log conflict retained both histories. Selector validation first identified
  the new `crates/cix-test-support/src/lib.rs` as unclassified; adding
  `crates/cix-test-support/src/**` and `CLAUDE.md` to the non-product contract
  made the final dry run fully classified, selecting 14/14 in 12.074s with a
  captured exit 0. Standard-tier captured exits were: source-size/module-map
  guardrail 0; workspace fmt 0; examples fmt 0; warning-denied workspace/all-
  target Clippy 0; serial full workspace tests 0; explicit tour generation 0.
  A redundant parallel explicit tour invocation exited 101 when its two tests
  raced over `.workspaces`; the required serial correction exited 0, followed
  by tour drift 0 and diff check 0. The single permitted normal progressive
  run then selected 14/14, stayed at no more than two QEMU guests with no
  foreign Nix build parent, and synchronously exited 0 in 666.376s. No forced
  rebuild was rerun after the scope trim. Next: commit this receipt entry and
  leave the branch clean for orchestrator verification.

- 2026-08-06T02:05:00Z — `track/expand-ntfy-filebrowser` staged the pinned
  ntfy v2.27.0 and Filebrowser v2.63.23 upstream contexts with
  `corpus/migrate/fetch.sh`, then added both complete corpus anatomies,
  including faithful and dissolved Cixfiles, source provenance, checked
  artifact checksums, locks, probes, receipts, gaps, and corpus-ledger rows.
  Faithful and dissolved builds exited 0 for both cases; ntfy's system-manager
  probe returned the exact `{"healthy":true}` value. Filebrowser's runtime is
  deliberately not green: its upstream init cannot create `/config/settings.json`
  because arbitrary-path role realization makes `/config` read-only; the
  value-captured check exit is 1 and no health result is claimed. Next: cold
  replay and cold-stage compatibility checks, then Cixfile formatting/parser
  and final diff review.

  FRICTION:
  - `$VERSION` immediately followed by `_` needs the Cixfile shell spelling
    `$VERSION""_`; `${VERSION}` is interpreted as a Cixfile binder rather than
    a builder environment variable. The parser's diagnostic named the binder
    model, but the migration form is easy to reach for. → language
  - Raw upstream SHA-256 is not a `FETCH EXPECT` value: EXPECT hashes the
    fetched workspace directory. The first value-checked fetch reported the
    correct directory SRI, while the raw SHA remains verified inside FETCH. →
    language
  - `CONFIGDIR /config` looked right from the Docker volume name, but the
    upstream writes self-generated settings, so `STATEDIR` is the correct
    lifecycle. Both directives nevertheless expose the same arbitrary-path
    read-only mount defect; recorded in Filebrowser GAPS as a runtime wall. →
    language

- 2026-08-06T02:20:00Z — Final corpus-track receipts: fresh-workspace warm
  and `--cold` faithful builds both exited 0 for ntfy and Filebrowser, replaying
  their pinned FETCH snapshots; both dissolved twins also built cold with exit
  0. `regen-stage.sh` synchronously staged both cases with Dockerfile, SOURCE,
  check contract, cix binary, and pinned upstream `context/`. `cix fmt --check`
  exited 0 for all four Cixfiles. Corpus-browser regeneration exited 0, then
  the normal corpus suite (real parser, browser drift, determinism, and ledger
  discovery) passed 7/7 with one deliberate ignored generator. No Rust source
  changed, so the corpus-only spec does not require a workspace-wide test or
  VM matrix. Next: stage, review, commit the clean track branch; do not merge.

- 2026-08-06T02:25:00Z — Committed the complete corpus track as `57b39238`
  (`Corpus: add ntfy and filebrowser cases`). The commit contains no product
  Rust changes and no ignored upstream contexts. Filebrowser remains
  deliberately build-only; its exact runtime wall is retained in its receipt
  and GAPS rather than regraded as green. Next: commit this append-only commit
  record, verify clean branch state, and hand off without merging.
## 2026-08-06 — it-tools relock (section restored 2026-08-06: sat behind a stray diff3 base marker from the pnpm-wall-spike merge resolution)

- 2026-08-06T02:00:00Z — Started track/ittools-relock from `origin/main`.
  The case has a clean worktree, no fetched context, and a 1,544,041-line
  pre-CIP-99 lock. Next: fetch the pinned source, build the current cix from
  this worktree, then perform a scratch `--update-lock` build and a synchronous
  runtime probe; preserve any genuine wall in the case receipt and GAPS ledger.

- 2026-08-06T02:05:00Z — The pinned context fetch and `devenv shell -- cargo
  build -p cix` both exited 0. The binary is now available at
  `target/debug/cix`; no Rust source was changed. Next: remove only the case
  lock after copying it to ignored scratch, then wait synchronously for the
  current CIP-99 `--update-lock` build.

- 2026-08-06T03:00:00Z — The scratch `--update-lock` build exited 0 and
  produced `/nix/store/rqb8h3w47azlnf7l9y3g1h0fw13gfvw3-cix-item-web`.
  The new lock is 528,208 lines versus 1,544,041 before (−1,015,833,
  −65.79%); its step memo records 81,907 observed paths after CIP-99 root
  aggregation. Extended the case probe to print and assert the HTTP status;
  next run the synchronous check and record its actual status.

- 2026-08-06T04:00:00Z — The bounded check reached the item but its first
  runtime attempt was non-green: `./check.sh cix` was interrupted at 600s
  with exit 130 after repeated connection refusals. A direct detached system
  run gave a synchronous journal: nginx exited 1 because the upstream default
  config opened absent `/var/log/nginx/{error,access}.log`. The case's former
  `LOGDIR` workaround was already removed as DynamicUser-incompatible. Added
  a checked-in nginx config with stderr errors and disabled access-file logging;
  next: regenerate the lock for this final Cixfile and rerun the probe.

- 2026-08-06T04:20:00Z — The first config adjustment still left nginx's
  compiled main configuration in control: the direct journal again showed
  `/var/log/nginx/{error,access}.log`, before the `conf.d` server file could
  apply. Replaced the case config with a complete checked-in main config
  (`error_log stderr`, `access_log off`, `pid /run/nginx/nginx.pid`, events,
  and the SPA server) and pointed the Cixfile at `/etc/nginx/nginx.conf`.
  Next: re-lock this final assembly and rerun the live proof.

- 2026-08-06T04:45:00Z — The complete config was present in the item, but
  nginx still opened its compiled `/var/log/nginx/error.log` before parsing
  `/etc/nginx/nginx.conf`; the direct bounded probe exited 1 with the same
  journal. Added nginx's `-e stderr` startup option so initialization never
  requires that absent compiled log path. Next: regenerate the final lock and
  run `check.sh` with an exact HTTP-status receipt.

- 2026-08-06T05:05:00Z — The `-e stderr` probe reached nginx initialization,
  but the next synchronous journal showed the compiled `/var/log/nginx/access.log`
  open still occurs before config parsing. Added the managed `LOGDIR`, imported
  bash, and added a start wrapper that creates both compiled log files as the
  service identity before `exec nginx -e stderr`. Next: re-lock and perform the
  final runtime proof; if the managed log role itself fails, retain that journal
  as the honest wall.

- 2026-08-06T05:25:00Z — The managed `LOGDIR` was present, but the wrapper's
  synchronous journal showed `touch: command not found` (the service imports
  bash, not builder-only coreutils). Replaced `touch` with shell redirection,
  which needs no extra service dependency. Next: final lock refresh and probe.

- 2026-08-06T05:45:00Z — The wrapper then reached the managed log path but
  failed `: > /var/log/nginx/access.log` with permission denied. The captured
  unit properties show `LOGDIR` is a non-idmapped bind after this host drops
  `PrivatePIDs`; this is a cix-run environment wall, not a missing asset.
  One bounded alternative is under test: model nginx's compiled log path as
  ephemeral `RUNDIR` (access logging remains off), keeping `/run/nginx` as a
  second runtime role. If that cannot start, preserve the wall and do not
  claim an HTTP receipt.

- 2026-08-06T06:30:00Z — `RUNDIR /var/log/nginx` is a successful runtime
  workaround: a direct system-manager launch returned HTTP 200 on attempt 1
  (exit 0) for `/nix/store/4zalfi4g7n2bd52niggwbhh4873iq4h6-cix-item-web`.
  The current-cix lock remains volatile at 1,536,045 lines; the last scratch
  attempt again showed only `.modules.yaml` before its known lock-scale path,
  so it was interrupted at the declared bound rather than claimed as a green
  aggregate. The old check harness then printed HTTP 200 but exited 1 because
  its `--fail` curl treated the SPA's nonexistent-route 404 as fatal; next:
  make that secondary status observational, rerun the harness, and finalize
  the case ledgers with the aggregation wall retained.

- 2026-08-06T07:00:00Z — The ordinary fresh-build replay was synchronously
  bounded at 240 seconds and exited 124 before emitting an item. Extended
  `check.sh` with an explicit `CIX_ITEM` path, then ran
  `CIX_ITEM=/nix/store/4zalfi4g7n2bd52niggwbhh4873iq4h6-cix-item-web ./check.sh cix`;
  it exited 0 and observed `/` HTTP 200 plus the secondary 404. The 404 is
  observational, not a failed assertion. Updated the receipt, GAPS, and
  corpus row to separate this verified runtime from the unresolved volatile
  lock aggregation. Next: regenerate corpus output, run format/parser/drift
  checks, then commit the clean track.

- 2026-08-06T07:30:00Z — Corpus browser regeneration exited 0 and changed
  only the generated it-tools page/index for the regraded row. Synchronous
  gates also exited 0: Cixfile and workspace fmt checks, example formatting,
  warning-denied workspace clippy, full workspace tests, normal corpus drift /
  determinism tests, tour regeneration, tour drift, shell syntax, and
  `git diff --check`. No VM scenario is selected for this corpus-only track;
  the ordinary source replay remains the sole intentional exit-124 wall.

## FRICTION

- 2026-08-06T00:00:00Z — The generic `--update-lock build` spelling from the
  track spec was rejected synchronously because HTTPD's lock-bearing builder is
  named `httpd-build`; it made no build or lock change and is not a receipt.
- 2026-08-06T00:01:00Z — The case-specific `--update-lock httpd-build` spelling
  was also rejected before execution because HTTPD's FETCH has an author
  `EXPECT`; no lock change occurred. The valid fresh-workspace probe is a plain
  build against that verified snapshot, preserving the checksum assertion.
- 2026-08-06T00:02:00Z — The first successful assembly was not a valid cold
  pair: its warm workspace inherited `output/` from the preceding missing-
  context attempt, so cold correctly reported warm `Directory` versus cold
  `Absent` at `output`. Discarding that pair and rerunning with empty state and
  workspace is required; neither success is claimed as final evidence.
- 2026-08-06T00:03:00Z — Clearing only the isolated state/workspace still
  produced a completed-output memo hit (zero Nix subprocesses), because the
  case lock carries its memo metadata. That result is not a fresh-build
  receipt; the lock memo/output metadata must be removed before the forced
  regeneration.
- 2026-08-06T00:04:00Z — The valid clean warm build exited 0, but its
  synchronous cold replay exited 1 at generated
  `src/modules/core/.libs/mod_watchdog.o` (warm `Some(Absent)`, cold `None`).
  This is not a network or upstream wall; do not regrade HTTPD or claim the
  cold gate until the exact syscall evidence explains the mismatch.
- 2026-08-06T05:57:28Z — Preserved syscall evidence identified three scheduling
  classes behind the HTTPD replay mismatches: probes before a later same-step
  `mkdir`, child cwd loss across split `clone`/`clone resumed` records, and
  direct `O_RDWR|O_CREAT|O_TRUNC` compiler outputs. The trace parser now
  suppresses observations beneath every same-step created root after the full
  trace, understands resumed syscall records, and classifies truncating creates
  as output-only; focused regressions and the full workspace test pass.
- 2026-08-06T05:57:28Z — Final HTTPD regeneration receipts are synchronous exit 0:
  clean warm build, empty-workspace cold replay, and `./check.sh cix` runtime
  probe, all producing `/nix/store/3zgq560rmcq6hs9i4p1z2hq5s8dznr23-cix-item-httpd`
  where applicable. The lock is 38,562 lines versus 124,383 before (delta
  `-85,821`), SHA-256
  `67f26d3a2e165a94e5a9264e04d84c03a3ea1e7d86b065380517a8b1dbd4a1fd`, and the
  whole-corpus lock diff names only HTTPD. Browser generation/drift/determinism,
  fmt, examples fmt, warning-denied clippy, full workspace tests, tour
  regeneration/drift/determinism, and progressive VM selection (0/14 selected)
  all exited 0. No merge performed.

## 2026-08-06 — pnpm-wall spike

- 2026-08-06T07:20:32Z — Started `track/pnpmwall-spike` from clean
  `fefb1ddf` with the devenv active. Read the current project journal,
  authoritative D38/D39/D47/D56–D58 build decisions, the track spec, the
  four-chapter draft, and the dozzle/verdaccio/directus corpus contracts.
  The pinned contexts are absent as expected; next: restore all three with
  `bash corpus/migrate/fetch.sh <case>`, then diagnose dozzle under a bound
  with explicit cacert, pnpm network logging, strace, and socket capture.

### FRICTION

- 2026-08-06T07:20:32Z — `crates/cix/LOG.md` contains histories from many
  merged tracks and is not globally chronological, despite the repository
  journal being newest-first. This track appends its own dated section at EOF
  to preserve the explicit append-only contract. → process

- 2026-08-06T07:22:18Z — Restored all three immutable corpus contexts with
  synchronous `bash corpus/migrate/fetch.sh dozzle|verdaccio|directus`
  invocations; each exited 0 and reported the SOURCE-pinned revision. Their
  project pins are dozzle pnpm 11.17.0, verdaccio pnpm 11.1.2, and directus
  pnpm 10.27.0. The first two are the required store-spike matrix; directus is
  retained for the separate upstream-coherence check. Next: build the current
  `cix` binary, then run the bounded dozzle network diagnosis.

### Evidence

- 2026-08-06T07:33:00Z — Dozzle hang class: **cacert masquerade**, not IPv6
  fallback. A clean exact-pnpm-11.17.0 probe with the nixpkgs cacert bundle,
  pnpm debug NDJSON, `NODE_DEBUG=net,tls`, `strace -ff -e trace=network`, and
  once-per-second `ss -tnoap` capture exited 0 in 6561 ms; it verified 818
  lock entries, every `pnpm:fetching-progress` event had `attempt: 1`, and
  recorded 52 IPv6 plus 38 IPv4 TCP connect calls with zero TLS verification
  errors. Exact repro: the bounded command recorded at
  `/var/tmp/cix-pnpmwall-dozzle.WG4ReW` (status in `exit-status`; logs in
  `pnpm.ndjson`, `pnpm.stderr`, `network.strace.*`, and `ss.log`).

- 2026-08-06T07:33:00Z — The actual cix IMPORT-union A/B reproduced the old
  symptom. Without `${pkgs.cacert}`, `timeout 180 ../../../../target/debug/cix
  build --file Cixfile.pnpmwall-without-cacert --update-lock web ...` exited
  124 with zero store/CAS files; retained trace
  `/var/tmp/cix-read-trace-pSTPmk/syscalls` ended in repeated ENOENT probes for
  OpenSSL hashed certificates. With `${pkgs.cacert}`, the same clean build
  completed both FETCH stability probes, named only derived volatility
  (`.cache/pnpm/lockfile-verified.jsonl`, store `index.db`, and
  `node_modules/.modules.yaml`), executed FETCH in 23108 ms, copied only
  `.local/share/pnpm/store/v11/files`, and exited 0 with item
  `/nix/store/rb7n26wc79a6bqypsyjv95ag9rpkgr43-cix-item-pnpm-store`. Exact
  green repro and foreground log:
  `/var/tmp/cix-pnpmwall-cix-with-green.8niV1O/build.log`.

### FRICTION (continued)

- 2026-08-06T07:33:00Z — The first cacert-enabled diagnostic completed its
  FETCH but the deliberately minimal artifact COPY named the historical
  `.pnpm-store/v11/files` location; pnpm 11.17 now reported its real store as
  `.local/share/pnpm/store/v11`. That run exited 1 rather than being promoted
  to a green receipt; correcting the consumer path produced the exit-0 run
  above. Store location is version/configuration data, not a stable hard-coded
  cix convention. → language

### Two-phase store spike

- 2026-08-06T07:39:36Z — **Bare pnpm CAS does not work at either required
  version.** Two clean network FETCHes per corpus pin produced byte-identical
  `files/` trees: dozzle/pnpm 11.17.0 =
  `sha256-4QrchaNTAHRaccr7KG/BbRXiihxDrlv6T3tl9BnoIXo=` (20,175 files),
  verdaccio/pnpm 11.1.2 =
  `sha256-9yhmtdGwQCapcFYJkazRhC+l1XSwkTL4eaFuMg6hU/s=` (56,280 files).
  Exact roots are `/var/tmp/cix-pnpmwall-store-dozzle.dmg1gk` and
  `/var/tmp/cix-pnpmwall-store-verdaccio.pFFbxD`; each `fetch` and `fetch-2`
  ran foreground under `timeout 900`, exited 0, and the harness value-checked
  both NAR hashes and file counts with `cmp`.

- 2026-08-06T07:39:36Z — After copying only that immutable `files/` tree into
  two independent stores, `pnpm install --offline --frozen-lockfile
  --ignore-scripts --store-dir <bare>` did **not** reconstruct `index.db`.
  Dozzle's two installs both exited 1 with
  `ERR_PNPM_NO_OFFLINE_TARBALL` for the same first package
  (`@codemirror/autocomplete@6.20.3`) while leaving the CAS hash unchanged.
  Verdaccio's two installs also exited 1 with the same error class, but
  concurrent lookup chose different first packages (`@changesets/changelog-
  github@0.7.0` vs `@changesets/get-github-info@0.8.0`); the wall is stable,
  its first diagnostic is not. Raw independent `index.db` SHA-256 values also
  differ (dozzle `b5ef…` vs `86ec…`; verdaccio `2b16…` vs `94fc…`), and the
  database embeds volatile `checkedAt` values in its `package_index.data`
  blobs. Therefore neither exclusion+offline regeneration nor byte-level
  normalization by SQLite dump is available generically.

- 2026-08-06T07:39:36Z — npm 11.13.0's cacache has the analogous physical
  split (`_cacache/content-v2` plus `_cacache/index-v5`) but a materially
  different replay property: a package-lock's integrity maps directly to
  content-v2. Two independent `npm ci --offline --ignore-scripts --no-audit
  --no-fund` runs from content-v2 alone exited 0, made no AF_INET/AF_INET6
  connect, did not regenerate index-v5, preserved content hash
  `sha256-tVga/A8A4vn7TVttbR5tomYScTVN66BWJIEEFoZX7sM=`, and produced identical
  node_modules hash
  `sha256-e2p79xOvHAbRCqnCa7N9w45aFB+2ZblZIywvPG8mb0w=`. Exact foreground
  evidence: `/var/tmp/cix-pnpmwall-npm-cacache.we9YUR/no-audit-{a,b}`.

### FRICTION (continued)

- 2026-08-06T07:39:36Z — The first verdaccio comparison harness used plain
  `cmp` under a shell without `set -e`; it printed differing first-package
  errors but continued to an exit-0 summary. That summary is explicitly
  invalid as a receipt. The individual captured install exits and CAS hashes
  remain observations; the differing diagnostic is recorded above, not
  papered over as deterministic. → process
- 2026-08-06T07:39:36Z — npm `--offline` still performs the audit request by
  default: the first otherwise-successful bare-content run opened a registry
  TCP connection and the harness correctly exited 96. Adding `--no-audit
  --no-fund` yielded the two network-silent exit-0 runs above. “Offline
  package materialization” and “all npm subfeatures networkless” are distinct
  switches. → ecosystem
- 2026-08-06T07:39:36Z — `nix build --print-out-paths nixpkgs#sqlite` returns
  multiple outputs, so treating it as one executable path produced no dump;
  those empty-file comparisons are invalid. The rerun used
  `nixpkgs#sqlite.bin`, exited 0, and showed the logical dumps differ too. → nix

### Verdaccio payoff attempt

- 2026-08-06T08:04:00Z — A two-builder Verdaccio rewrite sealed only the
  stable pnpm 11 `files/` CAS from FETCH and copied it into a fresh build
  builder. The dependency FETCH itself is green: it exited 0, ran both cix
  probes, and produced `/nix/store/swbd0jgxv3zr1skfjl4gl41a9kwzapq3-cix-build-view`.
  The downstream cix build did not become a payoff: after bypassing pnpm
  11.17's project-version switch with `--pm-on-fail=ignore`, the foreground
  `timeout 360` run exited 124 while the offline install was still under the
  read tracer (`/var/tmp/cix-pnpmwall-verdaccio-payoff.OGv38g/build-sixth.log`,
  `sixth-exit-status`, and `/var/tmp/cix-read-trace-WvMuLc/syscalls`). This
  timeout is a wall, not a green receipt. The smaller two-run replay above is
  the value-checked semantic result: pnpm cannot resolve lockfile packages
  from `files/` without its consumed volatile index.

### FRICTION (continued)

- 2026-08-06T08:04:00Z — COPY cannot target a missing deep directory in a
  builder, and a preceding RUN that creates it does not make that directory
  visible to the next COPY staging transaction. The diagnostic therefore
  copied the CAS to a top-level name and moved it in a RUN. → language
- 2026-08-06T08:04:00Z — pnpm 11.17 honors Verdaccio's `packageManager:
  pnpm@11.1.2` before install; `COREPACK_ENABLE_PROJECT_SPEC=0` and generic
  config spelling did not disable that new pnpm-native behavior. The precise
  current switch is `--pm-on-fail=ignore`. → ecosystem
- 2026-08-06T08:04:00Z — The full monorepo offline install under cix's read
  tracer did not reach a package diagnostic within 360 seconds even though
  the reduced replay reaches `ERR_PNPM_NO_OFFLINE_TARBALL` promptly. The
  bounded synchronous timeout remains useful evidence, but tracing overhead
  makes it a poor inner loop for this payoff shape. → performance

### Directus coherence check

- 2026-08-06T08:13:00Z — Independent validation **disproves** exhibit 4's
  upstream-incoherence diagnosis. The fetched context's `package.json` and
  `pnpm-lock.yaml` hashes exactly match git revision
  `b1d7a45a77661fd13928a53448c06649f36b56f5`. Under the declared Node 22 and
  exact pnpm 10.27.0, `pnpm install --lockfile-only --frozen-lockfile
  --ignore-scripts` exited 0 in 230 ms for all 41 workspace projects. A full
  install against a clean empty store explicitly reported “Lockfile is up to
  date, resolution step is skipped,” then exited 1 at the expected
  `ERR_PNPM_NO_OFFLINE_TARBALL`; it did not report a manifest/lock mismatch.
  Synchronous logs and statuses are under
  `/var/tmp/cix-pnpmwall-directus-current.6xD66E`.

- 2026-08-06T08:13:00Z — A nearby independently checked coherent revision
  also exists: `d87981b99d2e7916905ac797fda79f33dc01190b` (`fix dependencies`,
  the pinned commit's second first-parent predecessor). A detached full
  checkout under Node 22 and pnpm 10.27.0 passed the same frozen-lockfile
  validation for all 41 workspaces, exit 0 in 233 ms. Exact foreground
  receipt: `/var/tmp/cix-pnpmwall-directus-nearby.uONmCj/install.log` and
  `exit-status`. The 14 narHash regenerations are therefore not gated by
  upstream lock incoherence; Directus's observed current wall is instead
  incomplete package metadata for offline deploy.

### FRICTION (continued)

- 2026-08-06T08:13:00Z — The first independent Directus validation used the
  ambient Node 24 and exited 1 at pnpm's engine guard (`Expected: 22`), before
  lock validation. It is invalid as coherence evidence. Re-running with
  nixpkgs Node 22 produced both value-checked receipts above. → process

### Reproduction and gate

- 2026-08-06T08:13:00Z — Minimal exact semantic repro for either pnpm pin:
  in a clean copy of the corpus context, run `corepack pnpm@<pin> fetch
  --ignore-scripts --store-dir fetch-a`, copy only
  `fetch-a/v11/files` to `bare-a/v11/files`, then run `corepack pnpm@<pin>
  install --offline --frozen-lockfile --ignore-scripts --store-dir bare-a`.
  Use 11.17.0 in `corpus/migrate/docker/dozzle/context` and 11.1.2 in
  `corpus/migrate/docker/verdaccio/context`; both final commands synchronously
  exit 1 with `ERR_PNPM_NO_OFFLINE_TARBALL`. Repeat as `fetch-b`/`bare-b` and
  value-check CAS identity with `nix hash path fetch-{a,b}/v11/files`. The
  captured two-run results and every per-command status are under the
  `cix-pnpmwall-store-{dozzle,verdaccio}` roots named above.

- 2026-08-06T08:13:00Z — Directus exact repro from the worktree root:
  `cd corpus/migrate/docker/directus/context && nix shell nixpkgs#nodejs_22
  -c bash -lc 'corepack pnpm@10.27.0 install --lockfile-only
  --frozen-lockfile --ignore-scripts'` exits 0. Adding `--offline --store-dir
  /path/to/empty-store` to a full install passes lock validation and exits 1
  only at `ERR_PNPM_NO_OFFLINE_TARBALL`. The nearby-revision receipt used a
  detached checkout of `d87981b99d2e7916905ac797fda79f33dc01190b` and the
  same first command, which exited 0.

- 2026-08-06T08:13:00Z — Final scoped gates all exited 0 synchronously:
  `devenv shell -- cargo fmt --all --check`; `devenv shell --
  target/debug/cix fmt --check corpus/migrate/docker/dozzle
  corpus/migrate/docker/verdaccio corpus/migrate/docker/directus`; corpus
  regeneration via `devenv shell -- cargo test -p cix --test corpus --
  --ignored generate_corpus_browser`; and the normal `devenv shell -- cargo
  test -p cix --test corpus` suite (7 passed, 1 generator ignored). Generated
  browser changes are limited to the three touched cases, and `git diff
  --check` exits 0. No Rust source changed, so this corpus-only spec does not
  call for `cargo test --workspace`.

### FRICTION (continued)

- 2026-08-06T08:16:00Z — The first corpus regeneration preceded the manual
  `docs/corpus.md` ribbon audit. After correcting the three stale ledger rows,
  the drift test exited 101 and named exactly Directus, Dozzle, and Verdaccio
  in `docs/corpus/index.html`; it is not a green receipt. Regenerating after
  the ledger edit exited 0, and the immediately following normal suite passed
  7/7 with the generator ignored. → process

- 2026-08-06T08:18:00Z — Committed the scoped spike as `9a3f5896` (`Spike
  pnpm cold-store wall`). The draft remains unadopted; the track makes no
  language decision. Dozzle's trust prerequisite is fixed, bare-CAS pnpm
  replay is rejected with two-version evidence, Verdaccio retains the precise
  missing-index wall, and Directus's prior incoherence diagnosis is corrected.
  No merge was performed.

## 2026-08-06 — coldreplay sweep

- 2026-08-06T08:06:49Z — Started `track/coldreplay-sweep` from a clean
  worktree. Scope is seven existing source-compile corpus cases: replay each
  faithful lock cold under the widened self-write parser, regenerate only a
  genuine mismatch, and prove the final whole-corpus lock diff. No Rust changes
  are authorized by the track. Next: build the current cix, restore each
  ignored pinned context as needed, then run the receipts synchronously.

- 2026-08-06T08:08:00Z — The first exact Redis cold command exited 1 before
  replay because this worktree's local cache lacked the locked FETCH snapshot;
  cix explicitly reported `--cold never refetches` and made no lock change.
  This is an invalid cold pair, not a case mismatch. The receipt's older warm
  step must be replayed first to materialize the local snapshot; then cold is
  rerun and value-checked.

- 2026-08-06T08:12:00Z — Redis's warm prerequisite exited 1 on upstream
  tarball EXPECT drift, with declared `sha256-PsELmdlX+Gux9kXvgjb2FuGNuTKFgZ1xxZPEyiGhroo=` and observed
  `sha256-LijBYlrDlf7uKh6a7XacdBt0oLJbRXtp8BkqgJ126Nc=`. The failed attempt
  also dirtied `Cixfile.lock` by inserting this exact transient block:
  `"builderDevEnvs": { "redis-build": "0c893f7880636a08462d076382173a67170e69179867c354685cdb0c8d583b1d:51e3e3998882359eb48d3cba5d5a0b17fb9a1a691870d224ad473f48e136f34c" },`
  before `"outputs"`; it was removed byte-for-byte. Redis is recorded as an
  upstream-drift wall and the sweep continues.

- 2026-08-06T08:15:00Z — Mosquitto's warm prerequisite exited 1 at its GPG
  key FETCH: declared `sha256-vSgE3uH3CP8JOW6MW1bZ7CAH6sEglZQHhKQDPqweLNs=`;
  observed `sha256-t6R8FXIdR0EmuvPy0OTBWvZzkNnmF/y4rNCsyVUr67c=`. No lock
  diff occurred. This is a second upstream-drift wall, recorded in the case
  GAPS/receipt; continue to the next case.

- 2026-08-06T08:18:00Z — Memcached's warm prerequisite exited 1 on source
  tarball EXPECT drift: declared `sha256-F+YQ+MXoOLqMZsr63YPuVpRDyWgvUxKDBog1frUyOhM=`;
  observed `sha256-Oar8dErGyg32RgIbe4NtXHyJ0E062bX+IlMpbjUue94=`. The failed
  attempt inserted this transient lock block, which was removed byte-for-byte:
  `"builderDevEnvs": { "build": "0c893f7880636a08462d076382173a67170e69179867c354685cdb0c8d583b1d:6af9386a2873340ab115d6b11102ec7b83f857f583dcf3700dd3982d268b05be" },`
  before `"outputs"`. Memcached is recorded as an upstream-drift wall.

- 2026-08-06T08:22:00Z — Nginx warm and cold faithful commands both exited 0;
  cold returned `/nix/store/aqf3p4z5gyjbx5pqfsvjdclz5iyiayz1-cix-item-nginx`.
  Verification dirtied only `Cixfile.lock`'s `sourceHash`, exactly
  `2289625103e7245081b02115293cc8910f4da9520cdb8104152ec153e26dfba0` →
  `31aa13b1809fbe04ae8957eac7ca84368a76f92cf35dad307e5afb73302fdf93`.
  The line was restored byte-for-byte and the case is retained as a
  keying-neutrality exhibit; no regeneration was performed.

- 2026-08-06T08:27:00Z — HAProxy's warm prerequisite exited 1 on its source
  tarball EXPECT: declared `sha256-3nwJp1hqkCTmrDZFwIfxmWdkqqGLbgIVVv5tyG/bEgo=`;
  observed `sha256-Wsa8Y8YS2fvnln9n7uDSj34kg/8gNHb6aydwV4psjGI=`. No lock diff
  occurred. HAProxy is recorded as an upstream-drift wall and the sweep
  continues.

- 2026-08-06T08:34:00Z — Tomcat warm and cold faithful commands both exited 0,
  with cold returning `/nix/store/5bqhzp9yc7plf621fr33560zs6hdz41v-cix-item-tomcat`.
  Verification dirtied the lock with exactly this two-line output change:
  `sourceHash` `4e8b397afdd22a4bc32bf5e1beffd2be13842037a8bbfdbac64df7f809a1ff14`
  → `a98267fb02f1acf91908f1e3e8f8ae081bae22b9f65e37b7f186dd97a2c5a60a` and
  `storePath` `/nix/store/s58jpph2qgzj18xwwam5is3jkzhqa9mf-cix-item-tomcat`
  → `/nix/store/5bqhzp9yc7plf621fr33560zs6hdz41v-cix-item-tomcat`. Both lines
  were restored byte-for-byte; Tomcat is a keying-neutrality wall, not a
  regeneration candidate.

- 2026-08-06T09:10:00Z — Valkey's first ordinary warm command was discarded
  as an invalid completed-output memo hit (`zero Nix subprocesses`). The clean
  workspace `--update-lock build` exited 0 after a full 251.120 s RUN; both
  FETCH update probes were identical and the canonical lock stayed byte-
  identical. The valid cold replay from that pinned snapshot exited 0 after a
  full 235.539 s RUN and returned `/nix/store/fgm45ck2453mrhpv4hqhc64kcwa3f6-cix-item-valkey`.
  Valkey is verified under the widened parser; no regeneration was needed.

- 2026-08-06T09:12:00Z — Whole-corpus SHA-256 comparison over every tracked
  `corpus/migrate/docker/*/Cixfile*.lock` against `HEAD` synchronously exited
  0 and reported `whole-corpus changed lock count: 0`. This proves no lock
  changed and therefore no case was regenerated; the remaining lock-churn
  observations were restored exactly as required.

- 2026-08-06T08:23:29Z — `devenv shell -- cargo fmt --all --check` and
  `git diff --check` exited 0. The first corpus drift check exited 101 only
  because the seven edited receipts/GAPS panels had not yet been regenerated;
  `devenv shell -- cargo test --test corpus -- --ignored generate_corpus_browser`
  exited 0, and the final `devenv shell -- cargo test --test corpus` exited 0
  with 7 passed, 0 failed, and 1 ignored (including browser determinism).

- 2026-08-06T08:23:29Z — Timestamp correction: the earlier entries labeled
  08:27:00Z, 08:34:00Z, 09:10:00Z, and 09:12:00Z were sequence labels entered
  ahead of the observed wall clock, not detached receipts. Their command
  outcomes and exact values are unchanged; this append-only correction gives
  the actual synchronous gate timestamp above.

## 2026-08-06 — tourdet teardown

- 2026-08-06T09:20:00Z — Started `track/tourdet-teardown` from `9f103413`.
  The failure is confined to the executed tour harness. Source inspection
  confirms run units are transient `systemd-run --collect` services
  (`crates/cix-run/src/manager.rs`), so a completed/stopped unit is unloaded
  asynchronously; this is not `RemainAfterExit=yes` (that setting belongs to
  the scheduled-APP GC-root helper). Next: make only a post-stop
  `LoadState=not-found` successful, audit each tour transient-unit stop, add a
  hermetic regression for the classification, then run the Rust/tour gates.

### FRICTION

- 2026-08-06T09:20:00Z — The spec's parenthetical calls this
  `RemainAfterExit` semantics, but the authoritative implementation uses
  `systemd-run --collect`; retaining that distinction prevents a product
  lifecycle change to solve a harness teardown race. → process

- 2026-08-06T09:28:00Z — Implemented the harness-only classification at both
  cleanup seams and every tour receipt that stops a transient run unit
  (Chapter 1, Chapter 5's two web runs, and Chapter 6's listener and unary
  producer). The generated command retries no stop: it accepts a failed stop
  only when a fresh `systemctl --user show --property=LoadState --value` says
  exactly `not-found`; `loaded` and all other states remain failures. The
  scheduled timer's explicit removal is a persistent user-unit lifecycle, not
  a `--collect` transient race, so it is unchanged. `cargo fmt --all --check`
  and the hermetic `tour_stop_only_accepts_an_unloaded_unit_after_a_stop_failure`
  test each synchronously exited 0. Exact repro: `devenv shell -- cargo test
  -p cix --test tour tour_stop_only_accepts_an_unloaded_unit_after_a_stop_failure
  -- --exact`.

- 2026-08-06T09:31:00Z — The first synchronous two-render receipt,
  `devenv shell -- cargo test -p cix --test tour generated_tour_is_deterministic
  -- --exact`, exited 0 (1 passed, 61.04s). Explicit regeneration via
  `devenv shell -- cargo test -p cix --test tour -- --ignored generate_tour`
  then exited 0 (1 passed, 29.92s). Next: verify the committed-document drift,
  run the requested workspace gates, review generated pages, and commit the
  clean track; no product/runtime code changed.

- 2026-08-06T08:53:51Z — Timestamp correction: the preceding track entries
  labeled 09:20:00Z, 09:28:00Z, and 09:31:00Z were sequence labels entered
  ahead of the observed clock, not detached receipts. Their value-checked
  outcomes and exact commands are unchanged.

- 2026-08-06T08:53:51Z — Final synchronous receipts all exited 0: `devenv
  shell -- cargo fmt --all --check`; `devenv shell -- cargo run -p cix -- fmt
  --check examples`; `devenv shell -- cargo clippy --workspace --all-targets
  -- -D warnings`; and `devenv shell -- cargo test --workspace --
  --test-threads=1` (including the tour suite: 6 passed, 1 explicit generator
  ignored). Tour regeneration and committed-document drift are independently
  checked above; `git diff --check` also exited 0. The only generated changes
  are the expected Chapter 1/5/6 teardown commands. No focused VM scenario or
  Docker/corpus ledger row applies to this harness-only track. Next: commit
  the reviewed scoped diff; do not merge.

- 2026-08-06T08:55:05Z — Committed the scoped track as `d5ede6e6` (`test:
  make tour transient teardown idempotent`) and created
  `origin/track/tourdet-teardown`. The push transferred that exact commit but
  could not set its local upstream because the shared repository has a
  pre-existing read-only `/home/mathijs/composix/.git/config.lock`; it is not
  removed. The independent synchronous value check `git ls-remote origin
  refs/heads/track/tourdet-teardown` returned
  `d5ede6e613f1e2210533db262e5aeab5e0799c97`, equal to local `HEAD`, and the
  worktree is clean. No merge was performed.

## 2026-08-06 — fmt-key evidence

- 2026-08-06T00:00:00Z — Started `track/fmtkey-evidence` from the supplied
  evidence-only spec. Read the draft, exhibit receipts, lock types, build
  fingerprint construction, trace hashing, and FETCH-state pinning. The first
  source trace establishes an important boundary to prove with tests: persisted
  `dev`/`inode`/`mtimeNs` fields are validation hints in `stepMemo` and are not
  serialized into `build_fingerprint`; `read_hash` and automatic FETCH path
  pins instead fold the complete POSIX mode. Next: add hermetic CURRENT-behavior
  characterization tests, then write the per-site NAR inventory without
  attributing the recorded sourceHash churn to fields that do not flow there.

### FRICTION

- 2026-08-06T00:00:00Z — The three corpus receipts call the observations one
  family, while the code currently has materially different paths (source-tree
  hash, full-mode read hashes, FETCH lock pins, and non-key validation hints).
  The evidence chapter must preserve that distinction rather than turn a
  plausible correlation into a causal claim. → evidence

- 2026-08-06T09:26:35Z — Added three hermetic **CURRENT behavior** tests and
  the draft evidence inventory. `read_hash` and automatic FETCH path pins each
  distinguish `0644 → 0600` despite identical bytes; the source fingerprint
  consumes arbitrary non-lock source files and `fetches`, but not source file
  modes or `stepMemo`. Focused synchronous receipts all exited 0:
  `cargo fmt --all --check`; the two `cix-build` characterizations; the
  `cix-cixfile` source/fetch/stepMemo characterization; and `git diff --check`.
  The structural audit found no new `Arc`/`Rc`/`Mutex`/`RwLock`/`RefCell`/static
  site in this track's changed Rust. Next: run the requested warning-denied
  clippy and full workspace-test gates, then review/commit the evidence-only
  diff without merging.

- 2026-08-06T09:26:35Z — One preliminary focused-test command incorrectly
  supplied two Cargo test filters and synchronously exited 1 before compiling;
  it was discarded and rerun as two independent named-test commands, each exit
  0 above. → process

- 2026-08-06T09:26:35Z — Requested full Rust gates are synchronously green:
  `devenv shell -- cargo clippy --workspace --all-targets -- -D warnings`
  finished with exit 0, and `devenv shell -- cargo test --workspace` finished
  with exit 0 (all non-ignored workspace tests passed, including the three new
  characterizations). There is no VM gate for this evidence-only track. Next:
  final diff/status review, then commit the scoped branch; do not merge.

- 2026-08-06T09:26:35Z — Final scoped review found exactly the evidence draft,
  three characterization-test locations, and this append-only journal; staged
  `git diff --check` passed. The clean evidence track is committed and remains
  unmerged; terminal final-state verification confirms the amended commit and
  clean worktree.

## 2026-08-06 — narhash regens

- 2026-08-06T09:23:29Z — Started `track/narhash-regens` and completed the
  required pre-edit fetch-level inventory. `InputLock.narHash` values were
  excluded; the table below contains every non-empty `FetchPin.narHash` in
  `corpus/migrate/docker/*/Cixfile*.lock`. Classification is by matching the
  lock entry to an `EXPECT` carried by the corresponding Cixfile, not by the
  presence or absence of `snapshotNarHash`.

  | lock | fetch entry | narHash | classification |
  | --- | --- | --- | --- |
  | `caddy/Cixfile.dissolved.lock` | `builder:assets:0-98f9d79d6553` | `sha256-84v+HnEZ/9AwHhsD3ZeZ4DuCgyrw8/LEL6PTxGwsUrY=` | EXPECT-backed |
  | `caddy/Cixfile.dissolved.lock` | `builder:assets:1-c2fa3ad48195` | `sha256-rBR2z6KgBLjXqhqVUKjUsqCnrUaqoig+OGdPJK2f/XM=` | EXPECT-backed |
  | `caddy/Cixfile.lock` | `builder:upstream:2-98f9d79d6553` | `sha256-84v+HnEZ/9AwHhsD3ZeZ4DuCgyrw8/LEL6PTxGwsUrY=` | EXPECT-backed |
  | `caddy/Cixfile.lock` | `builder:upstream:3-c2fa3ad48195` | `sha256-rBR2z6KgBLjXqhqVUKjUsqCnrUaqoig+OGdPJK2f/XM=` | EXPECT-backed |
  | `caddy/Cixfile.lock` | `builder:upstream:4-b0ddf94cba0c` | `sha256-osNsyktm3O6Gcaihq7Ba6I+Ez4oTLEMNLea2mT3XnZo=` | EXPECT-backed |
  | `directus/Cixfile.lock` | `builder:build:3-e8a0afba7b11` | `sha256-vsqPz2Da9kHmra41ci3RU6bcF/vbwCHULrbh3sqMtKg=` | legacy-automatic (pnpm; out of scope) |
  | `echo-server/Cixfile.lock` | `builder:dependencies-fresh:5-36c6b50956d4` | `sha256-hSJ2xuInAJfmjLEUadUei8tfyu3jtW4S+ttTRcal2qM=` | legacy-automatic |
  | `echo-server/Cixfile.lock` | `builder:dependencies-fresh:5-8b4ab4e2175a` | `sha256-CdO/YytCv5FJPcbru38NW335RMODknzdDj5Jvn8BfRw=` | legacy-automatic |
  | `filebrowser/Cixfile.lock` | `builder:release:3-8bc1a68b59f9` | `sha256-k6239590pl8k4UojhPb4QmwjmlZwvlJ2ta9fsXC9wIM=` | EXPECT-backed |
  | `filestash/Cixfile.lock` | `builder:build:2-27a70b69a918` | `sha256-KSviBOEh74raIuUUupkL2T1sPpSqb9Ck1ByDOTRYVP8=` | legacy-automatic |
  | `filestash/Cixfile.lock` | `builder:build:2-42a8263a643c` | `sha256-HYMB8Ae+Nwd6RqAYViWFX4AmGP45fSvdLNr2qEdvVuM=` | legacy-automatic |
  | `haproxy/Cixfile.lock` | `builder:build:3-00ba784e8f7b` | `sha256-3nwJp1hqkCTmrDZFwIfxmWdkqqGLbgIVVv5tyG/bEgo=` | EXPECT-backed |
  | `httpd/Cixfile.lock` | `builder:httpd-build:3-bc8d912fbf6f` | `sha256-scD5NGTPwH0AsHCMCv2ewVVUNyPkgnlFCee8P3ofgbU=` | EXPECT-backed |
  | `memcached/Cixfile.lock` | `builder:build:3-1868940fa4df` | `sha256-F+YQ+MXoOLqMZsr63YPuVpRDyWgvUxKDBog1frUyOhM=` | EXPECT-backed |
  | `mosquitto/Cixfile.lock` | `builder:build:3-9ac5f5370493` | `sha256-8LobuvzinnPOoFGcKeeR3amC4nfSa5ijN1g2ilcP5lg=` | EXPECT-backed |
  | `mosquitto/Cixfile.lock` | `builder:build:4-3778efd0689a` | `sha256-5WzB/nFTRjxAHJGomdnPeKAKXk83xMXtTB92T4Tb4+s=` | EXPECT-backed |
  | `mosquitto/Cixfile.lock` | `builder:build:5-1c4685c6da96` | `sha256-vSgE3uH3CP8JOW6MW1bZ7CAH6sEglZQHhKQDPqweLNs=` | EXPECT-backed |
  | `nats/Cixfile.lock` | `builder:final:3-11bbc8e4e546` | `sha256-0a7KA8SV3F8VxIqNAIJNipngAvAW0YUdTiLFSoc3B7w=` | legacy-automatic |
  | `nats/Cixfile.lock` | `builder:runtime:3-11bbc8e4e546` | `sha256-kqUeAFIPk6wGUFYrYQQX/bBX6ryPdaCOUuLCbWjc838=` | EXPECT-backed |
  | `ntfy/Cixfile.lock` | `builder:release:3-6b7b0677f094` | `sha256-lGGZu8Tos7cQvG0RRQ3cPyzULGhvcwpQ/w6/aN6BbaQ=` | EXPECT-backed |
  | `redis/Cixfile.lock` | `builder:redis-build:3-161bb32cce97` | `sha256-PsELmdlX+Gux9kXvgjb2FuGNuTKFgZ1xxZPEyiGhroo=` | EXPECT-backed |

  Inventory total: 21 fetch-level entries in 13 lock files; 15 are
  EXPECT-backed and 6 are legacy-automatic. The eligible non-pnpm refresh
  population is five entries in `echo-server`, `filestash`, and `nats`; the
  sixth legacy entry is Directus and remains excluded with the pnpm cases.
  No lock or receipt was changed before this entry was appended. Next:
  execute each eligible case's documented regeneration command, preserving
  EXPECT values at upstream-drift walls.

- 2026-08-06T09:35:00Z — Echo-server refresh receipt: `bash
  corpus/migrate/fetch.sh echo-server` exited 0, then `devenv shell --
  ./target/debug/cix build --update-lock dependencies-direct
  corpus/migrate/docker/echo-server#echo-server` exited 0 synchronously. The
  FETCH double-read was identical, npm installed 435 packages, webpack built
  the service, and the item was `/nix/store/hvr3vmwaba7ci38gc0f3009p13iq9vm1-cix-item-echo-server`.
  The command refreshed the active `dependencies-direct` pin but retained the
  two obsolete `dependencies-fresh` whole-tree entries; those stale lock-map
  entries are the five-entry cleanup target identified by the inventory, not
  new EXPECTs. `FRICTION`: my first literal transcription used the nonexistent
  `corpus/migrate/docker/fetch.sh` path (exit 127); no corpus state changed and
  the receipt's actual `corpus/migrate/fetch.sh` command then exited 0. Next:
  verify the case probe/cold command and clean only the stale legacy map entries
  after the eligible refresh attempts are complete.

- 2026-08-06T09:43:00Z — Filestash refresh attempt: `bash
  corpus/migrate/fetch.sh filestash` exited 0 and the documented build, run
  from the repository root as `devenv shell -- ./target/debug/cix build
  corpus/migrate/docker/filestash#filestash`, exited 1 synchronously after the
  FETCH (16.058s) and compilation reached the RUN. The upstream build failed
  on missing `brotli/decode.h` and `libraw/libraw.h`; no item or runtime probe
  is claimed and `filestash/Cixfile.lock` was not changed. The two legacy
  whole-tree entries therefore remain a wall, not a refresh. `FRICTION`: the
  receipt shorthand `.#filestash` was first tried at repository root and
  correctly failed before reading a Cixfile; the explicit case ref above is
  the value-checked attempt. Next: run NATS, then the four known EXPECT-drift
  walls, without changing any declared EXPECT.

- 2026-08-06T09:36:29Z — NATS refresh exited 0 synchronously with the
  documented `devenv shell -- ./target/debug/cix build
  corpus/migrate/docker/nats#nats` command. It fetched and checksum-verified
  `nats-server.tar.gz`, ran the extraction, and produced
  `/nix/store/rzk74i60ylpqy1x2drf65mjf7q612m9n-cix-item-nats`. The active
  `builder:runtime` pin now carries its snapshot; the stale undeclared
  `builder:final` legacy value was removed from the same refreshed lock. No
  EXPECT changed.

- 2026-08-06T09:36:29Z — Known EXPECT-wall receipts, all synchronous and
  value-checked, were run without accepting any new value: Redis failed at
  `sha256-PsEL…` versus fetched `sha256-LijBY…`; memcached failed at
  `sha256-F+YQ…` versus `sha256-Oar8…`; and HAProxy failed at
  `sha256-3nwJ…` versus `sha256-Wsa8…`. Their four source contexts were
  restored to the recorded revisions, and their tracked locks remain
  byte-identical to `HEAD`. Mosquitto's ordinary warm run initially reused
  memo hits; a temporary clean-lock replay at
  `/var/tmp/cix-narhash-mosquitto.viPLeP` forced all three EXPECT FETCHes and
  exited 0, including a good GPG signature, producing the existing broker item.
  The direct `--update-lock` attempt correctly refused because `EXPECT FETCH`
  is not updateable. The earlier same-day receipt's GPG-key drift is therefore
  recorded as a non-reproduced wall, not silently converted into a translation
  change; `mosquitto/Cixfile.lock` remains byte-identical to `HEAD`.
  `FRICTION`: the forced clean-lock probe required a temporary copy with memo
  records removed because ordinary warm replay hid the network FETCHes; that
  temporary case is outside the worktree and was not used as corpus state.

- 2026-08-06T09:36:29Z — Timestamp correction: the preceding Filestash
  entry was labeled `09:43:00Z` ahead of the observed clock. Its synchronous
  exit-1 result and wall classification are unchanged; this correction is the
  durable timestamp receipt, not a detached-process claim.

- 2026-08-06T09:37:58Z — Final value-checked gates: `devenv shell -- cargo
  test --test corpus -- --ignored generate_corpus_browser` exited 0;
  `devenv shell -- cargo test --test corpus` exited 0 (7 passed, 1 ignored,
  including committed-browser drift and determinism); and `devenv shell --
  cargo fmt --all --check` exited 0. `git diff --check` exited 0 and every
  one of the 51 Docker lock files parsed as JSON. The whole-corpus SHA-256
  audit compared each worktree lock with `git show HEAD:<path>`: exactly
  `corpus/migrate/docker/echo-server/Cixfile.lock` and
  `corpus/migrate/docker/nats/Cixfile.lock` changed; every other lock was
  byte-identical to HEAD. Browser regeneration changed only the corresponding
  `docs/corpus/docker-echo-server.html` and `docker-nats.html` pages.

  Remaining legacy-automatic inventory: **3 entries in 2 locks** — one in
  `corpus/migrate/docker/directus/Cixfile.lock` (pnpm, explicitly out of scope)
  and two in `corpus/migrate/docker/filestash/Cixfile.lock` (the synchronous
  missing-native-header build wall). The other 15 fetch-level hashes are
  EXPECT-backed and retained. No EXPECT value was changed.

### FRICTION

- 2026-08-06T09:37:58Z — The main friction was lock metadata churn: a failed
  or memo-hidden FETCH can rewrite inode/mtime observations even when no pin
  is accepted. I restored all four wall-only locks to their exact HEAD bytes,
  then retained only the two successful refresh locks. Receipt command
  shorthands also assumed case-directory context in two places; explicit root
  case refs were used and their exit statuses were captured synchronously.
  The branch is ready for commit and must not be merged in this track.

## 2026-08-06 — staterole-bindfix

- 2026-08-06T09:22:04Z — Started `.dev/specs/track-staterole-bindfix.md` on the assigned branch. The declared semantics remain arbitrary absolute app paths for all managed roles; D11's old FHS restriction was superseded by the current parser/runtime contract. Initial source characterization isolates the fault to `cix-run/src/directories.rs`: system-mode arbitrary roots are collected and emitted as `TemporaryFileSystem=<root>:ro` only after all role `BindPaths=`, so `/config:ro` can cover the earlier managed bind at `/config`. Next: capture the exact live filebrowser unit properties as a synchronous failure receipt, then move only realization ordering/add the focused VM proof.

### FRICTION

- 2026-08-06T09:22:04Z — The requested `cix-spec` location is no longer a workspace crate; the relevant manifest/spec model is `cix-manifest` and realization is owned by `cix-run`. → process

- 2026-08-06T09:24:26Z — Value-checked reproduction: the pinned Filebrowser item started with the current local `cix`, then its native readiness path exited 1 after the declared 10-second bound. While the transient unit was activating, `systemctl show` reported `StateDirectory=cix-run-filebrowser cix-run-filebrowser/config cix-run-filebrowser/database cix-run-filebrowser/srv`, `BindPaths=/var/lib/cix-run-filebrowser/config:/config:rbind …`, and later `TemporaryFileSystem=/var/lib:ro /config:ro /database:ro /srv:ro`; `ReadOnlyPaths=` and `ReadWritePaths=` were empty. The journal value was `cp: cannot create regular file '/config/settings.json': Read-only file system`. This proves the stated realization ordering, not an application semantic issue. The ordinary corpus rebuild is separately blocked because the worktree's Filebrowser context lacks the tracked copied `init.sh` source; the pinned item is present and was sufficient for the runtime reproduction. Next: replace the arbitrary-root tmpfs overlay with exact `ReadWritePaths=` exclusions under existing `ProtectSystem=strict`, then prove state backing/restart/root isolation in the dirs VM.

- 2026-08-06T09:37:15Z — Repaired only realization: arbitrary system-role targets now emit exact `ReadWritePaths=` allow-list entries instead of a later top-level `TemporaryFileSystem=:ro` overlay. The existing Filebrowser item then started synchronously, with `ReadWritePaths=/config /database /srv`, only `/var/lib:ro` as the temporary filesystem, and `curl --fail /health` returning `{"status":"OK"}`. The new `dirs2` VM member uses `STATEDIR /config/state`, increments durable state across a systemd restart, rejects a sibling `/config/undeclared` write, and asserts the exact allow-list/no `/config:ro` overlay. The focused seven-scenario derivation set completed and its exact current outputs were foreground value-checked by `nix build` exit 0. Corpus browser regeneration and its 7/7 drift suite also exited 0.

- 2026-08-06T09:37:15Z — User-manager characterization is a separate wall, now directly reproduced: `cix run --user` exited 1 after the user manager failed `NAMESPACE` setup (`Operation not permitted`); cix then deliberately retried without `PrivateUsers`, `PrivatePIDs`, `ProtectSystem`, `ProtectHome`, `PrivateTmp`, and `BindPaths`, before native readiness timed out. This is the documented degraded fallback, not an arbitrary-path ordering problem; the exact transient unit was stopped and confirmed `LoadState=not-found`. Filebrowser GAPS/receipt and corpus/Docker ledgers now say so honestly. Next: run full Rust/examples gates, structural audit, review, and commit only the scoped track.

- 2026-08-06T09:42:54Z — Final synchronous gate set completed without a failure: `cargo fmt --all --check`; `cargo run -p cix -- fmt --check examples`; warning-denied `cargo clippy --workspace --all-targets -- -D warnings`; and the full workspace test suite (including corpus and tour integration tests). The targeted unit regression and corpus browser regenerate+drift receipts are above. `git diff --check` passed; the mandated shared/interior-mutability audit reported only existing, justified sites outside this directory-realization change. The focused VM selection covered closedroot-audit, dirs2, gc-survival, lifecycle, side-by-side, tree, and update-repin, and the current selected derivations were value-checked by foreground `nix build` exit 0. Next: commit this reviewed scope; do not merge.

- 2026-08-06T09:43:00Z — Committed the scoped repair as `8ba2f1c6` (`fix: preserve arbitrary role-directory binds`). It includes the runtime realization, focused VM regression, Filebrowser and user-fallback receipts, and regenerated corpus ledger. No merge or push was performed. Next: final clean-branch check and handoff.

### FRICTION

- 2026-08-06T09:42:54Z — `nixfmt` is not supplied by the active devenv (`exec: nixfmt: not found`). The NixOS test framework's type check and test-script lint both accepted the changed scenario; the required Rust/examples format gate is green. → environment

## 2026-08-06 — fmtkey-impl

- 2026-08-06T00:00:00Z — Started CIP-110 implementation from
  `.dev/specs/track-fmtkey-impl.md`. The accepted contract requires one
  NAR-invariant filesystem identity primitive (type, content, executable bit,
  symlink target), canonical-AST semantic key serialization with honest
  versioning, and a lock/cold-replay/fmt equivalence fixture. Corpus locks must
  not be regenerated in this track. Next: map every listed keying site and the
  existing characterization tests before changing the implementation.

### FRICTION

- 2026-08-06T00:00:00Z — `crates/cix/LOG.md` carries append-only history from
  prior tracks in this shared worktree lineage; this track's entries are under
  the `fmtkey-impl` heading and will remain append-only.

- 2026-08-06T00:00:00Z — The first focused library receipt compiled the new
  code but failed one subtree-aggregation assertion: the new identity result
  was accepted even when the existing completeness check returned `None`.
  Restored that guard before continuing; the failed receipt is discarded and
  will be replaced by a synchronous passing run. → implementation

- 2026-08-06T00:00:00Z — A combined local verification command let a failed
  initial `cargo fmt --check` be followed by successful tests, so it was not a
  value-checked receipt for formatting. Rustfmt's only requested change was a
  one-line assertion; it will be applied and each gate rerun separately. →
  process

- 2026-08-06T00:00:00Z — The first serial workspace run failed only the two
  store-backed tour tests: Nix's Git source intentionally omitted the new,
  untracked `fingerprint.rs`, so its isolated build could not resolve the
  declared module. Both new source modules are now staged (not committed) for
  the source-backed verification retry; the failed suite is not a green
  receipt. → packaging

- 2026-08-06T00:00:00Z — The formatter-equivalence fixture, corpus-browser
  regeneration, generated-tour drift regeneration, deterministic tour (153.21
  seconds), and the tour committed-document drift test (38.17 seconds) all
  passed synchronously. The subsequent full serial workspace rerun reached the
  unrelated `cix-run::unit::closed_root_snapshots_cover_claims_dirs_materializations_and_modes`
  failure: its checked-in expected unit retains `TemporaryFileSystem=/cache:ro`
  while the unchanged runtime generator emits `ReadWritePaths=/cache`. This
  track does not touch `cix-run`; the stale fixture is a baseline/integration
  wall and the suite's exit was 101, so it is not claimed green. → integration

- 2026-08-06T00:00:00Z — Integrated current `origin/main` at `d55c0978`, which
  updates the missed closed-root `/cache` fixture to `ReadWritePaths=/cache`.
  The staged implementation was preserved through a named temporary stash and
  reapplied without conflicts. Next: run the full serial workspace suite with
  the requested `.gate-exit-workspace` recorded-status receipt, then commit the
  clean track branch if it records zero.

- 2026-08-06T10:35:26Z — Re-read CIP-110 before commit and corrected the
  filesystem primitive so directory identity is type plus sorted children,
  without directory permission bits; regular-file executable bits remain
  semantic. Added the directory-mode regression. The requested captured
  `devenv shell -- cargo test --workspace -- --test-threads=1` receipt wrote
  and was value-checked as `0` after this correction. `cargo fmt --all --check`,
  `cargo run -p cix -- fmt --check examples`, and warning-denied workspace
  clippy each exited 0. The structural shared/interior-mutability audit found
  only existing justified sites; this track adds none. `git diff --cached
  --check` exited 0. Next: commit the scoped implementation; no corpus locks
  were regenerated.

### FRICTION

- 2026-08-06T10:35:26Z — The declared focused VM command, `devenv shell -- nix
  run .#progressive-vm-check`, exited 1 before any scenario ran. Current main's
  `closedroot-audit.nix` asserts that its audited/downgraded inventory covers
  the corpus, but main contains 32 cases and the audit lists 30. This track
  does not touch the audit registry; the full workspace receipt is green, but
  the VM gate cannot be claimed green. → integration

- 2026-08-06T10:35:54Z — Committed the reviewed CIP-110 implementation
  (`feat: make Cixfile keys format-neutral`). The temporary
  `.gate-exit-workspace` receipt was value-checked then removed; no merge or
  push was performed. Next: hand off the clean committed branch, with the
  upstream VM inventory wall explicitly retained above.
