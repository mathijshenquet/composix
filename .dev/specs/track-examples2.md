# Track: examples2 — postgres polish + redis, caddy:80, node (JIT)

Environment note: run Rust/nix gates via `devenv shell -- …` or the toolchain on PATH; never
wait for environment confirmation.

Territory: `examples/postgres/`, `examples/redis/` (new), `examples/caddy/` (new),
`examples/node-app/` (new), `docs/cixfile.md` (one new subsection), `nix/` (VM check
extension). Do NOT touch `examples/compose/` (another track), `examples/dstyle/`,
`examples/buildshape/`, or `crates/`. COMMIT AS YOU GO. Work log:
`examples/LOG-examples2.md`.

## 1. Postgres heredoc-free (the dirname-$0 pattern)

Convert `examples/postgres/Cixfile` scripts to verbatim sibling files: `LINK` the needed
package binaries item-internally next to the scripts (`bin/initdb`, `bin/postgres`, and the
nss_wrapper .so via a lib/ link), scripts reference them as `"$(dirname "$0")/…"` and source
the shared env via `. "$(dirname "$0")/../lib/runtime-env.sh"`. Result: zero heredocs, all
COPY siblings. Keep `default.nix` as-is (coexistence demo). Both build paths + sudo demo
green. Then document the pattern in `docs/cixfile.md` (short subsection under the directives:
"Scripts and their tools — sibling links, `$(dirname "$0")`, no templating").

## 2. New examples (each: Cixfile-only, sibling files, demo.sh with cleanup trap, sudo-green)

- **redis**: light. Unix socket in RUNDIR *and* a value TCP port; demo: redis-cli PING over
  both. Config as COPY sibling.
- **caddy**: static file server on **PORT http = 80** — the <1024 capability demo. demo.sh
  asserts `AmbientCapabilities=cap_net_bind_service` via `systemctl show` and curls :80.
- **node-app**: a tiny Node HTTP server (nixpkgs `nodejs`), stdlib-only — the **JIT** demo.
  Spec has `JIT`. The demo runs it once WITHOUT the flag (patch the built spec in a temp copy
  or build a no-jit variant) to show MemoryDenyWriteExecute genuinely killing V8, then with
  `JIT` working — the adversarial proof that the flag is load-bearing. If V8 happens to
  survive MDWE on this host, record exactly that in the LOG (also a finding) and show the
  flag's unit-level effect instead.

## 3. VM check

Extend `nix/vm-dogfood.nix` to also run redis + caddy + node-app (same style as the existing
two; caddy proves :80 inside the VM). Keep total VM runtime reasonable; note the delta.

## Done gate

fmt/clippy/`cargo test --workspace` green (nothing in crates should change — if a runtime bug
blocks an example, STOP on that example, record it as a finding, and continue with the rest);
all demos green ×2 under sudo; VM check green; no leftover units; committed; LOG summary.
