# mailpit migration receipt

2026-08-05 clean independent re-verification. `corpus/migrate/fetch.sh mailpit`
restored the pinned upstream context. `target/debug/cix build --update-lock
compile .#mailpit` exited 0 and produced
`/nix/store/15wjdg9dx7v1jkvgz6s43ghrpk55yzl9-cix-item-mailpit`.

`CIX=../../../../target/debug/cix ./check.sh cix` exited 0: the user-manager
fallback served `/livez` on port 8025. Its output explicitly records degraded
isolation and the mount-namespace fallback.

The supported system-manager run was separately attempted and failed after the
Mailpit process started: its native readiness action tried to execute the
workspace-local `target/debug/cix` under `ProtectHome`, yielding `203/EXEC`.
That limitation is not hidden by the degraded receipt.
