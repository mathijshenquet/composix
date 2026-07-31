# track/demofix — repair demos for the D62 build contract + nginx verbatim rename

Read AGENTS.md first. Authoritative: design.md D62. Two jobs, one round.

## Problem
D62 changed bare `cix build <dir>` output to a JSON member map; ten call sites
across eight `examples/**/demo.sh` scripts still capture it as a bare store
path (`store_path=$(cix build "$example_dir")`) and are silently broken —
found post-merge because no automated gate runs the demos.

## Work
1. Fix every demo build call to the D62 contract. Preferred form: the member
   selector, which prints a bare path — `"$cix_bin" build "$example_dir#<member>"`
   (member = the SERVICE/APP block name in that example's Cixfile). Multi-step
   demos (compose/stack) select per built dir the member they need.
2. Rename `examples/pack/nginx` service: `SERVICE nginx` → `SERVICE my-nginx`
   (Mathijs), update its demo selector accordingly, and restore README.md's
   sample caption from "adapted from examples/pack/nginx" to
   "this is examples/pack/nginx, verbatim" — after making the README sample and
   the example file byte-identical in the block body (README shows the same
   directives; verify by eye and say so in the LOG).
3. E2E-verify EVERY touched demo end to end (they use sudo + the system
   manager; prior rounds ran these — stop/reset any created units afterwards,
   verify with `systemctl list-units 'cix-*'` clean). Record each demo's exact
   invocation + result in the LOG. If one cannot pass for a reason unrelated
   to this change, record the honest finding — do not paper over it.
4. Append a note to .dev/LOG.md? NO — that file is the orchestrator's; instead
   record in crates/cix-cixfile/LOG.md the structural gap you are fixing
   around: demos claim "e2e-verified" but no automated gate executes them
   (candidate future home: the scenario VM tier).

## Gate
`devenv shell -- cargo fmt --all --check`; warning-denied workspace clippy;
`cargo test --workspace`; tour drift + determinism twice (README/examples are
not tour-generated, but run it to prove nothing drifted);
`devenv shell -- nix build .#checks.x86_64-linux.vm-dogfood --no-link -L`
(vm-dogfood consumes example asset files); all demo e2e runs green per point 3.
Exact repros in crates/cix-cixfile/LOG.md. Commit on this branch when green.
