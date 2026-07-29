# composix work log

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
