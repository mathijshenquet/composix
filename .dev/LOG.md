# composix work log

## 2026-07-30 (track/items complete)

- Merged work: none in this worktree. Completed D40/D41 on `track/items` in `e25fa5f` and
  `58a0f51`: hard ITEM rename, per-item TAKE plucks and v4 bare manifests, persistent
  build CACHE plus `--no-cache`, multi-item tagging, v1–v4 runner compatibility, D41
  legacy diagnostics/compose selector removal, all example migrations, and the real
  two-item proj1 build/run tour.
- Decisions: implemented existing D40/D41 without amendment. Item-scoped relative PATH is
  resolved against the item store root end to end; multi-item tagging uses the current
  `cix-index` API and this branch does not modify `crates/cix-index`.
- Verification: the full fmt, warning-denied workspace clippy/test, and
  `nix build .#checks.x86_64-linux.vm-dogfood --no-link` gate passed. This includes proj1's exact item
  listings, v4 manifests, worker-only invalidation, warm memo reuse, byte-identical clean
  `--no-cache` rebuild, and the tour's API run/curl. Exact focused generation/repro commands
  and the one transient user-systemd retry are recorded in `crates/cix-cixfile/LOG.md`.
- Open with Mathijs: none. Open for agents: independently verify and merge `track/items`;
  coordinate with concurrent `track/index2` rather than resolving its branch here.

## 2026-07-30 (the compose tree round — D40 go + D41–D46)

- Big design session with Mathijs, dialog-driven; full story in **docs/compose-tree.md**
  (the working paper), decisions recorded as D40–D46 in design.md. The arc: ocimport
  post-mortem (verdict stands: don't merge; parser reusable for future cix migrate) →
  dependency map (spec+run is nix-thin by design) → escalation-ladder path (packs →
  compose → side-by-side → long-running scenario tier) → netns read: egress renamed
  `outbound` (capability polarity kept — zero-machinery-by-default), `network: host`
  escape replaced by pod-ness-as-optional-property → composite tree (his fleet infra as
  the worked case; slice tree = resource axis, netns = flat at pod-claiming nodes,
  nearest-pod-ancestor) → host = one profile with tracked refs (granular *change* via
  tags/locks, atomic *rollback* via generations, semantic undo = tag push) → manifest/
  compose merge question dissolved by his devil's advocate: **item = one service**, bare
  def-node manifest v4, one tree grammar over two artifact kinds → ref/lock semantics
  unified (refs always name:tag, operative lock at deployer, repin deliberate, no
  ranges, override = evidence-gated cargo-[patch] future) → **index re-founded**
  (per-name content-addressed tag tables, CAS name pointer, history chain, advisory
  yank, name-level auth, signing = table hash) → computable composes (publish-time $tag
  expansion; monorepo bulk publish).
- D40 go given via "spec het maar precies en laat het bouwen" (covers the ITEM/TAKE/
  CACHE design he tasted directly); veto window open as always — everything is on
  track branches.
- Launched two codex agents: **track/index2** (terra, .worktrees/index2, D45 index
  re-founding, spec .dev/specs/track-index2.md) and **track/items** (sol,
  .worktrees/items, D40+D41 ITEM/TAKE/CACHE + manifest v4 + proj1 gate, spec
  .dev/specs/track-items.md). Items-track is fenced off crates/cix-index (index2 owns
  it). Launch lesson: codex must run outside the Bash sandbox (read-only fs kills its
  app-server init).
- Next wave (after these merge): the tree grammar itself in compose (D42/D43: nesting,
  pod-ness, publish/bind, path-based naming), then parametric publish tooling (D46),
  then the netns realization (compose-netns.md mechanics at pod-claiming nodes).
- Open with Mathijs: none new — D40–D46 recorded per his go; micro-fix policy nuance
  from 07-29 still unvetoed. Open for agents: the two running tracks; independent
  re-verification before merge as always.

## 2026-07-29 (track/tourfix correction round 1)

- Merged work: none in this worktree; corrected the old-systemd `Unknown assignment: PrivatePIDs=yes` tour leak on `track/tourfix`.
- Decisions: no new design decision. `cix debug` retains the raw old-systemd line in its captured failure diagnostics for fallback classification, but does not stream it separately; its existing loud D13 warning owns the human-facing explanation. Tour normalization removes the full assignment-line class, as it already does host-varying fallback detail and the optional systemd-run description prefix.
- Verification: explicit tour regeneration, tour determinism/drift, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` passed. The generated pages are unchanged on the current systemd ≥257 host; synthetic coverage proves old-host output normalizes identically.
- Open with Mathijs: none. Open for agents: merge the committed `track/tourfix` branch after independent verification.

## 2026-07-29 (night: CI-green saga; D40 pending)

- RUN v0 on CI took five rounds, each a distinct host-variance class, all catalogued:
  (1) runner missing bwrap -> apt install; (2) apparmor-confined bwrap kills loopback-up ->
  terra found bwrap's lo-setup is fatal-by-design (honest no-commit), redesigned to a
  seccomp inet/packet-deny fallback tier (sol; kernel-level filter tests; covers stock
  Ubuntu 24.04 desktops too); (3) runner is a hybrid (userns probe passes, uid-map denied
  inside) that satisfies neither tier -> CI runner unrestricted via sysctl+profile removal,
  CI tests tier 1, tier 2 covered by seccomp tests; (4) degraded-warning-pair PRESENCE is
  host-specific (permissive kernels never degrade) -> pair stripped entirely from tour;
  (5) orchestrator's own hand-fix failed fmt -> the full agent gate applies to the
  orchestrator too. Main green incl. VM job.
- Policy nuance adopted (pending Mathijs veto): micro-fixes (<~10 lines) inside an active
  verification loop may be orchestrator-direct; full gate (fmt!) applies.
- Discussed with Mathijs: dockerfile->cixfile auto-migration direction (trace-classified
  FETCH/RUN; bottleneck = distro-package -> nixpkgs-attr mapping; future, folds into cix
  migrate); D40 package designed and PENDING his go: OUTPUT (declared plucks, multi-item
  Cixfiles - his multistep insight) + CACHE (advisory per-step dirs outside key and
  snapshot; docker cache-mount analogue; enables ecosystem-incremental builds, bounded by
  sampled clean rebuilds) + proj1 as gate.

## 2026-07-29 (evening: RUN v0 lands)

- Merged track/inspect (terra): cix inspect both worlds + ls -l SYSTEMS column; verified
  live in both worlds independently. Merged track/runv0 (sol): FETCH/RUN directives,
  bubblewrap offer-only sandbox, memo+pin sections in Cixfile.lock, ${build}, projB +
  projB-chef; tour conflict resolved at merge (build-with-run=12, inspecting=13).
- Independent verification of runv0: 17 suites; chef selectivity PROVEN by hand (real src
  edit -> cook memo-hit, only final RUN re-ran; two earlier sed attempts silently matched
  nothing - always verify the edit landed); e2e run+curl of the RUN-built binary under full
  hardening. Honest finding: forced re-execution of the same RUN key gives a DIFFERENT
  snapshot hash (cargo .fingerprint noise) while the binary inside is byte-identical -
  D39.1 layer-ruis confirmed at v0 granularity; memo consistency holds, re-execution
  byte-determinism does not (sol's tour showed miss->hit, not re-execution determinism).
- Design follow-up for Mathijs: ${build} ships the whole final workdir snapshot, so items
  carry cargo bookkeeping (bloat + the nondeterminism lives exactly there); candidate fix =
  prune item assembly to referenced subpaths (or declared outputs). Also queued: netns
  proposal (docs/compose-netns.md) awaiting his read; FETCH-prelude fence rejected as YAGNI
  (positional rule does not even cover the abuse class).

## 2026-07-29 (afternoon: exec-era closes, RUN-era opens)

- All tracks landed and CI-green on the runner host: D33 manifest rename, D34 exec/debug
  (+1 sol correction), D35 part-1 ledger round, D36 PrivatePIDs (+empirical addenda: Ubuntu
  userns fallback observed live; node-as-PID-1 ignores SIGTERM = init-shim evidence), D37
  restructure (pack/compose/build + withSpec x2; +1 terra correction: listenfds cannot be a
  Cixfile per D29, restored as withSpec) and two tourfix rounds (host-variance classes:
  which properties a user manager rejects, and which properties old systemd knows).
- Build-story arc with Mathijs: BUILD rust (Variant A) specced and started (sol), then
  paused on his design doubt; engine-contract model designed as alternative; superseded by
  his RUN hypothesis -> D38 recorded, spike executed same day (sol) and PROMOTED: stable
  traced closures across cargo-chef/go/pnpm/uv with an ecosystem-agnostic harness. Engine
  contract will not be built; buildtool worktree parked (716 uncommitted lines preserved).
- Open with Mathijs: RUN productization design round (the three problems in the spike
  report: non-store input granularity, sound observer, writable layers/nondeterministic
  realisations); compose v1/netns round (W3) still queued; naming cixpack working title.
- Process: independent re-verification caught 4 real defects today (sol exec resolver +
  overclaimed transcripts, terra listenfds deletion, 2 CI-only tour drifts). Model table
  updated in nix-config accordingly (terra tally now has one miss).

## 2026-07-29 (part-1 ledger round)

- D35 recorded (Mathijs's docker.md part-1 review): signing scoped (content ✅ now, entry
  signing ⏳ publish-era), image lifecycle = tag lifecycle + nix GC (no gc machinery, hint
  only), mirrors refused (substituters for bytes, HTTP infra for the index, redistribution
  gated on entry signing), cix inspect designed 🔶 (du parked; docker prior art: system df),
  docker manifest = no verb (native per-system entries; ls -l systems column still owed).
- Ledger rows flipped accordingly (+ new registry-mirrors row). Naming noted: cixpkgs = the
  catalog (working title), artifact word stays "item"; "cixpack" floated as working title.
- New ❓ for Mathijs: PrivatePIDs=yes in the generator (systemd 257+) — real pid ns per
  service, but the app becomes ns-PID 1 (docker's zombie-reaping/tini problem).

## 2026-07-29 (track/exec correction round 1)

- Merged work: none in this worktree; corrected `track/exec` after independent verification
  exposed empty-Environment command lookup and overclaimed namespace isolation. Exec/debug now
  resolve shells and bare commands through recorded/generated PATH followed by `/usr/bin:/bin`.
- Decisions: amended D34's empirical wording. Exec compares namespace identities and joins only
  unit-private handles; the nginx port fixture has a private mount namespace but shares
  PID/network/IPC/UTS with the host. Its process listing is therefore a host view, not proof of
  PID isolation.
- Verification: focused tests, full workspace fmt/build/clippy/tests, deterministic tour/drift,
  root and user live nginx debug, default/root live exec against literal `Environment=`, and the
  NixOS VM dogfood check all pass. Exact commands and transcripts are in
  `.dev/specs/track-exec.LOG.md`.
- Open with Mathijs: none. Open for agents: merge the corrected committed `track/exec` branch
  after independent verification.

## 2026-07-29 (track/exec close)

- Merged work: none in this worktree; completed and verified `track/exec` for D34. `cix debug`
  runs a generator-identical fresh sandbox with shell/one-shot entrypoint override, while
  `cix exec` selects live units, reconstructs their recorded environment, directly joins five
  namespaces, and defaults to the runtime UID/GID with explicit `--root`.
- Decisions: no new design decision. Implementation realizes D34 with direct `setns(2)` +
  fork rather than an `nsenter` dependency; D13 supplies both verbs' loud user degradation,
  and D31 supplies the shared PATH shell resolution.
- Verification: workspace fmt/build/clippy/tests, deterministic tour/drift, root/user live
  debug+exec demos, and `nix build .#checks.x86_64-linux.vm-dogfood` all pass. Exact transcripts
  and repro commands are in `.dev/specs/track-exec.LOG.md`.
- Open with Mathijs: none. Open for agents: merge the committed `track/exec` branch after
  independent verification.

## 2026-07-29 (new session)

- Bootstrapped repo-level agent context: AGENTS.md (shared map — where truth lives, env,
  conventions, session-close ritual) + thin CLAUDE.md (@AGENTS.md + orchestrator notes:
  start ritual, decision queue, delegation policy: codex-exclusive implementation —
  luna=rote, terra=tight spec, sol=taste/on-the-fly decisions).
- Design round with Mathijs on the nature of the baked spec: D33 recorded — baking is
  nix-correct (load-bearing interface ⇒ hash-covered; nix-support precedent; gap filled =
  eval-free distribution of the runtime contract). File renamed cix-spec.json →
  cix-manifest.json, key cixSpec → cixManifest (spec = schema, manifest = baked instance).
  D31 addendum: toolbox-LINK refused; cix exec reconstructs env from the manifest; PKG
  rationale corrected (borrowed magic + half-coverage, not "new magic").
- Merged track/manifestrename (terra, clean run; gate re-verified independently: tests green,
  residue scan clean, tour regenerated). Not yet pushed.
- cix exec designed (nsenter-join of unit namespaces, env from unit's recorded Environment,
  default = service uid, sh fallback chain, --user = loud no-join degraded). Three decisions
  parked with Mathijs: confinement fidelity (no seccomp/cgroup join — operator surgery per
  D20a/b), default-user choice, shell fallback. Implementation (sol) blocked on his verdict.

## 2026-07-29 (session close)

- Compose v0 merged: the fourth part works (schema/check/diff, lock, profiles, activation,
  up/down/rollback, unix edges, listener binds; 3-tier fd-only demo verified independently).
- Cixfile language finalized through Mathijs's simplification arc: D31 PATH (explicit, no
  magic), D32 PKG scrapped → `${pkgs.attr}`, required `FROM <full-flakeref> AS <name>`
  (registry names refused, WITH rejected, overrides = .nix escape hatch). Per-input
  Cixfile.lock.
- Meta loops all fresh: README, 3-column ledger reconciled (+section 6 post-compose), tour at
  10 scenarios covering all four parts, CI green incl. VM job (fixed three env classes:
  missing user manager → linger; cold-store nix progress → normalize; systemd description
  prefix → normalize). Tour hermetic vs foreign units (decoy regression test).
- History rewritten (filter-repo) + force push: all private identifiers scrubbed from all
  refs; verified clean across 146 commits.
- Examples: 6 services + dstyle + buildshape + compose stack. OCI import: prototyped,
  verdict distraction, branch preserved.
- Process lessons: codex launches must be bare (memory saved); agents' "green" claims need
  independent re-verification (lock-fixture staleness caught at merge).
- Open with Mathijs: D31 toolbox-LINK addendum y/n; model table update (terra exceptional
  all session); naming (cixpkgs/pack); remaining ledger ❓s; compose v1 backlog (netns,
  scale, health, secrets, reconciler daemon).

## 2026-07-28 (night)

- D22 v3 filesystem projection: items are sparse rootfs fragments (mounts field, deny-list,
  stress-verified: ro shadowing, symlink escape blocked, 25 mounts). /app removed entirely;
  examples on native paths; sibling COPY files replace verbatim heredocs (nginx Cixfile is now
  7 docker-readable lines).
- Networking direction D23-D27 recorded (composite netns, SocketBind enforcement, caps tier,
  pluggable networks, talks-to). dstyle track (sol) proved the D25 tier live and produced 3
  ranked proposals: unix edges w/ per-edge groups, a listeners contract distinct from ports,
  socket-activated publish. Awaiting Mathijs review.
- compose-formats doc (sol): TOML recommended (data-only, strict), Cixfile-DSL as
  evidence-gated challenger, YAML/nix-lite rejected with reversal conditions. Awaiting verdict.
- OCI import prototyped (sol): real nginx/redis images ran hardened via RootDirectory, but
  verdict = distraction (second runtime model); branch track/ocimport preserved unmerged;
  ledger updated; cix migrate kept open.
- buildshape (sol): generic stub of the real rust+frontend flake shape (privacy-audited twice)
  + docs/cixfile-build.md — BUILD half judged: Variant A (inline minimal magic) first, plugin
  system (B) behind a 5-point evidence bar.
- Open with Mathijs: compose TOML verdict, dstyle proposals, docker.md remaining questions,
  model table update.

## 2026-07-28 (evening)

- Spec v2 amendments: D21 env de-typing (Mathijs's YAGNI push), D22 /item stable mount
  (dissolves the ${self} templating problem; empirical: ExecStart resolves pre-namespace so
  argv stays store-path-based). Both implemented (terra) and merged.
- Cixfile v1 designed (docs/cixfile.md with honest Dockerfile comparison) and implemented
  (sol): cix-cixfile crate, parser, nix codegen, Cixfile.lock, cix build -t; both examples as
  Cixfiles, verified e2e independently. Doc/design reconciled post-merge (heredocs interpolate;
  COPY verbatim; LINK'd executables).
- docker.md: adversarial audit (sol; receipts, residuals, honest gaps) then curated by Claude
  after Mathijs's "LLM slop" feedback — scope stated once, four actionable gaps, Evidence-we-owe
  list, 86→56 open questions.
- Tour: split into per-scenario pages + run/ps scenario (terra); fixture riff collapsed to one
  subshell command (terra; nix store add stdin stores a /proc symlink, so file-based one-liner).
- VM dogfood check (flake + NixOS test) merged; runs both examples as root in a disposable VM.
- Known chore: tour run-scenario reads system-manager state via cix ps → drift-flaky when a
  system cix-run.slice lingers; should filter to user manager.
- Terra: 6 strong runs today (one spec-induced port-race defect, self-fixed). Sol: deep on
  systemd/nix walls; skipped commits once (fixed with explicit gate). Luna: 1 good mechanical
  sweep. Model table update pending Mathijs.

## 2026-07-28 (continued)

- Design rounds with Mathijs on claims → publications → final form D17 v2: cix serve exposes the
  bare tag DB, no root_url arg, qualification IS reachability; qualified tag targets error;
  publish deferred until it means "ask a server" (push-shaped, server-side authz). D18 v2: one
  content-negotiated URL space (vnd.cix media type), /v1 dropped, the ref is literally a URL.
- Merged track/run (sol): spec parse/validate, golden-tested unit generation, cix run/ps; live
  --user demo verified. Empirical finding: user manager lacks mount-ns bind remapping
  (PrivateUsers path EOPNOTSUPP); degraded mode drops cap controls loudly.
- Merged track/litdoc (terra): tests/tour.rs harness + docs/tour.md (drift-checked, determinism
  test). Second strong terra datapoint.
- Launched track/web (terra): serve refactor to D17v2/D18v2.
- Merged track/web (terra, 3rd strong datapoint): bare-tag serving, negotiated URL space, HTML
  pages, conneg pull client; verified via tests + demo.
- Public launch: README, MIT LICENSE, docs/index.md; repo flipped public; GitHub Pages live at
  mathijshenquet.nl/composix (custom domain; also …github.io/composix). Launched track/tour2
  (terra): serve/pull/refresh scenarios for the literate tour.
- First-contact dogfood (Claude, by hand): system mode was broken for any ported service —
  RestrictAddressFamilies used nonexistent `+` merge syntax (golden tests encoded the bug);
  root PATH lacks nix; port-env model can't express env-blind apps (nginx).
- Merged track/tour2 after correction round (terra): parallel-test port race fixed.
- Merged track/dogfood (sol): nginx AND postgres run e2e under full hardening (sudo demos
  verified independently). Deep fixes: idmapped role-dir binds on systemd 257, nix-free store
  path resolution. nss_wrapper for postgres NSS; entrypoint-pattern initdb. Spec boundary
  proposals harvested in crates/cix-run/LOG.md → agenda for spec v2 design round. Process note:
  sol skipped its commit step; orchestrator committed on its behalf.

## 2026-07-28

- Project kickoff, design phase. Context dump from Mathijs: 4 parts (index, spec, compose,
  Cixfile). Claude (fable-5) is design partner + agent orchestrator.
- Gating decisions taken (see DESIGN.md "Decisions so far"): Rust; system systemd root-managed;
  index and spec+run built in parallel by agents; Cixfile Dockerfile-ish with .nix escape hatch;
  compose surface language deliberately open pending prototyping.
- Wrote DESIGN.md v0: index CLI/API + local tag DB (symlink farm as gcroots), spec schema v0 +
  unit-generation mapping, cix run semantics, compose mechanism (resolve → lock → build →
  activate, per-composite nix profiles), Cixfile positions. Open questions marked O1–O3.
- Next: Mathijs reviews DESIGN.md (esp. O1 push-vs-serve, O2 serve --with-store, O3 dir model);
  then repo bootstrap (cargo workspace + devenv) and two agent tracks in worktrees.
- Round 3: O1 → D12 (docker-style self-describing refs). Design feedback round with Mathijs:
  D13 (cix run --user degraded dev mode), D14 (per-system index entries), D15 (spec rejects
  unknown fields), D16 (baseline parts 1–3 is the nix-native product; Cixfile is the adoption
  bridge; composix.lib.withSpec as early rung). Bootstrapped cargo workspace (cix bin +
  cix-common/cix-index/cix-run) + devenv; CLI surface stubbed and compiling. Published private
  repo github.com/mathijshenquet/composix. Wrote specs/track-{index,run}.md; launched codex
  agents in worktrees .worktrees/{index,run} on branches track/{index,run}: index → gpt-5.6-terra
  (epsilon exploration, mechanical-leaning track), run → gpt-5.6-sol (systemd nuance).
- Round 2: O2 resolved → D10 (--with-store in MVP; `nix copy --to file://` + static serving is
  ~free). O3 resolved → D11 (app-path dirs model, docker VOLUME-like; Mathijs's push, my hybrid
  dropped). O1: serve-only agreed; wrote out the docker-style self-describing ref model (identity ≠
  address ≠ socket; docker disambiguation rule; publish = tag into namespace) vs git-style named
  remotes — awaiting Mathijs's pick.

## 2026-07-29 (track/restruct close)

- Completed D37(a+b): service examples now live under `examples/pack/`, buildshape is
  `examples/build/proj1`, and `examples/README.md` documents the Cixfile → withSpec → plain
  Nix adoption ladder. `dstyle/` and `LOG-examples2.md` remain untouched as the archive.
- Added `nix/lib.nix` and exported `lib.withSpec`; Redis demonstrates the helper, including a
  Nix build + `cix` parser test and live TCP/Unix-socket demo. The compose stack now consumes
  that moved Redis pack through its tag/lock path.
- Verification passed: workspace Rust gate, regenerated/drift-checked tour, VM dogfood, moved
  nginx/redis live demos, compose integration demo, and active stale-path scan. Exact commands
  and transcripts are in `.dev/specs/track-restruct.LOG.md`.
- Open with Mathijs: none. Open for agents: commit `track/restruct` after final staged review.

## 2026-07-29 (track/runv0 close)

- Merged work: none in this worktree; completed D39 RUN v0 on `track/runv0`. Cixfile now has
  a linear COPY/FETCH/RUN snapshot chain, offer-only bubblewrap execution outside Nix eval,
  networkless memoized RUN, TOFU-pinned networked FETCH, `${build}`, and backward-compatible
  lock memo/realisation records.
- Decisions: no new design decision. Implementation follows D39’s no-tracer v0 and strict
  user-namespace refusal; the only host files admitted are FETCH resolver fixtures. The Nix
  package supplies bwrap/nix explicitly.
- Verification: workspace fmt/build/clippy/tests, deterministic regenerated tour/drift, clean-lock
  projB determinism, live build/run/curl/stop, chef source-edit selectivity, and
  `nix build .#checks.x86_64-linux.vm-dogfood --no-link` pass. Exact transcripts are in
  `.dev/specs/track-runv0.LOG.md`.
- Open with Mathijs: none. Open for agents: independently re-verify and merge the committed
  `track/runv0` branch.

## 2026-07-30 (track/index2 close)

- Completed D45 on `track/index2`: immutable deterministic per-name tag tables, locked CAS name
  pointers, current-table roots, yank/history/migration, and `cix index history`. Existing `ls`
  and `inspect` output shapes remain covered; the tour now truthfully shows table storage.
- Commits: `a698439` and `5fa2bee`. No new design decision; D45 is implemented as written.
- Verification: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test -p cix-index`, `cargo test --workspace`, regenerated/drift-checked deterministic
  tour, and `git diff --check` all passed. Exact repro commands are in `crates/cix-index/LOG.md`.
- Open with Mathijs: none. Open for agents: independently re-verify and merge `track/index2`.

## 2026-07-30 (track/composefallback close)

- Merged work: none in this worktree; completed the systemd 261 compose fallback on
  `track/composefallback`. The shared generator now drops only `PrivatePIDs=yes` when a
  versioned realization probe proves the DynamicUser persistent-directory combination
  unsupported. `cix up` names the affected unit/property/reason, and the generation manifest
  records the same degradation. Rootless compose diff reuses the active generation's decision.
- Decisions: no new design decision. The implementation applies D36's minimal hardening fallback
  and D13's loud-degradation doctrine. The minimal proven failure is
  `DynamicUser=yes + PrivatePIDs=yes + StateDirectory=`; `RuntimeDirectory=` does not reproduce.
- Verification: regenerated and reviewed the unchanged tour; workspace fmt, warning-denied
  clippy, full tests, `vm-dogfood`, and the new `compose-fallback-vm` systemd 261 check all pass.
  Exact repro commands and the upstream systemd issue draft are in `crates/cix-run/LOG.md`.
- Open with Mathijs: decide whether to file the drafted upstream systemd regression issue. Open
  for agents: independently re-verify and merge `track/composefallback`.
## 2026-07-30 (track/scenarios close)

- Merged `main`/`track-composefallback` into `track/scenarios` and completed the scenario tier. Lifecycle now captures the capability-dependent compose warning and verifies its `manifest.json` degradation record using a runtime-derived systemd version; the obsolete VM user-namespace sysctl experiment was removed.
- No new design decision: this consumes D36's loud degraded fallback. D43 and D44 remain the explicit scenario FRONTIER markers (pod networking / nested-composite `--update <edge>` respectively).
- Verification green: all five `scenario-*` VM checks, ignored index hammer, workspace fmt/clippy/tests, and explicit tour regeneration with zero `docs/tour` diff. Exact commands and current-truth assertions are in `nix/LOG.md`.
- Open with Mathijs: none. Open for agents: independently re-verify the committed `track/scenarios` branch before merge.

## 2026-07-30 (track/scenarios fixture gate close)

- Committed `c1ad836` (`Harden scenario fixtures`): bounded every scenario curl probe, repaired the DynamicUser Unix-edge socket permissions, bounded fixture client I/O, and corrected lifecycle state/cgroup and collision assertions.
- Verification after the fixture repair: lifecycle and side-by-side each passed their already-recorded 3/3 forced-rebuild series; update-repin, gc-survival, and observability passed their serialized VM checks; ignored index hammer, workspace fmt/clippy/tests, and explicit tour regeneration with zero `docs/tour` diff are green. Exact repro commands are appended to `nix/LOG.md`.
- Decisions: no new design decision. Open with Mathijs: none. Open for agents: independently re-verify `c1ad836` before merge.


## 2026-07-30 (track/blocks close)

- Completed D47 on `track/blocks`: Cixfile is now a backward-only graph of named FROM/FETCH/
  BUILDER/SERVICE/APP/ITEM binders; RUN and build PATH/CACHE are builder-scoped; TAKE and the
  ambient `${build}` name have migration-grade errors; unified COPY supports both explicit
  binders and the amended implicit local directory context.
- v4 manifests distinguish service/app/item without a version bump. Apps run as hardened
  transient oneshots and return their exact exit status; asset-only items are refused by
  `cix run`. Every example was migrated, and `examples/build/ingredient` proves an independently
  pinned top-level FETCH binder.
- Merged current `main` at `dc4e331`. Active docs, Docker/corpus ledgers, and executable tour
  reflect D47; the `nix/scenarios/**` ownership fence was not touched.
- Verification passed: workspace fmt, warning-denied clippy, all Rust tests, explicit tour
  regeneration/drift/determinism, proj1 selective/warm/clean rebuild, root VM dogfood, and the
  systemd-261 compose fallback VM. Exact commands are in `crates/cix-cixfile/LOG.md`.
- Open with Mathijs: none. Open for agents: independently re-verify and merge `track/blocks`.

## 2026-07-30 (track/sdbisect close)

- Completed the same-host NixOS A/B check on `track/sdbisect`. Stock systemd 261 and a systemd
  261 build with the `StateDirectory=` caller behavior from upstream `6431c34b8a84` reversed both
  fail the minimal DynamicUser + PrivatePIDs + StateDirectory unit with `226/NAMESPACE`.
- Decision: no new design decision. The candidate commit is not confirmed as causal; the upstream
  issue draft now records the negative A/B result. Exact command and VM evidence are in
  `.dev/sdbisect.LOG.md`.
- Verification: `nix build .#checks.x86_64-linux.sdbisect-revert-vm --no-link -L` passed (exit 0);
  the two-node test verifies the systemd version, patched PID 1, and failure transcript on both
  VMs. Open with Mathijs: decide whether a further upstream investigation is worthwhile. Open for
  agents: independently re-verify the committed `track/sdbisect` branch.

## 2026-07-30 (track/blocks final-main update)

- Merged main's concurrent corpus/systemd-bisect/D48 design work through `6e9a136`; no D47 code
  conflicts and no paths under `nix/scenarios/**`.
- Repeated the complete Rust, tour regeneration/drift/determinism, dogfood VM, and compose
  fallback VM gate on that exact final snapshot; all passed. Exact command is in
  `crates/cix-cixfile/LOG.md`.
- Open with Mathijs: none. Open for agents: independently re-verify and merge `track/blocks`.

## 2026-07-30 (track/polish close)

- Merged work: none in this worktree. Completed the D48(b) hard rename in `55fdde9`:
  Cixfile `EGRESS`, manifest `egress`, migration-grade refusal of both old surfaces,
  and all examples, active docs, and generated tour migrated.
- Completed `.dev/specs/track-tour-proj1.md` in `603766d`: page 14 cats the D47 Cixfile
  and visibly proves cold miss → worker-edit warm miss with unchanged API → clean cold
  miss with byte-identical items, without timing output, before running the API.
- Decisions: implemented D48(b) without amendment. The cache marker stays in the
  builder snapshot rather than an item, so marker state can change while the clean
  rebuild still proves byte-identical shipped artifacts.
- Verification: fmt, warning-denied workspace clippy, all workspace tests, explicit tour
  regeneration/zero drift, drift test, determinism twice, vm-dogfood, and
  compose-fallback-vm all passed. Exact commands are in `crates/cix-cixfile/LOG.md`;
  test-created user/system units were cleaned up.
- Open with Mathijs: none. Open for agents: independently re-verify and merge
  `track/polish`.
