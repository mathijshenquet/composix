# Track: index (part 1) — tag / untag / ls / serve / pull

You are one of two parallel agents on this repo. Read `DESIGN.md` first — "Part 1 — index" plus
the "Decisions so far" list is your contract (esp. D5–D7, D10, D12, D14). This file only adds
implementation constraints; where it and DESIGN.md conflict, DESIGN.md wins. Do not edit
DESIGN.md — propose changes as notes in your LOG instead.

## Ground rules

- Work log: keep `crates/cix-index/LOG.md` current (append-only, timestamped) per the global
  directives. Re-read it after any compaction.
- Territory: you own `crates/cix-index/` and `crates/cix-common/` (shared store-path type, `nix`
  subprocess helpers, ref parsing — the run track is forbidden from touching cix-common and may
  duplicate small helpers; dedupe happens at merge). Do NOT touch `crates/cix-run/`,
  `crates/cix/src/main.rs` (already wired), or anything else outside your territory.
- The CLI variants in `crates/cix-index/src/cli.rs` are the intended surface; adjust
  flags/help there as needed, keep the enum shape.
- Dependencies: add what you need to the workspace with judgment; prefer boring and light
  (suggestion: `axum` + `tokio` or `tiny_http` for serve, `ureq` for the pull client — your call).
- Nix interop: shell out to the `nix` CLI (Determinate Nix 3.21 / nix 2.34, flakes enabled):
  `nix build --out-link`, `nix path-info --json`, `nix copy`, `nix store sign`,
  `nix store add-path`. Wrap subprocess calls in cix-common with good error surfacing.
- Commit to your branch as you go, meaningful messages.

## Deliverables

1. **Ref parsing** (cix-common): `[root_url/]name[:tag]`, docker disambiguation rule (first
   slash-component containing `.` or `:port`, or `localhost` ⇒ root_url), default tag `latest`.
   Thorough unit tests including nasty edge cases.
2. **Local tag store** at `$CIX_STATE_DIR` (env override, default `~/.local/state/cix`):
   symlink farm `roots/<encoded-ref>` → store path, each registered as an indirect nix GC root
   (`nix build <path> --out-link <link>` or `nix-store --add-root`), plus a JSON sidecar per tag:
   per-system `outputs` (D14), optional `drvPath`, `upstream` (root_url origin), timestamps.
   Design a clean filename encoding for refs (they contain `/` and `:`); document it in code.
3. **tag / untag / ls**: `cix tag <installable> <ref>` accepts a store path, flake installable
   (build it), or existing ref (alias). Untag removes symlink+sidecar (unpins). `ls [prefix] -l`
   shows tag, system(s), store path, upstream, age.
4. **serve**: HTTP JSON API per DESIGN.md (`/v1/resolve/{name}/{tag}`, `/v1/tags/{name}`,
   `/v1/names`), serving exactly the tags prefixed with the given root_url. `--substituter`
   populates entries. `--with-store` (D10): maintain a file:// binary cache in the state dir via
   `nix copy --to 'file://<dir>'` for every served path, serve it statically under `/store/`, and
   advertise `http://<listen>/store` as a substituter; `--sign-key` signs paths.
5. **pull**: `cix pull <root_url>/<name>:<tag> [--as ref]` — resolve over HTTP, pick the entry
   for the current system (error if absent), `nix copy --from` a listed substituter, verify
   narHash via `nix path-info --json`; document in LOG what signature enforcement you implement
   (narHash verification is the floor). Tag locally recording `upstream`. Bare `cix pull`
   re-resolves every upstream-carrying tag and fetches the ones that moved.
6. **Tests**: unit tests for ref parsing, encoding, sidecar round-trip. One integration test
   (may use localhost network + real nix store): create a trivial store path
   (`nix store add-path` on a generated dir), tag it, `cix serve localhost:<port> --with-store`
   in-process or as child, then with a second `CIX_STATE_DIR` acting as the other machine:
   `cix pull localhost:<port>/x:v1 --as x`, assert the path exists and the tag + upstream are
   recorded. This test is the DONE gate.
7. **demo.sh** in `crates/cix-index/`: the flow above, runnable by a human, echoing each step.

## Done criteria

`cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace` all
green; demo.sh works; LOG.md has a final summary entry listing any deviations from DESIGN.md and
open questions for the maintainers.
