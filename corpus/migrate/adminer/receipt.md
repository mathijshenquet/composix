# adminer migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`EXPECT`, builder `IMPORT`, role directories, explicit artifact `bin/`, and D62 selector builds).

Docker side: historical 2026-07-30 receipt, not rerun. Historical image ID: `sha256:1b74a51d52b661e95107ce2eaec2186d2f55c7c2d5d73602b76a0e6897778659`.

## `./check.sh cix`

```text
cix item /nix/store/g0yd5br9x691yawy8mjsfx6dd0ssp8mp-cix-item-adminer
PASS cix
```

Exit status: 0. The login-page probe passed.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Worker warm evidence: the staged ordinary primary/dissolved builds and supplied
HTTP probe exited 0, producing `/nix/store/bw93qxr1pl5dr9byk7ila1r7nz8xsi0q-cix-item-adminer`
and `/nix/store/cvx894bi51vwd2dkqa4p2v6dwhgix0vq-cix-item-adminer`. Its cold
attempt had no local replay snapshot.

Assembler evidence after `bash corpus/migrate/fetch.sh adminer` (exit 0):

- `target/debug/cix build corpus/migrate/adminer` and the identical build in
  `./check.sh cix` exited 1. The published file SHA-256 check printed `OK`, then
  FETCH rejected declared `sha256-bJPKViwRiTQlcxrQKFRp9YcNjlBIfkx5BcjTR2T8Ing=`
  versus fetched `sha256-XJVIfFbNTbUDfZOfIT045BHY1SSbRlfcdqM4d742DfQ=`.
- `target/debug/cix build --file Cixfile.dissolved corpus/migrate/adminer`
  exited 0 with `/nix/store/ayxlixrcrbl2w61s54whwa2d8r4bhm3q-cix-item-adminer`.
- `target/debug/cix build --cold corpus/migrate/adminer` exited 1 because the
  failed fresh fetch left no replay snapshot; the dissolved cold build exited 0
  with the same dissolved item. No EXPECT or lock pin was updated.

Docker mode was not rerun.
