# whoami migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`IMPORT`, builder `ENV`, bare builder commands, and bare artifact `START`).

Docker side: historical 2026-07-30 receipt, not rerun. Historical image ID: `sha256:bf3c544f03d387bd30e9b8bc2e08bc6b6f4aae80d884822fe43e472844ab5d44`.

## `./check.sh cix`

```text
cix item /nix/store/5raa2baz0ixyj7lrhqzrcdbpvf8rlj0i-cix-item-whoami
PASS cix
```

Exit status: 0. The HTTP probe passed.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Worker warm evidence: both staged twins built and the supplied HTTP probe exited
0. The primary item was `/nix/store/8rpibhpfz2v69qr6xn29w6jh1c4c7qz1-cix-item-whoami`;
the dissolved item was `/nix/store/1j7fzvgv6izkkhhy73rr9dr45zzwvxp5-cix-item-whoami`.

The assembler observed the same two items from
`target/debug/cix build corpus/migrate/docker/whoami` and the `--file
Cixfile.dissolved` twin command, both exit 0. The supplied probe, run
synchronously from the case directory with this worktree's `CIX`, exited 0 and
printed the HTTP request plus `PASS cix`. Repeating both builds with `--cold`
also exited 0 with the same items. Whoami has no fetched local context, and
Docker mode was not rerun.
