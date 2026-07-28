# composix work log

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
