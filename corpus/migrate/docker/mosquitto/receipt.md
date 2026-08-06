# Mosquitto migration receipt

2026-08-05 independent re-verification. `corpus/migrate/fetch.sh mosquitto`
restored `eclipse-mosquitto/mosquitto` at
`5cd2546511596a269dbf53f85858c623b09ebdd6`, context `docker/2.0-openssl`.

`CIX=../../../../target/debug/cix ./check.sh cix` exited 0 after the faithful
2.0.22 source build. Inside the running service, the bounded subscriber
received the exact value `cix-ok` published to `cix/roundtrip`. Cix reported
the host's documented `PrivatePIDs` degraded fallback while the value check
still passed.

`target/debug/cix build --cold corpus/migrate/docker/mosquitto#broker` exited
0, replaying all three pinned FETCH snapshots and producing
`/nix/store/xc8j2q6vhzh51b6br4p8sdbzf24qkss4-cix-item-broker`.

The dissolved nixpkgs twin built warm and cold with exit 0, producing
`/nix/store/xggwfv26l7nsraymni7kqcwlgizwmrbm-cix-item-broker`.

## 2026-08-06 widened-parser cold-replay sweep

The faithful warm prerequisite,
`devenv shell -- ./target/debug/cix build corpus/migrate/docker/mosquitto#broker`,
exited 1 while refetching the GPG key: the declared digest was
`sha256-vSgE3uH3CP8JOW6MW1bZ7CAH6sEglZQHhKQDPqweLNs=` and the observed digest
was `sha256-t6R8FXIdR0EmuvPy0OTBWvZzkNnmF/y4rNCsyVUr67c=`. Therefore the
worktree had no valid local snapshot for the corresponding cold command, and
no cold replay or lock regeneration claim is made; the drift is recorded in
`GAPS.md`.
