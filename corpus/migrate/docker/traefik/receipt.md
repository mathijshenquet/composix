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
./target/debug/cix build corpus/migrate/docker/traefik
./target/debug/cix build --file Cixfile.dissolved corpus/migrate/docker/traefik
cd corpus/migrate/docker/traefik && CIX=/home/mathijs/worktrees/composix/track-regen1/target/debug/cix ./check.sh cix
```

All four commands completed synchronously with exit status 0. The faithful build
produced `/nix/store/hs71647jq7ny98451vv9j18gng3c3vad-cix-item-traefik`; the
dissolved build produced
`/nix/store/h3c65zmdkfbihrj4lpcdxvmdrvqp3f6q-cix-item-traefik`. The unchanged
ping probe returned `OK` and `PASS cix`. This was a warm memo-hit receipt: it
therefore reproduces the product defect that fails to validate the two identical
copy-pasted `EXPECT` declarations against their recorded pins; it does not
validate those declarations.

## 2026-08-05 CIP-102 volatile-fetch sweep

`target/debug/cix build --update-lock release corpus/migrate/docker/traefik#traefik`,
`CIX=/home/mathijs/worktrees/composix/track-cip102/target/debug/cix ./check.sh cix`,
and `target/debug/cix build --cold corpus/migrate/docker/traefik#traefik` each exited
0 synchronously. Both update-probe reads were identical; the release metadata and
asset FETCHes now use TOFU consumed pins, while the vendor digest remains checked by
`sha256sum --check`. The probe returned `OK` and `PASS cix`; the cold replay produced
`/nix/store/3b3fy3i4waczrpdliwldcaqqfc9sah8h-cix-item-traefik`.
