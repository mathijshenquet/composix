# memcached migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (explicit artifact `bin/` and bare `START`).

Docker side: historical 2026-07-30 receipt, not rerun. Historical image ID: `sha256:176adbf343271bf411648dced32bbe97b2c734052e66bfc11ba7ba7aebeea8d5`.

## `./check.sh cix`

```text
cix item /nix/store/i9zz4b30wg3lnjmshj5bkjz51999hpmc-cix-item-memcached
VERSION 1.6.42
PASS cix
```

Exit status: 0.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Docker mode was not rerun.

```text
devenv shell -- cargo build -p cix
bash corpus/migrate/fetch.sh memcached
./target/debug/cix build corpus/migrate/memcached
./target/debug/cix build --file Cixfile.dissolved corpus/migrate/memcached
cd corpus/migrate/memcached && CIX=/home/mathijs/worktrees/composix/track-regen1/target/debug/cix ./check.sh cix
```

Every command completed synchronously with exit status 0. The fetch reconstructed
revision `53ac0ecb0bf88b471a0110f8996ce791baf1a667`. The faithful build produced
`/nix/store/vj6jca5s0j4fgi3wqdlyyyh1gvqc69ld-cix-item-memcached`; the
dissolved build produced
`/nix/store/jipgr1jay2wx2d7dsw63nm6z9263jsm9-cix-item-memcached`. The
unchanged protocol probe returned `VERSION 1.6.45` and `PASS cix`. The faithful
builder reported that the upstream test harness was skipped because its isolated
environment has no root account; compilation and the version/runtime checks passed.
