# ntfy migration receipt

2026-08-06 synchronous independent receipt. `corpus/migrate/fetch.sh ntfy`
restored `binwiederhier/ntfy` at
`4c2b69e0591b51d7ed7b2e71954f0f7be936b47f`, context `.`.

`devenv shell -- target/debug/cix build corpus/migrate/docker/ntfy#ntfy`
exited 0 and produced
`/nix/store/fvfpg6vpw54zdzdqvmlkalxdmc8lqrgr-cix-item-ntfy`. Its pinned
FETCH downloaded the upstream v2.27.0 Linux amd64 tarball and its internal
published SHA-256 check passed before extraction. The dissolved nixpkgs twin
also exited 0, producing
`/nix/store/isl3wgxj0i559h3ja5p0sj3j4k0m01vv-cix-item-ntfy`.

An empty-workspace faithful `--cold` replay exited 0 using the pinned FETCH
snapshot and produced the same item. The dissolved twin's `--cold` build also
exited 0.

`devenv shell -- env CIX="$PWD/target/debug/cix" ./check.sh cix` exited 0.
The system-manager service returned the exact value `{"healthy":true}` from
`GET /v1/health`. Cix reported the host's documented `PrivatePIDs` degraded
fallback while the value check passed.
