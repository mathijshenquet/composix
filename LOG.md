# composix work log

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
