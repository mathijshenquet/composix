# Valkey migration receipt

2026-08-06 independent re-verification. `corpus/migrate/fetch.sh valkey`
restored `valkey-io/valkey-container` at
`b2ecf7da2cf3c7ed28869e9af0709876f0991497`, context `8.1/alpine`.

`CIX=../../../../target/debug/cix ./check.sh cix` exited 0. The faithful
8.1.9 source build completed and the system-manager probe value was exactly
`PONG` from `valkey-cli PING` on port 6379.

The pre-fix cold control synchronously exited 1 after the full compile at
`libbacktrace/.libs/stVX6SFe`: warm recorded `Some(Absent)` and cold recorded
`None`. Targeted syscall capture attributed the random file to a successful
`openat(..., O_RDWR|O_CREAT|O_EXCL, 0600)`. A second random `conf*` exhibit
showed that PID-namespace child IDs prevented inherited-cwd attribution for
`mkdir`, after which reads below that same-step-created directory entered the
read set.

With both observation-classification defects fixed, a fresh-workspace
`target/debug/cix build --update-lock build
corpus/migrate/docker/valkey#valkey` synchronously exited 0, followed by
`target/debug/cix build --cold corpus/migrate/docker/valkey#valkey` exit 0.
Both produced `/nix/store/fgm45ck2453mrpmhpv4hqhc64kcwa3f6-cix-item-valkey`;
the refreshed lock retains the tarball inputs and omits the generated random
paths from the recorded read set.

The dissolved nixpkgs twin built warm and cold with exit 0, producing
`/nix/store/49x5zalp4g2av97z7khzbyf7fzrmjz8j-cix-item-valkey`.

## 2026-08-06 widened-parser cold-replay verification

The first ordinary warm command was discarded because it returned a completed
output memo hit with zero Nix subprocesses. From a fresh workspace,
`devenv shell -- ./target/debug/cix build --update-lock build --workspace-directory
/var/tmp/composix-coldreplay-valkey.MuhhvN corpus/migrate/docker/valkey#valkey`
exited 0 after a full 251.120 s RUN; its two FETCH update probes were
identical and the committed `Cixfile.lock` remained byte-identical. The valid
empty-workspace replay,
`devenv shell -- ./target/debug/cix build --cold --workspace-directory
/var/tmp/composix-coldreplay-valkey.MuhhvN corpus/migrate/docker/valkey#valkey`,
exited 0 after a full 235.539 s RUN and returned
`/nix/store/fgm45ck2453mrpmhpv4hqhc64kcwa3f6-cix-item-valkey`. Valkey is
verified under the widened parser; no regeneration was performed.
