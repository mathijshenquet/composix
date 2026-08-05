# homer migration receipt

2026-08-05 independent re-verification. `corpus/migrate/fetch.sh homer` restored
the pinned `6972395b62463d4eec8ec5a1b12e60e8d482088a` context.

`target/debug/cix build --update-lock build .#homer` exited 0 and produced
`/nix/store/s9qyvvn9krr5sg134h57j23gn4wj7lyp-cix-item-homer`. The locked FETCH
and offline build completed with pnpm 11.18.0 and Vite produced `dist/`.

`CIX=../../../../target/debug/cix ./check.sh cix` exited 0: the system-manager
service answered on `127.0.0.1:8080/` with Homer’s app mount. The run emitted
the current D36 `PrivatePIDs` degradation warning; it did not change the probe
result.

The earlier staged registry failure was reproduced only before `cacert` and the
pnpm data directory were explicitly imported/traced. It is therefore retained
as staging friction, not claimed as a package-registry capability wall.
