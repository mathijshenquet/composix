# caddy migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`STATEDIR` and explicit artifact `bin/`).

Docker side: historical 2026-07-30 receipt, not rerun. Historical image ID: `sha256:339d03613a5f18e115c3c9b4ef12cc65c6bd80afaa720ce5b9ade79aa04cbe67`.

## `./check.sh cix`

```text
cix item /nix/store/d8aiy4fv1wb3zm6b69h410kfp28q82pi-cix-item-caddy
```

Exit status: 0. The HTTP probe passed after the existing D36 PrivatePIDs fallback.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Docker mode was not rerun.

```text
devenv shell -- cargo build -p cix
./target/debug/cix build corpus/migrate/caddy
./target/debug/cix build --file Cixfile.dissolved corpus/migrate/caddy
cd corpus/migrate/caddy && CIX=/home/mathijs/worktrees/composix/track-regen1/target/debug/cix ./check.sh cix
```

All four commands completed synchronously with exit status 0. The faithful build
produced `/nix/store/pnrn7yc3df094g759kabla8zyfqc757a-cix-item-caddy`; the
dissolved build produced
`/nix/store/1y2b1mhz2spkww9d1hdpz7q0m92va5si-cix-item-caddy`. The unchanged
HTTP probe passed after the documented D36 PrivatePIDs fallback.
