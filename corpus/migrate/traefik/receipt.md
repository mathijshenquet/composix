# traefik migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (explicit artifact `bin/` and bare `START`).

Docker side: historical 2026-07-30 receipt, not rerun. Historical image ID: `sha256:c864999938e1dfa9b7dfc5ad644d0e9c5f413612cc501dc69d261684417815a3`.

## `./check.sh cix`

```text
cix item /nix/store/nnil2w7r861fw33aarp97szdgkzxl33v-cix-item-traefik
OKPASS cix
```

Exit status: 0.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Docker mode was not rerun.

```text
devenv shell -- cargo build -p cix
./target/debug/cix build corpus/migrate/traefik
./target/debug/cix build --file Cixfile.dissolved corpus/migrate/traefik
cd corpus/migrate/traefik && CIX=/home/mathijs/worktrees/composix/track-regen1/target/debug/cix ./check.sh cix
```

All four commands completed synchronously with exit status 0. The faithful build
produced `/nix/store/hs71647jq7ny98451vv9j18gng3c3vad-cix-item-traefik`; the
dissolved build produced
`/nix/store/h3c65zmdkfbihrj4lpcdxvmdrvqp3f6q-cix-item-traefik`. The unchanged
ping probe returned `OK` and `PASS cix`. This was a warm memo-hit receipt: it
therefore reproduces the product defect that fails to validate the two identical
copy-pasted `EXPECT` declarations against their recorded pins; it does not
validate those declarations.
