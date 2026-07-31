# parse-server migration receipt

Source revision: `315e157637d902d85f465563b2863a9e19bf1ff4` (2026-07-31).

Docker: `./check.sh docker` passed on 2026-07-31. Parse Server image:
`sha256:395ee46833dd658437dcaedcba0d0ed3bea2e2b4cf03e17bd41540344bbb7289`;
MongoDB image digest:
`sha256:aaad67f2dca93148e5343c03210bcfc89a0107516a4756bfa018acd6579e5b18`.

Cix: D69 re-check on 2026-07-31. Two clean
`../../../target/debug/cix build --update-lock build .#parse-server` runs and a
subsequent ordinary `../../../target/debug/cix build .#parse-server` all built
`/nix/store/kf4d06323935ym7y4bllbprjidzn367b-cix-item-parse-server`. The
automatic FETCH lock now pins the seven consumed final paths, not the whole npm
workdir; their hashes exactly match the prior final memo. The double-fetch probe
records the volatile `.npm/_cacache/index-v5/**` metadata and timestamped debug
logs (names and sizes in `Cixfile.lock`) as facts. No Cix runtime pass is claimed:
the receipt still does not run the Mongo-dependent service probe.

Both modes start the same bounded MongoDB 8.0.4
companion because upstream documents a database as part of the runnable contract;
the central probe is `GET /parse/health` returning `{"status":"ok"}`.

Gap: Docker declares writable `/parse-server/cloud` and `/parse-server/config`
volumes. Current `CONFIGDIR` accepts only `/etc/<one-component>`, and linking the
Docker paths to runtime-created configuration directories is rejected by `cix run`
because the links are broken in the immutable item before the runtime mounts exist.
The conversion therefore does not claim those optional extension/config volume
paths are faithful.
