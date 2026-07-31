# Cixfile track work log

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
