# it-tools migration receipt

## Inputs and re-lock

2026-08-06 independent re-verification. `./corpus/migrate/fetch.sh it-tools`
exited 0 and restored the pinned source revision `d505845f918e946ec300af7b36efc107e2f66e9e`.
`devenv shell -- cargo build -p cix` exited 0.

The source lock was 1,544,041 lines (`sha256 4ae33203a00979a48787ee9de068fcf1f1fba11996106fdf62ca6508c070feab`). A from-scratch corrected `cix build --update-lock build .#web` exited 0 and produced a web item; subsequent current-Cixfile replay also exited 0 and produced the retained item `/nix/store/4zalfi4g7n2bd52niggwbhh4873iq4h6-cix-item-web`. The committed lock is 1,536,045 lines (`sha256 1f244e1b902fce4187bb74f5d92f77ce93e82a5140df828497e4a3029e66b8ed`): a delta of −7,996 lines (−0.52%).

The final trace contains 99,625 reads at `builder:build:4`; it does not demonstrate the requested CIP-99 workspace-root aggregation. Earlier scratch captures ranged from 550,128 to 1,536,045 lines, but the smaller capture was from a pre-`RUNDIR` service assembly and was deliberately not retained. No lock-scale green is claimed.

## Runtime

The ordinary `timeout 240 ./check.sh cix` replay hit the volatile build bound
and exited 124 before emitting an item. The extended harness accepts a
value-checked item through `CIX_ITEM`; with the retained item,
`CIX_ITEM=/nix/store/4zalfi4g7n2bd52niggwbhh4873iq4h6-cix-item-web ./check.sh cix`
exits 0 synchronously and prints:

```
HTTP GET / -> 200
HTTP GET /not-a-real-route -> 404
```

The root response is the verified service behavior. The secondary request is
observational because the fixture does not currently provide SPA deep-route
fallback; that remains an open case gap.

The service's complete nginx main configuration sends errors to stderr and
disables access-file logging. `RUNDIR /var/log/nginx` is ephemeral. A persistent
`LOGDIR` variant was tested and failed with `Permission denied` when the
system-manager PrivatePIDs fallback exposed the DynamicUser log bind, so the
runtime workaround intentionally gives up access-file persistence.

## FRICTION

- The build selector must be explicit: `cix build --update-lock build .#web`; a bare selector is not accepted for this Cixfile.
- CIP-99's observed pnpm trace is volatile. Replays of the same pinned source/current Cixfile produced materially different lock sizes, so the lock was never hand-edited to manufacture a reduction.
- Nginx opens compiled-in log paths before a `conf.d` fragment can fix them. A complete main configuration plus `nginx -e stderr` was required, and the host's DynamicUser bind behavior made persistent log directories unusable.
- The original `check.sh` used `curl --fail` for an expected secondary 404, making a valid root 200 receipt exit 1. The secondary probe now reports its status without deciding the check.
