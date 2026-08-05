# Valkey migration receipt

2026-08-05 independent re-verification. `corpus/migrate/fetch.sh valkey`
restored `valkey-io/valkey-container` at
`b2ecf7da2cf3c7ed28869e9af0709876f0991497`, context `8.1/alpine`.

`CIX=../../../../target/debug/cix ./check.sh cix` exited 0. The faithful
8.1.9 source build completed and the system-manager probe value was exactly
`PONG` from `valkey-cli PING` on port 6379.

`target/debug/cix build --cold corpus/migrate/docker/valkey#valkey` exited 1
after the full compile. The value-checked diagnostic is a warm/cold read-set
divergence at `libbacktrace/.libs/stVX6SFe`: warm recorded `Some(Absent)` and
cold recorded `None`. This is retained as a CIP-87 language defect, not hidden
by updating a lock.

The dissolved nixpkgs twin built warm and cold with exit 0, producing
`/nix/store/49x5zalp4g2av97z7khzbyf7fzrmjz8j-cix-item-valkey`.
