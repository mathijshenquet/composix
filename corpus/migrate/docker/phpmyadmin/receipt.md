# phpmyadmin migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`IMPORT`, `EXPECT`, `STATEDIR`, and explicit artifact `bin/`).

Docker side: historical 2026-07-30 receipt, not rerun; no completed Docker transcript was captured then.

## `./check.sh cix`

```text
cix item /nix/store/gbfjxsjkc07w22y99jgglmsxf3s0yydb-cix-item-phpmyadmin
```

Exit status: 0. The phpMyAdmin HTTP probe passed after the existing D36 fallback.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Worker warm evidence: the staged ordinary build and supplied HTTP probe exited
0 with `/nix/store/lgg4g2n6badwlm69f2i1bn6sclx0gpi2-cix-item-phpmyadmin`.
Its staged cold audit exited 1 at a warm/cold read-set difference for
`output/html`.

After `bash corpus/migrate/docker/fetch.sh phpmyadmin` exited 0, the assembler observed
a different, clean cold result:

- `target/debug/cix build corpus/migrate/docker/phpmyadmin` exited 0 after verifying
  the published SHA-256 and GPG signature, producing
  `/nix/store/dqgqk9bwxk363f0f8jxn8jdh3na6pxhk-cix-item-phpmyadmin`.
- `CIX=/home/mathijs/worktrees/composix/track-regen2/target/debug/cix
  ./check.sh cix` exited 0 synchronously from the case directory; the login-page
  probe passed after the documented D36 fallback.
- `target/debug/cix build --cold corpus/migrate/docker/phpmyadmin` replayed the pinned
  FETCH snapshot, executed both RUN steps, exited 0, and produced the same item.

The locked-universe audit used nixpkgs revision
`643809054d65fdd466a63e3155b8c498cb483c04`; its top-level and `phpPackages`
case-insensitive phpMyAdmin match lists were both empty. Docker mode was not
rerun, and the staged EXPECT was not changed.
