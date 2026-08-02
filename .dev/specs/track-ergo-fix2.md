# track/ergo fix round 2 — workspace-key instability after the outputs restore

The independent gate re-run on `fb7df76` fails
`proj1_multi_item_cache_selectivity_and_clean_rebuild`
(crates/cix-cixfile/tests/proj1.rs:227): the test expects ONE builder
workspace, two exist — two runs that must share a chain key now key
differently. Round 1 of this track did NOT fail here, so your
`fix: preserve ergo tour receipts` commit introduced it — prime
suspect: the lock `outputs` restore feeding output records (or
anything run-varying) back into chain/workspace keying.

Also account honestly: your fix-round receipt claimed the full
`nix flake check -L` green (71 checks) — this failure is in plain
`cargo test --workspace`. Explain the divergence with a synchronous
re-run of the exact failing test BEFORE your fix commit and AFTER it
(git stash/checkout dance or two worktree states); if it is
environment-dependent, that is a determinism bug — find the varying
input, don't paper over the test.

Fix: workspace/chain keys must not include anything derived from
outputs of prior runs. Re-run the failing test until green, then fmt /
examples fmt / clippy / workspace tests / tour regen + drift / full
`devenv shell -- nix flake check -L`. All receipts synchronous.
Commit on this branch when green. LOG: crates/cix-cixfile/LOG.md.
