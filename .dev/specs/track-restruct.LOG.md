# track/restruct work log

## 2026-07-29 UTC

- Started D37(a+b) restructuring. Read `AGENTS.md`, `.dev/LOG.md`, design decisions D16, D22, and D37, plus this track spec. Baseline is clean on `track/restruct`; next step is to relocate packs/buildshape and add the small `composix.lib.withSpec` helper.

- Moved the six service examples under `examples/pack/` and `buildshape` to `examples/build/proj1`; removed the old hand-rolled nginx, postgres, and listenfds defaults. Added the Redis `withSpec` rung and began moving all active references, including the VM dogfood fixture, to the new paths. Next: evaluate/build the helper and correct any Nix integration issues before the full gate.

- Initial `nix eval` found duplicate dynamic `packages.${system}` output assignments in the flake; consolidated package and check outputs into one attribute set each. No helper build has run yet; rerun after this structural fix.

- Staged the worktree so Nix can see new files in the dirty Git flake. The helper export evaluates as a function; the first Redis build exposed one stray `composixLib` argument to its two-argument example function. Removed it; rerunning Redis build/check next.

- `nix build .#withSpecRedis` and the `with-spec-redis` flake check now pass, proving the manifest and linked `/etc/redis` tree. The focused Rust parsing test initially compared `PathBuf` to `&str`; corrected the assertion to use `PathBuf`, then rerun formatting and the test.

- The Rust test's Nix expression had accidentally emitted literal backslashes before its quotes; changed it to a raw Rust string. Focused test remains the next verification step.

- Made the compose stack consume `examples/pack/redis` through its existing `stack-db:v1` tag, replacing the compose-private PostgreSQL item. The backend now pings Redis over the granted `/run/redis` socket; removed the redundant `compose/stack/db` Cixfile. Also changed moved pack demos whose deleted `default.nix` files had left them building the wrong way: nginx/postgres/listenfds now use Cixfile builds, while Redis deliberately builds its `withSpec` default.

- Focused Nix/Cixfile and compose fixture tests pass after the compose swap. Aligned the flake export with D37's required surface: it is now `lib.withSpec` (rather than a system-nested lib); the helper itself remains directly importable from `nix/lib.nix` with `pkgs`.

- Workspace build/test/fmt/clippy and the regenerated + drift-checked tour all pass. The first VM run stalled because its old assertion expected a `bin/redis-cli` link inside the new thin withSpec item; that helper deliberately attaches only the declared config mount and uses an absolute Redis executable. Stopped the stalled VM builder, and corrected the VM probe to call `${pkgs.redis}/bin/redis-cli` directly. Rerun the VM gate from this corrected fixture.

- Corrected VM gate passed (Redis PING over TCP and its mounted Unix socket both succeeded, followed by caddy/node checks and unit cleanup). Nginx's Cixfile-built live demo passed and cleaned its unit. The Redis withSpec live demo exposed a stale whitespace-sensitive manifest `sed` pattern; it started Redis but could not locate `redis-cli`, then its cleanup trap stopped the unit. Made the parser accept the compact JSON emitted by `builtins.toJSON`; rerun that live demo next.

## 2026-07-29 12:32 UTC

- Verification complete. `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --check`, and `cargo clippy --workspace -- -D warnings` passed. Tour regeneration (`cargo test --test tour -- --ignored generate_tour`) and drift check (`cargo test --test tour`) passed.
- `nix build .#checks.x86_64-linux.vm-dogfood --no-link --print-out-paths` passed; the final VM test completed in 60.48s. `nix build .#checks.x86_64-linux.with-spec-redis --no-link --print-out-paths` passed, and `nix eval .#lib.withSpec --apply 'f: builtins.isFunction f'` returned `true`.
- Live root demos passed with `CIX_BIN="$PWD/target/debug/cix" examples/pack/nginx/demo.sh` (Cixfile-built item, HTTP response) and `CIX_BIN="$PWD/target/debug/cix" examples/pack/redis/demo.sh` (withSpec item, TCP + Unix-socket `PONG`); both stopped their units. `CIX_BIN="$PWD/target/debug/cix" examples/compose/stack/demo.sh` passed end-to-end: moved Redis pack tagged as `stack-db:v1`, check/up/diff/selective update/rollback/down all succeeded.
- Final stale-path scan (`rg -n --glob '!*.LOG.md' --glob '!.dev/specs/**' --glob '!**/LOG*.md' 'examples/(nginx|postgres|redis|caddy|node-app|listenfds|buildshape)' .`) returned no active references; no active nginx/redis cix-run units remain. Ready to commit.
