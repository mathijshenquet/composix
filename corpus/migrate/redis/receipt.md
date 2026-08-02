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
