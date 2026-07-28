# Track: dogfood — make real services run (nginx, then postgres)

Goal: `sudo cix run` works for real nixpkgs services, starting from the diagnosed failures
below. Read `DESIGN.md` "Part 2 — spec + run" first. This is an empirical loop: run, hit a
wall, fix or record, iterate. Passwordless sudo is available to you — you may start/stop
system-mode transient units freely, but ALWAYS clean them up (`systemctl stop cix-run-…`,
verify with `systemctl list-units 'cix-*'`) even on failure paths, and never touch non-cix
units.

## Ground rules

- Work log: append to `crates/cix-run/LOG.md` — every wall you hit and how you resolved it.
  This log is design input; it matters as much as the code.
- Territory: `crates/cix-run/`, `examples/`. Do NOT touch other crates, `DESIGN.md`, `docs/`,
  `specs/`.
- The `cix-spec.json` SCHEMA IS FROZEN: do not add fields or change semantics. If a wall can
  only be cleanly solved by a schema/design change, solve it the boring in-store way if
  possible (e.g. a wrapper start script inside the store item) and RECORD the design proposal
  in LOG under a "spec boundary proposals" heading instead of implementing it.
- Done gate: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
  `cargo test --workspace` green; both demos run end-to-end under sudo.

## Diagnosed bugs to fix first (in cix-run)

1. **RestrictAddressFamilies merge syntax is invalid.** The generator emits
   `RestrictAddressFamilies=AF_UNIX` then `RestrictAddressFamilies=+AF_INET +AF_INET6`; the
   `+` prefix does not exist for this directive and systemd rejects the transient property
   (`Failed to set unit properties: Invalid argument`) — this currently breaks every
   system-mode run with declared ports. Emit one line listing all allowed families. Update the
   golden fixtures accordingly.
2. **`cix run` on a plain `/nix/store/...` path must not require `nix` on PATH.** Under
   `sudo`, root's PATH lacks nix and run fails with "failed to invoke nix". For an existing
   store path, skip invocation entirely (read the dir directly); when nix genuinely is needed
   (flake installables), fall back to `/nix/var/nix/profiles/default/bin/nix` if `nix` is not
   on PATH.

## Then: nginx end-to-end

`examples/nginx/default.nix` exists (build: `nix-build examples/nginx -o result-nginx`).
Iterate `sudo cix run <store-path> --detach` + `curl http://127.0.0.1:8080/` until it serves
the page. Journal (`journalctl -u cix-run-nginx-…`) tells you each next wall. Anticipated
walls you may hit (verify, don't assume): pid file location, temp paths, `/dev/stdout` access
log under systemd, mime types. Adjust the *example* (config/wrapper) where the fix belongs
there; adjust cix-run only for genuine generator/runtime bugs. When green, write
`examples/nginx/demo.sh` (build, run, curl, show `cix ps`, stop; must clean up).

## Then: postgres end-to-end

Write `examples/postgres/default.nix` in the same style: `pkgs.postgresql`, state dir
`/var/lib/postgresql` (app-path model), port 5432 declared. First-run initialization (initdb
when the state dir is empty) must be solved WITHOUT schema changes: a small start script
inside the store item (the docker-entrypoint pattern) — record in LOG that "first-run init
hook" is a spec boundary proposal. Expect walls around: initdb locale/env, unix socket
directory (put it in a writable dir; note the "runtime dir role" proposal if you feel it),
shared memory. Success = `sudo cix run … --detach`, then `psql` (from the store path) connects
over TCP as the created user and `SELECT 1` works. Then `examples/postgres/demo.sh` as above.

## Wrap up

Final LOG entry: complete wall list with resolutions, and the "spec boundary proposals"
section (these feed the next design round: runtime dirs, init hooks, fixed-value ports,
ports<1024 capabilities, MDWE opt-out, whatever else you hit).
