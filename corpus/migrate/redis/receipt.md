# redis migration receipt

Cix refresh: 2026-08-02. The conversion now preserves Docker's `/data`
application path directly with `STATEDIR /data`, exercising CIP-82 leg 1's
arbitrary-path, full-mirror role backing.

Docker side: historical 2026-07-30 receipt, not rerun; no historical Docker digest was captured.

## 2026-08-02 direct build and runtime probe

The upstream context was not needed for the package-dissolving Cix conversion,
so this round ran the same central PING probe directly:

```text
devenv shell -- cargo build -p cix
target/debug/cix build corpus/migrate/redis#redis
/nix/store/0zd94c03qk3gddgg01cwaznwgcywiap2-cix-item-redis
sudo target/debug/cix run --detach <item>
cix-run-redis-18c7ebb3d19834de0.service
PING -> +PONG
sudo target/debug/cix inspect --runtime <unit> --human
state dirs /var/lib/private/cix-run-redis,/var/lib/private/cix-run-redis/data
```

Exit status: 0. The first run exposed and corrected the stale corpus spelling
`START redis-server`; the current conversion names the locked store executable
explicitly. The final Redis PING probe passed after the existing D36 fallback,
and runtime inspection showed the full `/data` mirror under the unit-scoped
state root. Docker mode and upstream version parity were not re-verified.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Docker mode was not rerun.

```text
devenv shell -- cargo build -p cix
bash corpus/migrate/fetch.sh redis
./target/debug/cix build corpus/migrate/redis
./target/debug/cix build --file Cixfile.dissolved corpus/migrate/redis
cd corpus/migrate/redis && CIX=/home/mathijs/worktrees/composix/track-regen1/target/debug/cix ./check.sh cix
```

Every command completed synchronously with exit status 0. The fetch reconstructed
revision `2ac6f46c6ba6f3ece54183a518a2bfd865390368`. The faithful build produced
`/nix/store/kh91y700ig4sxrza0rl5rcq465xvjlkj-cix-item-redis`; the dissolved
build produced `/nix/store/f8xsydbpl1bmn3vr348fqyfxcp6lzm4h-cix-item-redis`.
The unchanged PING probe initially observed connection refusal while the service
started, then passed within its existing bound after the documented D36 fallback.
