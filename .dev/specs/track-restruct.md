# track/restruct — pack/compose/build layout + composix.lib.withSpec (D37 a+b)

Read AGENTS.md first. Authoritative design: docs/design.md D37 (a) and (b), with D16 (the
adoption ladder) and D22 (mounts are item-relative) as context. D37 wins on conflict.

## Restructure

- `git mv` the six service examples to `examples/pack/<name>/`: nginx, postgres, redis,
  caddy, node-app, listenfds. Cixfile is the canonical form.
- Delete the dogfood-era `default.nix` in packs where a Cixfile covers the same service
  (postgres, nginx, listenfds, …). Keep `demo.sh` files, fix their paths.
- `examples/compose/` stays but its inputs must consume the moved packs; verify the compose
  example still resolves/locks/builds end-to-end after the move.
- `git mv examples/buildshape examples/build/proj1` (content unchanged this track).
- `examples/dstyle/` and `examples/LOG-examples2.md` stay untouched (design-era archive).
- Add `examples/README.md`: the layout, the adoption ladder (Cixfile → withSpec → plain
  .nix), and a one-liner marking dstyle as archive.
- Sweep every reference to old paths: docs (tour pages regenerate via the harness — do not
  hand-edit), tests/tour fixtures, nix/vm-dogfood.nix, README.md, demo scripts.

## composix.lib.withSpec (build this; small nix lib)

`nix/lib.nix` (wired into flake.nix as `lib.withSpec`, also importable without flakes):

    composix.lib.withSpec {
      manifest = { cixManifest = 3; services.redis = { exec = [ "${pkgs.redis}/bin/redis-server" ... ]; ... }; };
      mounts = { "/opt/redis" = ./conf-dir-or-drv-path; };   # optional: item-relative mount sources (D22)
      name = "redis-cix";
    }

- Produces a derivation whose `$out` contains `cix-manifest.json` (via `builtins.toJSON` —
  store-path interpolation works naturally through nix string context) plus the declared
  mount trees linked/copied at their item-relative locations so the D22 "declared mount
  missing from store item" check passes.
- Validate nothing beyond JSON shape here — `cix` remains the validator (D15); this is a
  thin assembly helper, not a second schema implementation.
- Demonstrate it: `examples/pack/redis/default.nix` becomes the idiomatic withSpec form
  (redis keeps its Cixfile too — the pack README notes the two files ARE the two ladder
  rungs, same service).
- Test: a nix eval/build test (flake check or existing test harness style) that builds the
  withSpec redis, asserts the manifest parses via `cix` (e.g. the existing manifest-reading
  code path or `cix build`-adjacent check), and that the VM/tour gates still pass.

## Verification gate

1. `cargo build/test/fmt --check/clippy -D warnings` clean; tour regenerated + drift green.
2. `nix build .#checks.x86_64-linux.vm-dogfood` passes with the new paths.
3. Both moved flagship demos verified live (sudo): pack/nginx via Cixfile-built item,
   pack/redis via the withSpec item — run, one request/ping, stop, clean.
4. `grep -rn 'examples/(nginx|postgres|redis|caddy|node-app|listenfds|buildshape)' —
   no stale references outside historical LOG/spec files.
5. Commit on track/restruct. No commit = failed task.

## Log

Keep .dev/specs/track-restruct.LOG.md current (append-only, timestamped, transcripts).

## Correction round 1 (orchestrator post-merge finding, 2026-07-29)

`examples/pack/listenfds` is broken: its `default.nix` was deleted but NO Cixfile exists —
and none CAN exist: v3 listeners are deliberately not a Cixfile v1 directive (D29). The
demo's `cix build` therefore fails. The LOG claim "listenfds now use Cixfile builds" was
wrong; the listenfds demo was not in the live gate, which is how it slipped through.

Fix: recreate listenfds as the second `composix.lib.withSpec` example (the idiomatic rung is
exactly for what Cixfile cannot express). The old definition is preserved in git
(`git show 73ce3a8:examples/listenfds/default.nix`) — port it to withSpec form, adjust
demo.sh to build via nix (mirror how redis' demo builds its withSpec default), note in
examples/README.md that listenfds demonstrates withSpec-for-listeners. Gate: workspace tests
+ tour drift green, VM check passes, AND the listenfds demo runs live end-to-end (sudo) —
this demo is now mandatory in the gate. Commit on this branch.
