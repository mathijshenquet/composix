# nats migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (explicit artifact `bin/` and bare `START`).

Docker side: historical 2026-07-30 receipt, not rerun. Historical image ID: `sha256:f0f977e50ad69c0b9a041f145cce27df06166295792391f98f4ac415a067756c`.

## `./check.sh cix`

```text
cix item /nix/store/6nj4ggg4wmfpy6hw6hlp3wwnrn66w6ic-cix-item-nats
{"status":"ok"}PASS cix
```

Exit status: 0.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Docker mode was not rerun. No context fetch was possible: this case's `SOURCE`
does not identify a reproducible repository for the missing configuration and
entrypoint files.

```text
devenv shell -- cargo build -p cix
./target/debug/cix build corpus/migrate/docker/nats
./target/debug/cix build --file Cixfile.dissolved corpus/migrate/docker/nats
cd corpus/migrate/docker/nats && CIX=/home/mathijs/worktrees/composix/track-regen1/target/debug/cix ./check.sh cix
```

All four commands completed synchronously with exit status 0. The faithful build
produced `/nix/store/a16p3gqwxxldhvmg94wak99z9q6w3xjy-cix-item-nats`; the
dissolved build produced
`/nix/store/sncljqjd4jgw8kvmf57m0f4j7qsqikyp-cix-item-nats`. The unchanged
monitoring probe returned `{"status":"ok"}` and `PASS cix`.
