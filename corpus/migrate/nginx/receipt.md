# nginx migration receipt

Cix refresh: 2026-08-02. The current conversion uses a locked store executable,
quote-aware `START`, the service's real `LOGDIR`, and explicit stderr logging.

Docker side: historical 2026-07-30 receipt, not rerun; no historical Docker digest was captured.

## 2026-08-02 build, runtime, and log projection

```text
target/debug/cix build -t regrade corpus/migrate/nginx
/nix/store/s35rsvbhr2hi9qmm1wpj4bibgl3nssvz-cix-item-nginx
target/debug/cix compose check .dev/scratch/regrade/nginx-compose.json
compose corpus-nginx: 1 services, 0 edges, valid
sudo env PATH="$PATH" target/debug/cix up \
  .dev/scratch/regrade/nginx-compose.json --update='*'
activated corpus-nginx from \
  /nix/store/ih1xx84mpvnwggfwwdcaqkhc0qvysqw6-cix-compose-corpus-nginx-generation
curl --fail --silent http://127.0.0.1:80/
target/debug/cix logs corpus-nginx/nginx \
  --invocation e7e0b1a98d454881b8d5e7ec3ec5be04 -n 30
nginx/1.30.4
start worker process 1302377
```

Exit status: 0. The HTTP probe passed; `-g 'daemon off; error_log stderr
info;'` remained one argv word. `cix logs` printed and executed the equivalent
indexed `journalctl` query using `CIX_COMPOSITE`, `CIX_SERVICE`, and the current
invocation ID. Runtime used the existing D36 PrivatePIDs fallback. Docker mode
and upstream version parity were not re-verified. Cleanup used `cix down` and
removed both temporary tags.
