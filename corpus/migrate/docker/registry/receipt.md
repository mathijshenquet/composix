# Registry migration receipt

2026-08-06 synchronous receipts. `bash corpus/migrate/fetch.sh
docker/registry` fetched `distribution/distribution` at
`9f9289e23a133b02f61d8c768016eddbdec65c61`, with the repository root as
context.

`devenv shell -- target/debug/cix build --update-lock=build
corpus/migrate/docker/registry#registry` exited 0 after the native Go build
and produced `/nix/store/sr92mn94z57j0q5l4s657q80sz16s6yx-cix-item-registry`.
The dissolved twin build and its `--cold` replay also exited 0.

`devenv shell -- env CIX="$PWD/target/debug/cix" ./check.sh cix` exited 0.
After one transient readiness refusal, the service returned the exact value
`{}` from `GET /v2/`. The service also declares the upstream development
config's debug listener on port 5001.

The faithful warm build's dependency FETCH is pinned in
`Cixfile.lock`. A synchronous `target/debug/cix build --cold
corpus/migrate/docker/registry#registry` remains a wall: the warm FETCH read
set contains `.cache/go-mod`, while the cold read set has it absent. The
native warm item and runtime value are retained and this cold limitation is
not hidden.
