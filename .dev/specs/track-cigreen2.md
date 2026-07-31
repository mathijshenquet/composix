# track/cigreen2 — the last two CI reds; gate = full flake check

Read AGENTS.md first. Context: after the gc-root fix and fixture de-hosting,
CI on main still fails on exactly two items. Fix both. The gate for THIS track
is the full `devenv shell -- nix flake check -L` plus workspace tests — no
cherry-picked VM subset (tonight's lesson: per-track subsets kept missing tier
members).

1. **cix-index pull test race** (crates/cix-index/tests/pull.rs:191,
   `serve_and_pull_follow_the_bare_tag_web_contract`): ConnectionRefused on
   slow runners — the test connects before the spawned server listens. Make
   it robust: bounded wait-for-listen (retry connect with timeout) before
   asserting; apply the same guard to sibling serve tests if they share the
   race. No sleep-and-pray constants without a bound and a clear failure
   message.
2. **compose-fallback VM AssertionError** (nix/scenarios or
   nix/compose-fallback-vm.nix, test script line 7): reproduce locally
   (`devenv shell -- nix build .#checks.x86_64-linux.<the compose-fallback
   check> -L --no-link`), root-cause — suspect the NEW compose gc-root
   registration (crates/cix-compose/src/runtime.rs, landed tonight)
   interacting with the degraded systemd-261 path (root registration may need
   to degrade LOUDLY per D36 style rather than fail, or the scenario's
   assertion needs to cover the new output honestly). Cite file:line for the
   cause. Never weaken the scenario's actual contract; if behavior must
   change, it degrades loudly.
3. Gate: fmt / warning-denied clippy / `cargo test --workspace` /
   `nix flake check -L` FULL and green / cold_audit sweep. Exact repros in
   crates/cix/LOG.md. Commit on this branch when green.
