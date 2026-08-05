# caddy migration receipt

2026-08-05, generated `migrate.md@f474d3f` (gpt-5.6-luna). Docker mode was not rerun.

- `./check.sh cix` — synchronous exit 0; `/nix/store/d7cc7052ivjyp1jyh9j7yq539ixpf2df-cix-item-caddy` served HTTP (D36 PrivatePIDs fallback was reported).
- `target/debug/cix build --cold .#caddy` — synchronous exit 0; all three pinned FETCH snapshots replayed and the SHA-512 release check passed.
- `target/debug/cix build --file Cixfile.dissolved .#caddy` — synchronous exit 0; `/nix/store/p5ccqzksw3qgnrhlmyw1pwrzv9qmdagr-cix-item-caddy`.

The Cixfiles declare `80/tcp`, `443/tcp`, `443/udp`, and `2019/tcp`.

## 2026-08-05 assembly rerun

`./check.sh cix` exited non-zero before assembly: Cix's sandbox FETCH returned the same 769-byte body (`sha256-XnQDFzyLvpWytEFT1E48D4X0IY4QxmuuGtje8R+jyvE=`) for distinct raw GitHub asset URLs, so the first stable EXPECT pin rejected it. Direct host curl returned the distinct raw Caddyfile/index assets. No pin was changed.
