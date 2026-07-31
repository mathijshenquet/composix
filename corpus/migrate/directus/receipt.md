# directus migration receipt

Source revision: `b1d7a45a77661fd13928a53448c06649f36b56f5` (2026-07-31).

Docker: `./check.sh docker` passed on 2026-07-31. Image:
`sha256:1f0b93fb3d7cbb737e1b13e164c01266a85b824fe9510195492bd768435b1498`.

Cix: build-fail. After explicit `gnused` and Python 3.11 provisioning,
`../../../target/debug/cix build --update-lock build .#directus` completed pnpm
FETCH/native sqlite compilation and reached the offline monorepo build. It then
failed with:

```text
spawn .../sass-embedded-linux-x64/dart-sass/src/dart ENOENT
```

The downloaded ELF expects an FHS dynamic loader. D58 deliberately excludes
`lib` from IMPORT, so this is classified as a language gap; no runtime pass is
claimed.

The conversion dissolves PM2 into systemd,
keeps `bootstrap` as an idempotent setup hook, persists SQLite/extensions/uploads,
and probes `GET /server/ping` for exact `pong`.
