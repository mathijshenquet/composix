# filebrowser migration receipt

2026-08-06 synchronous build receipt. `corpus/migrate/fetch.sh filebrowser`
restored `filebrowser/filebrowser` at
`e8a388f840173580116f2743813d03b22286e44e`, context `.`.

`devenv shell -- target/debug/cix build corpus/migrate/docker/filebrowser#filebrowser`
exited 0 and produced
`/nix/store/w9z11aajzl0brzdrz5dg7lyglqxfv4y0-cix-item-filebrowser`. Its
FETCH uses the fixed v2.63.23 Linux amd64 release URL; the release's published
SHA-256 check passed before extraction. The dissolved nixpkgs twin also exited
0, producing `/nix/store/hxq52ncbblpwhhrl9sqsi3i070hs1c5p-cix-item-filebrowser`.

An empty-workspace faithful `--cold` replay exited 0 using the pinned FETCH
snapshot and produced the same item. The dissolved twin's `--cold` build also
exited 0.

2026-08-06 system-manager runtime receipt. The existing pinned faithful item
started with the repaired realization; `systemctl show` reported
`ReadWritePaths=/config /database /srv`, their `BindPaths=` sources under
`/var/lib/cix-run-filebrowser`, and only `TemporaryFileSystem=/var/lib:ro`.
`curl --fail http://127.0.0.1:80/health` exited 0 and returned
`{"status":"OK"}`. The source context in this checkout is currently incomplete,
so a fresh corpus build stops at its missing copied `init.sh`; this does not
affect the value-checked runtime receipt for the pinned item.

`--user` remains separately degraded where a user manager rejects mount
namespaces: cix then deliberately omits `BindPaths=`, so declared arbitrary app
paths cannot be projected. That fallback is not the repaired system-manager
ordering defect.
