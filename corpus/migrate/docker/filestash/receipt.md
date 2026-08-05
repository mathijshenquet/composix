# filestash migration receipt

2026-08-05, generated `migrate.md@f474d3f` (gpt-5.6-luna). Docker mode was not rerun.

`target/debug/cix build .#filestash` exited non-zero in its networked FETCH before C compilation or the claimed static closure:

```text
Get "https://sum.golang.org/lookup/cloud.google.com/go/accesscontextmanager@v1.9.7": dial tcp: lookup sum.golang.org on 127.0.0.53:53: server misbehaving
```

No Cix item or runtime probe is claimed. The `pkgsStatic` investigation must be rerun from a DNS-healthy builder.

## 2026-08-05 clean-worktree recheck

After `corpus/migrate/fetch.sh filestash`, DNS and network fetches progressed. The first FETCH then exceeded the 20-minute case bound while snapshotting a 2.7 GiB, roughly 69k-file Go module cache; it was terminated before compilation. This replaces the DNS wall with a bounded fetch-snapshot scale wall; no item or runtime probe is claimed.
