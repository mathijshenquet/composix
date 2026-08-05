# composix work log

## 2026-08-05 (dispositions record; fleet sweep; CIP-100..102 adopted; build-args v2)

- **`cips/dispositions.md` minted** (Mathijs's ask): permanent record of
  blessed ledger-disposition batches; the 2026-08-04 batch recovered
  from git history at full per-item context (it had been compressed to
  a summary paragraph). open-questions trimmed to inbox+pointers;
  cips/README documents the mechanism. docker.md application track
  still queued.
- **Fleet swept**: 15 stale herdr workspaces closed (12 regen-* panes +
  cip94/regen2/buildfixes); all merged-track worktrees removed after
  blob-level verification that no staged content was unique (three
  "unique" blobs were byte-identical to main or older LOG states);
  orphan track-fhspaths (hollow checkout) and ~/regen-stage (2 GB
  staging; NOTES archived to ~/.cache/composix-regen-stage-notes-
  20260805.tar.gz) deleted; the six old .worktrees/* checkouts removed
  on Mathijs's "sweep gewoon". Disk: root 91%, /tmp inodes 83% — GC
  before the next VM-heavy gate.
- **CIP-100 env-equals, CIP-101 tmp-relocate, CIP-102 volatile-fetch
  adopted** (Mathijs). CIP-102's decision records the corpus survey
  answering his question: remaining EXPECT consumers are stable
  release artifacts; exceptions are exactly the sweep list (traefik,
  phpmyadmin) + echo-server's script FETCH to audit.
- **build-args v2 drafted** (the requested design round): full
  prior-art survey; v1's lock-pinned ARG rejected for state skew;
  recommendation is closed-matrix ARG (file declares the finite value
  set, CLI only selects, lock covers every cell and stays a pure
  function of the file) with typed generation as the open-ended escape
  (never string templating). Awaiting Mathijs's read.
- **Open for agents next**: implementation tracks for CIP-96..99 and
  now CIP-100..102 (env grammar sweep can absorb CIP-96+100 in one
  track; CIP-102 corpus sweep; CIP-101 small); docker.md disposition
  application track; wave-3 regen (directus, filestash); k8s wave 1;
  docker CANDIDATES expansion; CIP-93 stratification follow-up; tour
  ch4 inspect-remote weave-in. **Open with Mathijs**: build-args v2
  read; tour detail feedback.

## 2026-08-04 night (train closed: hand-holding tour, defect rounds, k8s axis, progressive selector, all-pretty JSON)

- **Coda (late night): CI-green after a seven-layer hardening tail.**
  Tour host-independence: cold-cache nix output, manager health-grade
  probe, unit-unload timeout + reset-failed (failed units never unload
  on older managers), nix version strings — all made
  environment-invariant with serial-verified receipts. Then the
  closed-root roster assert caught ten orphan pre-move context dirs
  that a broad `git add -A` had committed (old-depth ignore lapsed
  after the axis restructure) — caught on CI because no local full
  gate ran post-commit; exactly the designed defense. Draft round 3
  landed for Mathijs (env-equals, build-args, volatile-fetch v3);
  CIP-96–99 minted same-day (optional-env, granular-degradation,
  artifact-root-collision, lock-scale with the 4x coherence check);
  open-questions moved to cips/. Process lessons reinforced in memory:
  no pipes on load-bearing exits (bitten twice), no source edits in
  backgrounded chains (one edit silently lost), broad add -A after
  restructures is how debris ships.

- **Merged since the evening entry** (order): tourpolish (show_file
  typed blocks; ch6 listener from a real Cixfile); batch1 = corpusk8s
  (corpus/migrate/{docker,k8s} axis, 21 cases moved with history, k8s
  skeleton+CANDIDATES) + inspectremote (`cix inspect` on qualified
  remote refs, pull-grammar reuse — Mathijs's CLI-verb ask, resolved as
  extend-inspect); runfixes (CONFIGDIR path freedom via the STATEDIR
  mirror; localhost skeleton /etc/hosts with item override — two wave
  findings FIXED); buildfixes (EXPECT validated against the recorded
  pin before memo reuse; traced ENOTDIR as absent read — the real
  directus blocker, now clear; warm-root sequential COPY reconciles);
  jsonpretty ×2 (ALL JSON pretty — stdout and on-disk, alpha hash
  shifts accepted, Mathijs's fuck-the-hashes call); batch2 = tour4
  (hand-holding round) + cip93 leg 1 (derivation-diff VM selector:
  docs/corpus 0/13, code 13/13, loud, --full escape) + mastodon2 (six
  members on CIP-91 canon, full six-member probe + closed-root green).
- **The tour pipeline proved the mixed model**: sol restructure →
  orchestrator review (caught interpolated-argv canon leak) → two
  fresh-context cold readers (Docker-persona + Nix-persona, 306 lines
  of quoted confusion) → orchestrator editorial spec with verbatim
  load-bearing prose → sol execution → orchestrator prose pass on the
  branch. Chapter 1 now really runs the service rootless with declared
  normalization markers; every command typeable. Mathijs's detailed
  read is the remaining round.
- **Open-questions personally rewritten** (Mathijs): resolved items
  out, every entry self-contained, no bare D-numbers; six items
  promoted to drafts; **CIP-light format minted** (one screen:
  Problem/Proposal/Effort). Draft inbox now: granular-degradation,
  lock-scale, volatile-fetch, optional-env, artifact-root-collision,
  tmp-relocate — plus the contextualized disposition batch in
  open-questions.
- **Process minted today** (CLAUDE.md + memory): 10-minute heartbeat
  with active-PM questions (parallelize? PM harder?); df-guards
  (blocks AND inodes) before gates/fan-outs; CI watches never blocking;
  serial full gates with integration-batch gating for disjoint tracks
  (tree-identity verified at merge); codex swallow-verify; luna-first
  ladder (model table updated with luna 9/6?/5? on 18/18 cold
  translations + caveats); agents resolve their own semantic merges.
- **Open for agents next**: wave-3 regen (directus NOW unblocked —
  CIP-95 + ENOTDIR fixed; filestash via CIP-88/D70 verify); k8s wave 1
  from the new CANDIDATES; docker CANDIDATES expansion; volatile-fetch
  corpus sweep (behind its draft's adoption); CIP-93 stratification
  follow-up; tour ch4 inspect-remote weave-in. **Open with Mathijs**:
  the six drafts + contextualized dispositions; tour detail feedback.

## 2026-08-04 evening (the whole corpus on the new canon; tour rebuilt; CI-portability cascade closed)

- **Wave-2 regen MERGED** (`c3acdac`): all twelve remaining cases cold
  on the CIP-91/92 canon; the full 21-case corpus is now uniform.
  Two-layer warm/cold evidence rule born mid-wave (my GCs made rebuilds
  accidentally cold → cold-instability class surfaced: phpmyadmin
  EXPECT-refetch mismatch, echo-server/parse-server cold read-set
  divergences — ledgered, never re-pinned to hide). Luna final score:
  12/12 products, one false probe claim (nginx), honest walls
  (verdaccio), new findings: wallos artifact-root/role-path collision
  (→ language), adminer `__cix_unset__` optional-ENV smell, watchtower
  warm-root duplicate-COPY defect (caught by Mathijs mid-run;
  correction delivered, finding promoted).
- **Tour rebuilt as a 7-chapter new-user guide** (`1945070` + fixes):
  hello-first arc per orchestrator blueprint; orchestrator review
  caught the interpolated-argv canon leak in ch1/5; then a three-round
  CI-portability cascade: (1) CI's older user manager rejects
  PrivatePIDs → degraded fallback drops BindPaths → env-dependent
  probes (fixed; granular-degradation product item filed); (2)
  auto-width ps/stats tables measure ambient units → nondeterministic
  generations (fixed); (3) CIP-94 fetch drvs used bwrap → vanilla
  GitHub runners deny unprivileged userns (fixed userns-free,
  byte-identity verified with user.max_user_namespaces=0). CI green
  again at cip94fix (`b7f85b1`, rerun success). Environment lesson:
  GitHub-hosted runners (KVM yes per nix-installer, userns no, older
  user manager) vs devbeast (KVM no, userns yes) — portability across
  that matrix is the requirement; the local gate cannot substitute for
  CI's environment class.
- **CIP-94 buildCixfile milestone 1 MERGED** (`d542769`+fix): nix
  builds Cixfiles without cix; byte-identity flake check vs --cold;
  per-FETCH snapshotNarHash FODs; loud FHS/milestone boundaries.
- **In flight**: tour polish (sol: show_file affordance per Mathijs,
  ch6 listener from a real Cixfile, read-through nits);
  corpus/migrate/{docker,k8s} restructure + k8s skeleton (terra,
  Mathijs's direction).
- **Ops incidents**: /tmp tmpfs inode exhaustion (probe litter;
  CLEANUP.md sweep section, relocate-to-cache open item) and root-disk
  fill from unshared full-gate VM closures (267 GiB GC'd; df-guard now
  precedes every full gate). Process: 10-minute heartbeat while
  waiting (Mathijs; memory-recorded); CI watches never blocking.
- **Open for agents next**: the defect-fix round (CONFIGDIR, EXPECT
  warm validation, localhost, Not-a-directory, volatile-fetch
  normalization, warm-root duplicate COPY, granular degradation, lock
  scale, /tmp relocation, artifact-root collision, optional-ENV); k8s
  wave-1 + docker CANDIDATES expansion after the restructure; mastodon
  member regen; CIP-93 progressive-test design; directus/filestash via
  CIP-95 + Not-a-directory. **Open with Mathijs**: nothing — the draft
  inbox is empty.

## 2026-08-04 (track/tourfix main-CI repair)

- Merged work: none in this worktree; completed the `track/tourfix` repair for
  CI-only tour drift. Chapter 1 retains its canonical nginx Cixfile and
  manifest receipt, but the mount-dependent `nginx -t` execution is now
  clearly labeled non-executed system-manager prose pointing to VM dogfood.
  Chapter 5's boundary prose is manager-neutral, and the harness comment names
  CI's `PrivatePIDs=` rejection and resulting no-`BindPaths` D13 retry.
- Decisions: none. D13 and the filed granular-degradation defect remain
  unchanged. Two test-only assertions made stale by CIP-94 were aligned with
  its byte-bound eval-plan hash and complete FETCH snapshot hash; no cix
  implementation behavior changed.
- Verification: fmt, canonical examples fmt, warning-denied all-target clippy,
  full serialized workspace tests, explicit tour regeneration with zero
  drift, and explicit twice-rendered tour determinism all pass synchronously.
- Open with Mathijs: none. Open for agents: independently gate and merge the
  committed `track/tourfix`; the orchestrator layer still owns the full flake
  matrix.

## 2026-08-04 (corpus professionalization day: the loops built, CIP-91/92/93 same-day, wave-1 cold regen landed)

- **The corpus loops** (from Mathijs's rapid-fire corpus review):
  docs/corpus.md "How this corpus is maintained" — per-case GAPS.md
  (open-vocabulary arrow routing), corpus→CIP→feature staleness rule
  (AGENTS.md extension), cold-regeneration loop with Generated:
  provenance headers, later-automation intent. Merged: track/corpusgaps
  (sol; 21 routed ledgers, Fidelity/Evidence two-axis regrade,
  migrate.md parity/layout/version-binder addenda) + track/browser3
  (terra; every subordinate file rendered, context.files manifests,
  CSS-only twin tabs, gap panels, real-parser rot guard).
- **Adopted same day**: CIP-91 artifact-import (universal IMPORT,
  store-aware COPY link-by-rule + two static materialization triggers,
  LINK deprecation alias, STATEDIR-at-native-paths; spike clean, sol
  implemented, gate green `00078d9`); CIP-92 port-protocols (systemd
  `udp:443` single form + Docker-form hint, `cix build --file` sibling
  locks; terra `7b00e34`); CIP-93 test-pyramid (minimal e2e +
  progressive-tests amendment — change-keyed VM selection as design
  goal). Draft inbox restructured: cips/rejected/ + cips/deferred/
  minted; file-from REJECTED (YAGNI post-CIP-91, docs propagated);
  cixfile-build → rejected, compose-syntax → deferred; emit-nix v2
  adoption-ready (pure tier 1, cross-check split out with steelman
  recorded); fhs-interpreter iterated v3→v5 (trace-driven diagnostics +
  provide-the-FHS-paths via IMPORT lib-union + loader aliases; patchelf
  RUN stays taught escape; round-2 spike = ld.so search wiring vs
  RUNPATH shadowing; directus regen is the acceptance case).
- **Agent-open items closed**: netnsrace (sol) — NOT the suspected
  ordering race: 1s teardown budget killed `ip netns delete` under
  load; 10s stop budget on netns oneshots; 17/20→20/20 under identical
  contention (`c1d6c59`). adapterlive (terra) — pinger loss does NOT
  reproduce on pinned 257.6 nor 261; 7s retention assertion added;
  honest downgrade (`8ccf043`).
- **Wave-1 cold regeneration landed** (`54a9d36`): six dissolvable
  cases regenerated cold by gpt-5.6-luna in bwrap-lite staging dirs
  (regen-stage.sh; canon-blind; transcript-audited via codex session
  history), all 12 Cixfiles independently re-verified green, assembled
  by sol with fresh ledgers + probes. Luna 6/6 build-green with quality
  far above the table's guess — but ONE false probe claim (nginx
  "check.sh cix passed" did not reproduce: /var/log/nginx absent) →
  luna greens re-verify like everyone's. Caddy regen killed the probe
  toy (real config contract, four ports incl. day-old udp:443 syntax
  luna guessed unprompted — syntax-choice validation).
- **Product findings from the wave** (all in docs/open-questions.md):
  CONFIGDIR must-be-under-/etc at run while build accepts + docs teach
  path freedom (verified triple defect); no `localhost` in the service
  sandbox (skeleton /etc/hosts candidate); EXPECT not validated against
  recorded pin on warm builds (traefik's copy-pasted double EXPECT
  preserved in-tree as the living repro); unstable-API FETCH content is
  EXPECT-hostile; bare `Error: Not a directory` diagnosability defect +
  ~148k-line lock growth (fhsspike). Prompt findings → migrate.md:
  dissolved-twin contract, volatile-metadata normalization.
- **Process built**: regen-stage.sh cold staging; luna-first escalation
  ladder with failure-analysis-before-escalation; jail ladder decided
  (v0 transcript-audit → v1 user-separation → v2 bwrap; codex's own
  sandbox verified NOT read-confining); never-overlap-full-gates held
  all day (serial gate train through five merges).
- **Open with Mathijs**: emit-nix adoption (mint CIP-94 on his word);
  fhs-interpreter v5 (magic question dissolved by the path route; ld.so
  wiring spike next); the standing 14 dispositions + ARG re-marking.
- **Open for agents**: docs/cixfile.md full CIP-91/92 currency pass
  (LINK-first teaching throughout — queued); wave-2 regen (app-shaped
  cases + repro-pinning for the four evidence-gap cases); wave-3 (hard
  cases; directus gates on fhs v5 spike + Not-a-directory fix); the
  wave's product-defect fixes (CONFIGDIR, EXPECT validation, hosts);
  CIP-93 progressive-test design; nginx faithful probe red (log-path
  contract).

## 2026-08-03 early (day closed: board fully landed, GC done)

- **track/fetchself MERGED** (`e2402d1`, CI green): CIP-87
  self-observation rule with all four conditions, a/b adversarial
  regression, cold control green, two-prime workaround removed. The
  14.46s scare was an unlike comparison — steady-warm holds at
  **8.31s / 8.84s**, and the **no-op is now 0.07s** (parity with the
  raw upstream flake). docs/nix-build.md reports first-warm and
  steady-warm separately. ~5.9s of steady-warm is gitsitter's own
  .git/HEAD floor — upstream issue #24 filed; fixing it opens sub-3s.
- **track/thin2 MERGED** (`658e0df`, CI watch pending at write time):
  compose strata pass — directories.rs (426) + network.rs (247)
  extracted as pure moves, publish validation colocated, a shared-dir
  newline regression caught with an assertion. Gate r1 was starved by
  an OVERLAPPING full gate (VM boot timeout under shared bounded
  cores) — **operational rule: never run two full gates
  concurrently**; r2 solo green.
- **GC + roots audit done** (Mathijs's request): 1132 non-proc roots —
  795 censored other-user process roots (untouchable), 149 cix item
  roots (product-managed, kept), 117 devenv roots (mostly
  bittensor), 45 home-manager generations (left; `-d` not run
  unasked), plus composix worktree residue. Removed all 16 merged
  track worktrees + .worktrees/ergo, then `nix-collect-garbage`:
  **30,949 paths, 137 GiB freed** (disk 100%→71% across the evening's
  cleanups). ~/.cache/cix (12G) left untouched — warm workspaces live
  there; a trim is a fresh-session decision.
- **Board state**: every adopted CIP (75–90) + D70 implemented, merged,
  CI-green. Drafts awaiting Mathijs: emit-nix, file-from,
  test-pyramid. Dispositions batch + ARG re-marking still open with
  Mathijs — package as 14 yes/no lines next session.
- Remaining stray worktrees .worktrees/{cigreen2,crunchy,proj1?} are
  PRE-EXISTING (not from today) — successor: check their branches
  before touching.

## 2026-08-02 (the board-clearing day: every adopted CIP implemented; herdr C2; fable's first crack)

- **Merged, all CI-green** (order): obs/CIP-83 (`f4f39cc`); devices/CIP-78
  (`dc27687`) + devfix (PrivateDevices UserFull degradation — CI-only
  AppArmor class, `af88d71`); regrade (51 ribbons, Evidence column,
  `bb2c970`); CIP-reorg (85/86 retro-minted, cips/ to repo root,
  accepted/+draft/ split); corpusweb (one living corpus + side-by-side
  browser, `9823621`); health/CIP-79 (`f607a7f`); dirs2/CIP-82 leg 2
  (`9037d67`, terra ×3 — TWO fixture false-greens caught by the
  independent gate, then a real `cix clean` lifecycle fix); ergo/CIP-88
  (`81586c5`, ×3 — semantic merge seam + a workspace-env test race);
  overlay/D70 + trailing EXPECT + wallos on the three-line overlay form
  (`0402e05`); secrets/CIP-81 + `run --compose` (`857d185`); vmslim
  (scenario sweep **-53%**: teardown-dominated, DefaultTimeoutStopSec=1s,
  `527764e`); corpusstyle (authoring canon + browser v2 highlighting,
  `d8b463e`); closedroot/CIP-84 phase 1 (audit VM over 7 examples + 10
  corpus cases, 10 honest downgrades, `3540315`); tree1/CIP-85 leg 1
  (group nodes, path identity, per-path locks, root verbs, `e981beb`);
  netns/CIP-86 (`d82a4c5`, ×2 — full matrix caught a shared-surface
  regression); mastodon flagship (six members, shared-rw/readiness/
  secret/timer/logs receipts, audited under closed root, `13324f5`);
  hygiene-a+b/CIP-90 (clap boundary, env-free tests, `ps --json`, tour
  from one truth; `c92c9ed`+`e0ea60a`); thin1/CIP-89 leg 1 (assembler
  splits, compat deletions, 2000-LOC tripwire, `6567c5e`); tracefast/
  CIP-87 bar (**84.83s → 8.31s warm edit, capture complete** — terra ×2
  honest stops → sol frontier measurement → fable crack; `b5a52c1`);
  docstruth (`f94519c`); micro-fixes: hermetic store-prefix test
  (`daa7bdd`), fixed-width nonce (`3b0d4cd`), netns teardown flake
  (`f63ad1c`).
- **Decisions**: CIP-85/86 retro-minted (renumbering blessed; range need
  not be gapless); CIP-87 read-set keying + CIP-88 builder ergonomics +
  CIP-89 thinning + CIP-90 test hygiene drafted AND adopted same day;
  CIP-87 amendments: perf criterion (~9s green/crane floor), verify-only
  RUN memos (Mathijs in-session), FETCH self-observation rule with four
  load-bearing conditions (per-path hash, full-write-set apply,
  cold-replay precedence, same-memo scope); CIP-88 amendments: vendored
  dev-env replaces per-var minting, all sensible lock attrs, junk lint;
  trailing `FETCH … EXPECT`; file-first authoring canon. Drafts open:
  emit-nix (with tier-1b in-nixpkgs form + pure-build cut), file-from.
- **Process** (all recorded in CLAUDE.md/AGENTS.md/memory): /goal
  shorthand + drive-progress; herdr worker C2 + `herdr worktree` flow
  (unlosable completion signals, blocked detection, mid-flight
  re-prompts — replaced background codex exec after a lost-signal burn
  idled a slot ~4h); focused agent gates with the full matrix at the
  orchestrator layer only; receipts = synchronous exit statuses (terra's
  three detached-output false-greens); shared-state justification rule
  (inventory ran clean); ledger-currency rule; delegation cost rule
  (prompt vs fix); process autonomy with speed×correctness KPIs; gates
  bounded (nice, -j6 --cores 4). Escalation ladder proved: terra → sol
  → fable (first fable worker datapoint: exceptional; table updated).
- **The independent gate earned its layer**: seven real catches today
  that agent-local green missed (dirs2 ×2, ergo ×2, netns shared
  surfaces, mastodon audit inventory, hygiene-a workspace race).
- **In flight at close**: track/fetchself (CIP-87 self-observation
  rule) + track/thin2 (compose strata pass) — the final two board
  items; gates + merges follow the standing cycle.
- **Open with Mathijs**: the 14 one-line dispositions + ARG re-marking
  (docs/open-questions.md); emit-nix draft adoption; FILE…FROM draft
  adoption.
- **Open for agents**: netns activation race under load (recorded in
  open-questions); systemd-257 adapter-liveness retention; phase-2
  closed-root flip once the audit era proves out; D26/D27 named
  networks + talks-to; publish era; reconciler. Upstream: gitsitter
  issue #24 (.git/HEAD permanent-stale floor, affects all build
  routes).

## 2026-08-02 closed-root track complete (CIP-84 phase 1)

- **Completed on `track/closedroot`**: opt-in `--closed-root` compilation for
  run and compose with an empty per-unit root, whole-store read-only bind,
  D22/role/claim projections, exact synthetic NSS, `PrivateUsers`, egress DNS,
  journald compatibility, user-mode parity/fallback, and explicit shell/env
  dependency diagnostics. Default behavior remains unchanged for phase 1.
- **Audit and ledgers**: the exhaustive VM roster runs all seven pack members
  and ten reproducible migration contracts with their native probes and sealed
  root/forced-teardown assertions. Ten non-runnable or non-reproducible corpus
  cases are explicitly downgraded; Docker and corpus ledgers distinguish the
  receipts from desk claims.
- **Decisions**: implemented adopted CIP-84 without a new decision. The work
  follows D22 projections, D13 loud user-manager degradation, D48 identity and
  egress vocabulary, D58-style explicit environment dependencies, and D63
  anonymous-run GC lifetime. Direct ports below 1024 are refused in closed
  roots because `PrivateUsers` capabilities cannot authorize host-netns binds;
  named listeners remain the privileged-port route.
- **Proof**: fmt, examples formatting, warning-denied workspace/all-target
  clippy, explicit corpus/tour regeneration, workspace tests, focused dogfood
  and closed-root VMs, and the exact final
  `devenv shell -- nix flake check -L` all passed synchronously. Exact receipts
  and the one caught fixture/preparation fix round are in
  `crates/cix-run/LOG.md`.
- **Open with Mathijs**: none for this track.
- **Open for agents**: independently re-run the full gate, then merge
  `track/closedroot`.

## 2026-08-02 health track complete (CIP-79)

- **Completed on `track/health`**: strict READINESS/LIVENESS manifest and
  Cixfile vocabulary; native notify/http/tcp readiness adapters and watchdog
  liveness pinger; bounded rollout failures; opt-in restart limits; structural
  compose ordering without health-conditioned graph semantics; and explicit
  rejection of removed v0 `health { exec, interval }` input.
- **Proof**: focused Rust, formatter, clippy, tour, and health-VM gates passed;
  the required final `devenv shell -- nix flake check -L` passed all 67 checks,
  including `scenario-health` under the full parallel VM load.
- **Ledgers**: updated cixfile, Docker-gap, corpus, migration, open-question,
  and generated-tour documentation to reflect the delivered CIP-79 boundary.
- **Decisions**: implemented adopted CIP-79 without a design amendment; health
  remains service-local, while compose health-condition edges stay refused.
- **Open with Mathijs**: none for this track.
- **Open for agents**: independently re-run the full gate, then merge
  `track/health`.

## 2026-08-02 night (CIPs 82–84 adopted; dirs leg 1 landed after a caught fix round)

- **Adopted**: CIP-82 dirs (claims model — every dir declaration is a
  claim; decorated roles = cix-satisfiable dispositions; undecorated
  `DIR` = operator-supplied, after DATADIR/CLAIM-mount/CLAIM-data all
  lost the spelling debate; overlay backing with FULL host mirror kills
  the D11 root restriction, alias branch, and state-N indices;
  lifecycle table normative; recreate refused; explicit idmap
  acknowledgment); CIP-83 observability (journald projection:
  LogExtraFields selectors incl. CIX_ITEM, cix logs/stats, exit-cause
  mapping with the 200–245 table, opt-in logNamespace — compose-only,
  first recorded CIP-77 exception); CIP-84 closed-root (mandatory
  hermetic RootDirectory, no rawdog; **whole-store ro** — Mathijs's
  reference-scanner argument beat closure-only binds; residuals:
  scanner-escapes → lint candidate, runtime-supplied paths →
  gc-coherence note; closure-binds/RootImage recorded as tightenings).
- **systemd.exec read integrally** (4381 lines, v257.9) at Mathijs's
  request: BindLogSockets dissolves the closed-root logging edge,
  PrivateUsers shrinks NSS to a three-line passwd, RootImage+dm-verity
  as store-posture tightening, LogNamespace/LogExtraFields powering
  CIP-83, JoinsNamespaceOf/NetworkNamespacePath confirming D43/D49,
  ExecPaths W^X candidate, exit-code table for cix debug.
- **Merged**: track/dirs (`7d69fd9`, terra ×2 — CIP-82 leg 1). The
  INDEPENDENT gate re-run caught two real bugs terra's green missed:
  226/NAMESPACE (bind destination uncreatable in the ro root — missing
  TemporaryFileSystem per top component) and EACCES on the D36 degraded
  fallback (ownership machinery not reaching the mirror). Fix round
  `c6b9367` repaired both + tests the degraded path; second independent
  re-run green; CI green. Re-verification convention: two catches today
  (this + the start EXEC fixtures).
- **In flight**: track/obs (CIP-83, terra). Ledger cleanup landed:
  open-questions.md rewritten post-wave, the three ledger errors fixed
  (dup mirrors row, LABEL→D54, corpus→CIP-79). Checked: ITEM in
  migrate.md is correct (D68 revived it; not drift).
- **Open with Mathijs**: the 14 one-line dispositions + ARG re-marking
  (docs/open-questions.md).
- **Queue**: obs merge → devices impl (CLAIM gpu/device + SHM,
  Immich/Frigate) → health impl (READINESS/LIVENESS prober; sol) →
  closed-root phase 1 audit gate → CIP-82 leg 2 (compose
  materializations, cix clean/purge) → D70 overlay universes + wallos →
  tourvm → hardening-audit pass (systemd.exec set).

## 2026-08-01 evening (the CIP wave: process born, 75–81 adopted, four tracks landed)

- **CIP process adopted** (Mathijs): docs/cips/, drafts by name, adopted
  by number continuing the D-sequence, v0 in-place amendments, Decision
  sections at adoption. Rules in docs/cips/README.md. New decisions go
  through CIPs; D1–74 stay citable.
- **Adopted**: CIP-75 timers (compose `schedule:` raw OnCalendar),
  CIP-76 devloop (`cix watch`, sync ❌ forever), CIP-77 run-unary-compose
  (run = compose with one anonymous member; translation-quality guard),
  CIP-78 devices (CLAIM vocabulary — GRANT renamed; `CLAIM gpu`,
  `CLAIM device /dev/x`, `SHM`), CIP-79 health (READINESS/LIVENESS on
  notify/watchdog; health graph banned; probe types http/tcp/notify),
  CIP-80 exec-naming (EXEC→START, SETUP→START_PRE), CIP-81 secrets
  (SECRET/LoadCredential file-only; fetch tokens with direnv-shaped
  host-side consent — the lock-whitelist died in a 4× turn-over).
- **Merged implementations** (terra ×4, each gate independently re-run):
  track/claim (`d2d5033`), track/watch (`2733649`), track/timers
  (`322b19c`), track/start (`2c44ecc`). Fingerprints d74→d78→d80.
- **Drafts open**: dirs (r3 + a chat round that will become r4: dir
  declarations unify as claims; overlay-backing
  `/var/log/<unit>/<declared-path>`, kills the D11 root restriction and
  state-N indices; DATADIR dies in favor of CLAIM mount), closed-root
  (mandatory hermetic RootDirectory per Mathijs — no rawdog dial;
  store-posture and NSS mechanism open).
- **CI incidents**: timer gc-root test raced (PartOf-propagated cleanup
  is async; fixed with wait_until_succeeds, `b79e441`); start sweep
  missed two inline EXEC fixtures in build.rs lib tests — terra claimed
  workspace tests green while they failed deterministically (THIRD terra
  false-green; independent re-runs remain non-negotiable). Also: my own
  gate command piped through tail and swallowed the failure —
  gate-scripts-fail-loud applies to the orchestrator too.
- Ops: devenv.lock now tracked (Mathijs); stale conflict marker removed
  from cixfile LOG (`2f699b9`); inventory doc: docs/open-questions.md.
- **Open with Mathijs**: dirs r4 go/no-go (incl. host-mirror
  strip-question), closed-root draft read (§4.1 store posture, §4.4
  --user), the 17 one-line ledger dispositions batch.
- **Queue next**: health implementation (READINESS/LIVENESS prober —
  biggest; sol candidate), devices implementation (CLAIM gpu/device +
  SHM, Immich/Frigate dogfood), closed-root phase 1 audit gate after
  adoption, D70 overlay universes + wallos rewrite, tourvm.

## 2026-08-01 midday (fmt + leaks landed; CI-red root-caused; health design open)

- **CI red on main root-caused and fixed** (`547662b`, orchestrator
  micro-fix): vm-dogfood's in-VM `nix-store --gc --max-freed 1` (from
  `21a2fdc`) collects an arbitrary unrooted path; on CI's store-image VM the
  additionalPaths items are valid-but-unrooted until their `nix-store --add`,
  so the GC ate node-app-cix. Latent since the gc-root feature; armed by
  D72's item-hash reshuffle. Local 9p host-store mode hides it — which is
  why local full tiers stayed green against a red CI. GC exercise moved
  after the last add; CI confirmed green.
- **Merged**: track/leaks (`490b90d`, terra — FETCH probe snapshots
  chmod+close self-clean incl. failure paths, root cause read-only npm
  trees breaking TempDir Drop; measure-warm.sh honors TMPDIR); track/fmt
  (`cef6499`, terra ×2 — D74 complete: trivia-preserving fmt module,
  `--check` diffs, stdin, .gitignore discovery, golden+torture sweeps,
  examples reformatted as own commit, `cargo run -- fmt --check examples`
  in CI. Terra made an exemplary honest STOP at the lock-stability gate,
  surfacing a real D48a keying leak: COPY step keys contained physical
  `copy.source` text. Fixed by keying on parsed template parts + dst,
  CODEGEN_FINGERPRINT bumped d69-v1→d74-v1 to orphan old memos honestly;
  D69 wipe-proof re-held). Both gates independently re-run green (full
  tier).
- Canon nit for Mathijs: fmt also strips blank lines INSIDE blocks
  (defensible: blank line = block separator, but it was a silent choice).
- **Standing directive (Mathijs, recorded in memory): fill gaps
  autonomously; big design decisions stay joint; surface thin specs
  immediately. Until revoked.**
- **Open with Mathijs — health design (D48c amendment)**: ban the health
  graph (no `condition: service_healthy`, edges stay structural) in favor
  of k8s liveness/readiness hung on systemd natively: readiness =
  `Type=notify`/`READY=1` or ExecStartPost probe-await (rollout-status for
  free), liveness = `WatchdogSec` fed by app or by a cgroup-resident cix
  pinger (`NotifyAccess=all`), startup probe dissolves into
  `TimeoutStartSec`. Three questions pending: READY/LIVE vocabulary,
  parameter canon (EVERY/FAILURES defaults), graph-ban as explicit ❌ in
  docker.md.
- **Queue next**: D70 overlay universes implementation (+ wallos rewrite),
  tourvm, wave-two feature tracks; health after Mathijs's taste calls.

## 2026-08-01 morning (decompose + underlay landed — the warm loop is won)

- **Merged**: track/pinkeys (`5201d52`, terra ×3 — D69 complete; acceptance:
  byte-identical locks across a workspace WIPE between clean update-locks,
  independently proven; parse-server pins 7 consumed paths, volatile facts
  recorded not persisted); track/decompose (`f48bd7f`, terra ×2 +
  orchestrator gate-completion — cix-build crate exists, parser.rs
  2,741→thin mod over directives/machine/migrations/validate, index in five
  modules, spec.rs −690 via the v0 collapse; net −348 lines while adding a
  crate; torture snapshots byte-identical throughout); track/underlay
  (`aa682a1`, terra — D71 underlay semantics, warm-vs-cold fixture, docs).
- **The headline measurement, independently re-run on a clean harness:
  warm edit upstream 28.3s / crane 14.3s / cix 7.5s.** The scorecard now
  reads: authoring 17 vs 30 vs 38 LOC, cold 26.9s (best), warm 7.5s (best,
  2× over crane), no-op 1.13s (nix eval-cache keeps that one). Receipt
  dated in docs/nix-build.md.
- Ops finds during verification (queued nits): the D69 probe leaks its
  /tmp/cix-fetch-probe-* dirs (must self-clean, also on failure);
  measure-warm.sh hardcodes /tmp (should honor TMPDIR); tmpfs inode
  exhaustion twice root-caused (node_modules-class trees).
- **Queue next**: D74 fmt implementation (spec to write; after crunchy's
  parser landscape), D70 overlay universes implementation (+ wallos
  rewrite), probe-leak + harness nits, tourvm, wave-two feature tracks.
- Open with Mathijs: nothing blocking; the parser D-number question was
  resolved (banned), fmt CLI resolved (check subsumes diff).

## 2026-08-01 late (CI GREEN + D70–D74; crunchy landed; pinkeys in leg 3)

- **CI on main is fully green** (`fb0d7c5`) — first since 2026-07-30. The
  campaign fixed five named causes (gc-root product bug, host-dependent
  fixture, serve race, compose-fallback root registration, clippy
  unused_io_amount micro-fix). **New convention in AGENTS.md: track gates
  end with the full `nix flake check -L`** — the thirteen-merges-past-red-CI
  lesson.
- **Decisions**: D70 overlay universes (`FROM <flakeref> OVERLAY ./x.nix AS
  pkgs`; USING died — "a function call in a jacket"; getFlake stays
  evidence-gated); D71 the underlay (+retreat path: CACHE returns if it
  chafes; dial underlay→CACHE→prefix); D72 alpha manifest = version 0
  (schema moves freely until 1.0; D15 suspended); D73 decomposition round
  (cix-build crate split, index modules, spec v0 collapse, parser diet per
  analysis; complexity-monster house principle; addendum: NO D-numbers in
  user-facing diagnostics — stable doc anchors instead); D74 `cix fmt`
  (apply-by-default, recursive, `--check` prints diffs and exits 1 — check
  subsumes diff; `-` stdin; minimal canon v1; trivia-preserving printer).
- **Merged**: track/cigreen2 (`fb0d7c5`, terra + orchestrator micro-fix) and
  track/crunchy (`d20f866`, sol — 60-fixture torture corpus as permanent
  snapshot suite, did-you-mean suggestions, per-docker-directive guidance,
  D-number sweep, explicit accept-fixtures for the forgiveness boundary).
- **pinkeys (D69) in leg 3**: leg 1 flapped on snapshot paths in the lock
  (caught by the independent double-clean-update-lock proof); leg 2 moved
  snapshots to a stable-pin-keyed local cache but the probe still persisted
  npm's self-generated log path; leg 3 filters persisted volatile facts to
  later-consumed paths. Orchestrator acceptance: byte-identical locks across
  a workspace WIPE between two clean update-locks. Gate (incl. full tier)
  running.
- Parser analysis (sol, read-only): 403 machine / 715 directives / 186
  migrations / 760 helpers / 571 in-file tests → adopted into D73(d).
- Ops: /tmp cleaned per mtime (Mathijs-authorized; 31G + 125k inodes freed;
  ageq leftovers were the bulk); ENOSPC root-caused to tmpfs inode
  exhaustion; orchestrator shell-honesty lessons memorized
  (gate-scripts-fail-loud).
- **Queue**: pinkeys merge → decomposition round → underlay (+ nix-build.md
  re-measurement: does the warm edit beat crane) → fmt implementation (D74)
  → D70 implementation (overlay universes + wallos rewrite).

## 2026-08-01 (pin-stability D69/D70 + the CI-green campaign)

- **Decisions**: D69 FETCH pin stability (consumed-set keying [a]; update-lock
  double-fetch probe [b, Mathijs's mechanism]; consumed-volatile = authoring
  normalization [c]; STABLE FETCH + functions-in-${} refused [d]; --cold never
  refetches / offline by construction + codegen fingerprint in memo keys [e]).
  D70 overlay universes (`FROM <flakeref> OVERLAY ./x.nix AS pkgs` — the
  wallos forcing example; USING died in dialogue: "a function call in a
  jacket"; getFlake back to evidence-gated). Diagnosis report (sol, scratch):
  warm memo hits never refetch; parse-server's npm noise is 1,355 timestamped
  cache-index records, node_modules byte-identical → consumed-keying fixes
  3 of 4 exhibit classes; pnpm/dist is the normalization case.
- **CI-red anatomy resolved**: main's full tier red since the scenario tier
  entered CI (last green 4a1ddde, 2026-07-30) — thirteen merges passed a red
  CI unnoticed (track gates don't run the full tier; compounded by my
  tail-pipeline exit-code measuring bug). Two causes: (1)
  **scenario-gc-survival: real product bug** — `nix store add-path` created
  compose generations without reference edges to service items; profile
  rooted the generation, not the items; retag → GC ate the live item. Fixed
  (persistent indirect roots per generation+service, generation.rs:77 /
  runtime.rs) and merged with coldaudit (`98887dc`). (2) artifact_kinds
  fixture resolves host `sh` (/usr/bin/sh on CI) — track/cifix running.
  **Process fix needed: full flake tier into the merge gate** (or a loud
  daily main run).
- **Merged**: track/coldaudit+gcfix (`98887dc` — cold_audit standing test,
  projB/projB-chef pins proven unstable across two clean update-lock runs
  [cargo joins the dozzle class], D64 example repairs, observability receipt
  fix); track/nixcompare (`9b57ad1` — docs/nix-build.md: LOC 17 vs 30 vs 38,
  cold 26.9s wins, warm-edit loses to crane 20.3 vs 16.5 [explained: both
  discard own-step increments; our 4s = snapshot/hash overhead], no-op loses
  to eval-cache; honest zero-runtime-references caveat).
- **New design questions surfaced**: (1) **warm step replay** — let a RUN
  step reuse its own previous workspace state (true cargo increments, beats
  crane; licensed by D39.1 + the new cold-audit safety net; also fixes
  migrate.md's subtle overclaim vs BuildKit cache-mounts); (2) **closure
  truth for builder-made dynamic binaries** — they reference union paths,
  not store paths (nixcompare receipt: zero registered references; clean-
  store distribution unproven) — patchelf-at-the-dock or store-path
  toolchain refs, needs a round. Both await Mathijs.
- **Running**: track/pinkeys (D69 impl), track/cifix. Queued: D70 impl
  (overlay universes + wallos rewrite), coldaudit-style full-tier gate rule.

## 2026-08-01 early (the strata dialogue: D67/D68; r5 merged)

- **D67 registered** after a long dialogue round (first draft deliberately
  scrapped by Mathijs, re-derived in chat, registered on his "ja zeker"):
  the strata (manifest/runner · compose · Cixfile · index), the necessity
  chain 2⇒3⇒1a (store-native ⇒ artifact distribution ⇒ eval-free consumer;
  flake-interop an explicit non-goal), the recipes-vs-artifacts inversion
  with determinism-liberation as keystone (by-artifact distribution licenses
  D39.1), prior work weighed (fetchClosure/CA-derivations = byte layer
  solved; name layer open; OCI-flight as demand evidence), pin/audit quality
  as the differentiators (pin in META never in tag strings; docker's variant
  zoo = missing-metadata symptom), plateaus & avalanches (+CA-derivations
  expectation), early-vs-late binding ("isolation and relocation are the
  same mechanism"), the price list + trust ladder (D35 dependency). Open
  product questions marked: stratum-1a standalone; tool-distribution scope.
- **D68 registered**: ITEM returns as a manifest-less pure store tree
  (Mathijs's correction: a manifest is stratum-1a vocabulary).
- **Merged: track/migrate-r5** (`ed7f8be`, sol — class-split: node 1/3
  [excalidraw ✅; parse-server FETCH instability; directus FHS-native-binary
  gap], PHP 1/1 [wallos ✅ via the D4 .nix escape — first real exercise of
  that boundary], go+cgo 0/1 [filestash static-lib gap]; legacy: tomcat
  root-caused AND repaired ✅, dozzle pin-flap documented to the byte (seven
  sumdb tile files, 35,808 B). All three passes independently re-verified.
  Failure classes moved from "language too poor" to genuine design
  questions: FETCH-pin normalization, FHS binaries.)
- **Launched**: track/coldaudit (terra — D47e sampled clean rebuilds as a
  standing test target) and track/itemrevive (terra — D68). Queued after
  itemrevive: track/nixcompare (sol — gitsitter flake vs crane vs Cixfile,
  tour-style doc with timing receipts + index-distribution chapter).
- Coming design round once evidence lands: FETCH-pin stability
  (normalization vs per-ecosystem guidance; dozzle bytes are the exhibit).

## 2026-07-31 night (D66 + absdest, usrbinenv; famref + r5 launched)

- **Decisions**: D66 (absolute artifact destinations — "you declare places in
  your runtime world"; relative died, BUILDER stays workdir-relative; the
  "here" rule addendum: relative is coherent exactly where a here exists),
  D58 addendum (/usr/bin/env joins the sandbox skeleton; the NixOS-two-paths
  boundary — /bin/sh via union, /usr/bin/env via skeleton, never a third),
  D64(b) aligned with implementation (bare EXEC resolves against the
  EFFECTIVE PATH — declared ENV PATH replaces self-bin for resolution too).
- **Merged** (independently re-verified): track/absdest (`30e8262` — full
  absolute-destination sweep incl. corpus re-checks); track/usrbinenv
  (`575e2fe` — skeleton symlink, skeleton version in chain keys, actionable
  missing-env hint; **echo-server now FULLY green: first npm source-build
  through the whole chain** — 11th green pair, independently re-proven on a
  fresh fetch).
- **Launched, running**: track/famref (terra — D65 index refs as FROM artifact
  binders) and track/migrate-r5 (sol — batch: excalidraw, parse-server,
  wallos [.nix escape-hatch exercise], directus, filestash [cgo/lib
  boundary], + tomcat diagnosis + dozzle pin-instability documentation;
  class-split grading is the deliverable).
- **Open with Mathijs**: review-reads (design.md D60–D66 prose, docs/
  migrate.md, tour narrative after five sweeps). Coming design question:
  FETCH-pin stability for npm/go-mod-class fetches (r5 will document the
  evidence).

## 2026-07-31 evening (D63–D65 + demofix, selfbin, corpuspolish, gcroots)

- **Decisions**: D63 (two acts — anonymous loop vs naming act — + unit-lifetime
  GC roots under /run, compose-dev-without-tags parked), D64 (implicit self-bin:
  runtime PATH=<item>/bin, bare EXEC resolves against own bin/, leaf-consumer
  principle per nix prior art), D65 (FROM's three input kinds: tree/universe
  via flakeref — classic default.nix import, flake-only trees documented out —
  plus NEW cix-item index refs as artifact binders; universe-tags dissolved:
  prelude = substitution keyed on the lock pin). All Mathijs verdicts in
  dialogue; universe-tags formally closed.
- **Merged** (each independently re-verified): track/demofix (`7b0d93c` —
  found post-famtags: all 8 demo scripts silently broken by the JSON build
  contract, no automated gate runs demos [structural gap, noted for the
  scenario tier]; fixed via member selectors; nginx example renamed my-nginx,
  README verbatim again); track/selfbin (`b20c1cf`, D64); track/corpuspolish
  (`0007ec8` — all 14 pairs in D56–D64 language, fresh receipts: 10 cix-green,
  caddy/nginx/redis/phpmyadmin NEWLY green vs the r-rounds; product findings:
  /usr/bin/env shebang class fails in the builder union [echo-server], go-mod
  FETCH pin instability [dozzle], tomcat unreachable — r5 fodder);
  track/gcroots (`8b70dc7`, D63(b) — terra needed two legs [context ceiling,
  honest handoff]; live root lifecycle proven: present during run, cleaned on
  stop, dangling auto-link pruned by nix).
- Housekeeping: worktrees pruned 17→2 (sandbox caused phantom "busy" errors —
  metadata removal needed to run unsandboxed); model table updated in
  nix-config (terra 5 flawless rounds + honest interruption handling; sol
  clean prose round).
- **Open with Mathijs**: D66 — COPY/LINK destination spelling inconsistency
  (LINK accepts item-rooted absolute, COPY refuses; recommendation: allow both
  everywhere, leading `/` = own item root, docker muscle memory on the
  adoption bridge). Also: /usr/bin/env as sandbox-skeleton symlink (same class
  as /bin/sh, forcing example in corpus).
- Open for agents: D65 implementation (FROM index refs — spec to write),
  migrate r5 (new no-escape set + tomcat/dozzle/echo-server findings),
  tourvm (queued).

## 2026-07-31 addendum (D62 + famtags + promptrefresh)

- **D62 registered and amended same day** (dialogue rounds with Mathijs; prior-art
  scan docker/compose/bake/OCI/flakes/cargo/maven/go/npm/skopeo): three layers
  (store path = anonymous identity; SERVICE/APP block names = declared member
  names, never baked into bytes; tags = index metadata). NO NAMESPACE directive
  (YAGNI amendment — the not-baked rule had hollowed it to a default; family
  name is `--namespace` at tag time, required for multi-artifact). Bare
  `cix build .` = JSON member map only; `.#member` = backward slice, bare path;
  `-t` tag-only + repeatable (atomic multi-tag; semver cascade = multiple -t);
  selector XOR tag; no implicit `:latest` anywhere; `#`=build-side selection,
  `/`=index-side naming; family tag tables (tag → member map) ride D46 later —
  round one uses slashed names in existing tables.
- Merged: **track/famtags** (`23a4491`, terra — D62 round one; README
  inconsistency that started the discussion is fixed) and **track/promptrefresh**
  (`e4fc75a`, sol — docs/migrate.md living-receipts rewrite in D47–D62 language,
  both complete samples independently re-built; quality high, no overclaims
  found).
- Critical analysis of Mathijs's two-mode docker observation recorded in chat:
  the axis is unnamed-loop vs naming-act (not local vs CI); D7/D35(b) already
  give tag=root GC; **real gap found: `cix run` registers no runtime GC root**
  — anonymous dev-loop runs can be swept by `nix store gc` under a live unit
  (compose safe via D30 profiles). Proposed D63: two modes as acts +
  unit-lifetime GC roots + compose-dev-without-tags parked evidence-gated.
- **Open with Mathijs**: D63 verdict. Minor: examples/pack/nginx could rename
  SERVICE nginx → my-nginx to restore the README "verbatim" claim (cosmetic).
- Open for agents: gcroots mini-round (after D63), corpus polish round
  (post-D62 language + fresh receipts — echo-server check.sh currently fails
  honestly at pre-D58 PATH), migrate r5 from the new no-escape set.

## 2026-07-31 (D60/D61 + three tracks: argvenv, dirnames, corpusfetch)

- Merged (each gate independently re-verified before merge): **track/argvenv**
  (`ae06c59`, terra — D59 builder ENV + quote-aware EXEC/SETUP argv, D60
  `GRANT jit|egress` hard flip, STATE→STATEDIR, manifest v5 `grants` list;
  note: `GRANT jit` now legal on APP — latent-gap fix, hardening applies MDWE
  to app units too); **track/dirnames** (`faa4b0c`, terra — LOGSDIR/CONFIGDIR
  complete the D52 role-dir family); **track/corpusfetch** (`01ffc9c`, terra —
  vendored corpus context trees (~2707 files) replaced by pinned
  `corpus/migrate/fetch.sh`; all ten contexts re-verified byte-identical
  against the pins before deletion; echo-server smoke honestly fails at
  pre-D58 `PATH` — corpus-polish scope).
- Decisions registered: **D60** GRANT capability-grant family (one per line,
  SERVICE/APP, closed vocabulary, evidence-gated queue: mlock/net-admin/device/
  realtime/namespaces/fuse; refusals: no GRANT all, no raw CAP_*). **D61**
  rootless/non-Linux user story: (a) cix machine wanted unconditionally,
  (b) no homegrown rootless imitation (podman userns stack refused; surf
  systemd's unprivileged-sandboxing line — v260 `PrivateUsers=managed` is the
  DynamicUser-analogue landing), (c) Quadlet acknowledged as prior art,
  (d) daemon route = primary Linux answer (thin socket-activated compiler-
  daemon, nix-daemon pattern; property-boundary argument; machine becomes a
  transport; sequencing after corpus wave, before D49 netns). **D52 addendum
  closed**: LOGSDIR/CONFIGDIR. Universe-tags knots dissolved in discussion
  (FROM stays flakeref-only; prelude = substitution keyed on the pin) — not
  yet registered as a D-number.
- Corpus: +12 verified no-escape build-class candidates in CANDIDATES.md
  (hunt; two agent false-claims caught on independent re-check: stump and
  mattermost ARE packaged). systemd/podman rootless research findings in
  session scratchpad (`systemd-rootless-findings.md`).
- Environment: gitsitter (auto-sync daemon) documented in
  nix-config/global.AGENTS.md with precise ownership-gated push semantics.
- Specs queued: track-tourvm (de-user the tours via VM-generated transcripts,
  after argvenv-successor rounds), prompt-refresh (holding for the D62
  verdict so migrate.md teaches final tag semantics).
- **Open with Mathijs**: D62 family-tags proposal (`-t name:tag` only;
  multi-artifact → `family/member:tag`; single-artifact member elision — the
  elision taste call is his). Also still his: kernel-config probe follow-up is
  parked long-term (decided this session).
- Open for agents: prompt-refresh (after D62), corpus polish round (post-keys
  + post-D60 language, fresh receipts), migrate r5 drawing from the new
  no-escape set, then the queued wave-two tracks.

## 2026-07-30 (track/keys complete)

- Merged work: none in this worktree. Completed D56–D58 in `bfde201`,
  `4f0cadd`, `e84e31a`, `89594e9`, and `ebf25e2`: declared FETCH EXPECT
  hashes; pure builder chain keys; path-indexed consumed-output records;
  disposable persistent workspaces with warm prefix reuse; hard CACHE removal;
  exact warm/cold path attribution; and ordered package IMPORT unions replacing
  builder PATH. Migrated active examples, locks, reference/migration prose, and
  executable tour chapters 4–5.
- Decisions: implemented existing D56, all five D57 invariants, and rewritten D58
  without amendment. The fixed sandbox environment names the conventional
  `/etc/ssl/certs/ca-bundle.crt`, but no CA package is implicit: only an explicit
  `IMPORT ${pkgs.cacert}` makes it available. A changed suffix containing FETCH
  replays from step zero in a clean workspace; an unchanged pinned FETCH prefix
  may be reused only from its matching persistent workspace.
- Verification: the exact fmt, warning-denied workspace clippy/test, explicit
  proj1 warm/selective/cold/wipe acceptance, tour regeneration/zero-drift/
  determinism-twice, dogfood VM, systemd-261 compose fallback VM, and scenario
  lifecycle commands are recorded in `crates/cix-cixfile/LOG.md`; every stage
  passed on the final committed implementation. `corpus/**` and
  `nix/scenarios/**` are absent from the track diff, and test-created user units
  were reset and stopped.
- Open with Mathijs: none. Open for agents: independently verify and merge
  `track/keys`.

## 2026-07-30/31 (session close: the language-forge day — D47 through D59)

State for the next session: **main is green and pushed**; track/keys landed and merged the same night (entry above)
(sol's round: D56 EXPECT, D57 — the big engine round: D56 EXPECT, D57 narrow read-keying
increment 1, D58 IMPORT-replaces-PATH; on completion: independent gate re-run, then
merge). Everything else below is landed and verified.

**Merged this session** (each independently re-verified before merge): track/index2
(D45 tag tables), track/items (D40/D41 + proj1), track/composefallback (systemd-261
loud degradation + capability probe), track/scenarios (+flake-hardening +repinfix
rounds; VM scenario tier + index hammer + D43/D44 FRONTIER contract; CI now runs the
full `nix flake check` tier), track/blocks (D47 blocks&binders), track/polish
(EGRESS rename + tour-14 cache story), track/noscript (D55), track/tourbook
(six-chapter tour + D50–D53), track/sdbisect (×3 rounds), track/refresh (corpus
living-receipts), migrate rounds r1–r4 (corpus/migrate: 6 passing dual-receipt
pairs, honest fails documented, no-escape audit: only 5/48 candidates lack a
nixpkgs escape).

**Decisions D47–D59** recorded in design.md, nearly all distilled from Mathijs's
read-throughs: D47 blocks/binders (+bare-COPY context sugar amendment), D48 bundle
(CACHE-as-snapshot-exception→later superseded, egress returns, liveness/readiness
vocab, identity registry, systemd-transparency principle, hooks dissolve), D49
netns resolutions, D50 ITEM dropped, D51 COPY-dir/continuations/RUN-heredoc
(+deliberate no-FETCH-heredoc addendum), D52 CACHEDIR+LINK-flip, D53 comments,
D54 metadata (full arc: annotations→in/out→META source→workshop extraction — cix
parses nobody's manifest), D55 SCRIPT dropped, D56 EXPECT, D57 narrow keying
(cache/snapshot wall dissolves; CACHE removed; rm-rf-workspace-always-safe), D58
IMPORT replaces PATH (union-mount bin/etc/share, order=priority, no cacert
default), D59 builder ENV + EXEC argv quoting.

**systemd/kernel 226-saga: parked with dignity.** EPERM captured
(uid_map write), NOT a systemd 257→261 regression (same-harness 257 fails too),
NOT kernel 6.17→6.18 (both fail in VM); remaining axis = NixOS-VM environment vs
Ubuntu host. Next probe documented in .dev/upstream-systemd-226-namespace.md
(manual unshare repro + kernel-config diff); issue correctly unfiled; product
already safe via behavior-probe fallback.

**Queued, in order** (specs exist): (1) track/argvenv (D59, after keys — same
crate); (2) prompt-refresh: docs/migrate.md full rewrite post-keys (teach IMPORT/
EXPECT/builder-ENV; fix rotted ENV form, EGRESS, FILE mentions; add the
"everything bare inside builders" consistency lesson — r4 proved prompt-rot is a
real failure class: the prompt needs the living-receipts treatment on every
language change); (3) corpus polish round: rewrite all pairs in post-keys language
(adminer EXPECT, echo-server r5 retry as multi-FETCH, phpmyadmin bare tools),
fresh receipts; (4) migrate r5 (echo-server retry + next batch per class-split:
build-class loss is the real grade — r4: 1 pass/6 real attempts, three product
findings, two of which D58/D59 already fix); (5) wave two feature tracks
(track-health, hostbinds+sharededge identity pair, timers+hooks, fetchsecrets —
all D48-resolved); (6) D42/D43 tree-grammar wave starting from the scenario
FRONTIER flips; (7) D46 parametric publish; (8) netns realization (D49).

**Open with Mathijs**: the universe-tags design (`FROM nix:unstable AS pkgs` as a
cix tag — leaning interpretation 3: universe artifact = pinned flakeref for eval +
prebuilt substitutable prelude item; two knots open: FROM ref-grammar
disambiguation flakeref-vs-index-ref, and prelude priority must lose to user
IMPORTs). Also: whether to file the kernel-config probe follow-up.

**Process lessons (hard-won, memory updated)**: (1) NEVER merge on an agent's
green claim for checks not independently re-run — the update-repin fixture was
never green; the combined gate caught it (terra false-green recorded in tally);
(2) codex launches: bare AND `< /dev/null` — 0.146.0 blocks on open stdin; two
hangs from &&-chaining despite the recorded rule; (3) answer ≠ decision: when
Mathijs asks a question, chat first, register only on his verdict (corrected
twice); (4) the r4 merge executed during an interrupt — verify execution state
before claiming a hold.

## 2026-07-30 (track/refresh corpus maintenance)

- Committed `3ab519d`: refreshed the living migration corpus for D50–D53. Whoami
  now has independently pinned builder-local clone/module FETCH steps (new item
  `/nix/store/y696s2gxr34bvcqzndm8gz2hkkhf9fci-cix-item-whoami`); Adminer,
  Memcached, and the complete Echo Server upstream tree obey the `context/` layout;
  all five requested Cix checks pass. Caddy/Echo remain explicitly layout-only with
  their prior failure/timeout honest in their receipts.
- Added a per-candidate no-escape nixpkgs audit to `corpus/migrate/CANDIDATES.md`,
  including alternate attrs and Ghost/LinuxServer ambiguity notes. The corpus LOG
  holds the full reproduced commands and refreshed store paths.
- Verification: `cargo test --workspace` passed untouched. Exact corpus commands:
  `cd corpus/migrate/whoami && ../../../target/debug/cix build --update-lock build . && ./check.sh cix`,
  then `./check.sh cix` in traefik, nats, adminer, and memcached; root-layout and
  `git diff --check` audits passed. No open items.

## 2026-07-30 (track/tourbook complete)

- Merged work: none in this worktree. Completed the track in `63073b8` and
  `7a6be82`: D50 ITEM removal; D51 directory-preferred COPY, physical
  continuations, and RUN heredocs; D52 CACHEDIR and target-first LINK; D53
  full-line comments; the manifest vocabulary and no-op BUILDER sweeps; and a
  regenerated six-chapter executable tour whose examples expose their inputs
  before operating on them.
- Decisions: implemented existing D50–D53 without amendment. A BUILDER now appears
  in examples only for RUN or FETCH work. The tour's tag-driven run/debug story
  also closed a runtime seam by resolving cix-index refs before retaining the
  existing Nix-installable fallback.
- Verification: the exact combined fmt, warning-denied workspace clippy/test,
  tour regeneration/drift/determinism-twice, dogfood VM, systemd-261 compose
  fallback VM, and scenario lifecycle command is recorded in
  `crates/cix-cixfile/LOG.md`; every stage passed. Residue and unit cleanup scans
  are clean.
- Open with Mathijs: none. Open for agents: independently verify and merge
  `track/tourbook`.

## 2026-07-30 (track/repinfix complete)

- Merged work: none in this worktree. Diagnosed and corrected the update-repin
  scenario in `e0bee7d`: its API declaration now opts into D44 root-side tracking,
  and its generation/store-path assertions run before the bounded HTTP retry.
- Decisions: no design amendment and no product change. The failure was a scenario
  fixture bug: the declaration omitted `update: track`, so the documented default
  `pin` policy correctly replayed the adjacent v1 lock on the second `cix up`.
  Temporary VM diagnostics confirmed identical generations, manifests, API units,
  and v1 `ExecStart` inputs, with no restart; D47 was not involved.
- Verification: `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`,
  `cargo test -p cix --test tour -- --ignored generate_tour`,
  `git diff --exit-code -- docs/tour`,
  `nix build .#checks.x86_64-linux.scenario-update-repin --no-link -L`, and
  `nix build .#checks.x86_64-linux.scenario-lifecycle --no-link -L` are green.
  The corrected scenario proves v2 resolution plus selective API restart and v1
  rollback; lifecycle independently guards restart-changed and DB preservation.
- Open with Mathijs: none. Open for agents: independently verify and merge
  `track/repinfix`.

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

## 2026-07-30 (track/sdbisect follow-up)

- Merged current `main` before the follow-up and captured the stock-systemd-261 failure at the
  syscall level: the temporary namespace `uid_map` write returns `EPERM`.
- Added a same-kernel/VM-harness systemd 257.6 cell from pinned Nixpkgs. It also fails at that
  UID-map write and at `226/NAMESPACE`; the available evidence therefore does not establish a
  systemd 257→261 regression or support filing the upstream regression draft.
- Decision: no design decision. The issue draft now records the named failing operation and the
  same-harness falsification. Exact commands and logs are in `.dev/sdbisect.LOG.md`.
- Verification: `nix build .#sdbisect-revert-vm --no-link -L` passed with stock/reverted-261 and
  pinned-257 manager identities asserted, plus the exact UID-map `EPERM` trace assertion. Open
  with Mathijs: whether to investigate the remaining kernel/host-policy condition. Open for
  agents: none.

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
