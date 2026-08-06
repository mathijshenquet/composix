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

The system-manager runtime probe is deliberately **not** green. Its synchronous
`check.sh cix` exit was value-captured as `1`: the upstream `/init.sh` could not
create `/config/settings.json` because cix's arbitrary-path role realization
made `/config` read-only, then native readiness exhausted its 10-second bound.
The user-manager fallback also cannot project declared app paths and timed out.
No `/health` runtime value is claimed.
