# nginx migration receipt

Cix refresh: 2026-08-02. The current conversion uses a locked store executable,
quote-aware `START`, the service's real `LOGDIR`, and explicit stderr logging.

Docker side: historical 2026-07-30 receipt, not rerun; no historical Docker digest was captured.

## 2026-08-02 build, runtime, and log projection

```text
target/debug/cix build -t regrade corpus/migrate/docker/nginx
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

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Docker mode was not rerun.

```text
devenv shell -- cargo build -p cix
bash corpus/migrate/docker/fetch.sh nginx
./target/debug/cix build corpus/migrate/docker/nginx
./target/debug/cix build --file Cixfile.dissolved corpus/migrate/docker/nginx
cd corpus/migrate/docker/nginx && CIX=/home/mathijs/worktrees/composix/track-regen1/target/debug/cix ./check.sh cix
```

The compiler, fetch, and both builds completed synchronously with exit status 0;
the fetch reconstructed revision `e0f008fab4e1ce252c9451590c6a2aff305dd03c`.
The faithful build produced
`/nix/store/pnya3lvjhri5n6fkixybwpbrmp2srzvs-cix-item-nginx`; the dissolved
build produced `/nix/store/h6rdgrdjxy4n0a820wa15gajjb2v9cv5-cix-item-nginx`.

The unchanged Cix probe completed synchronously with exit status 1. After the
documented D36 PrivatePIDs fallback, nginx exited status 1 before HTTP became
reachable. The unit journal reported:

```text
nginx: [alert] could not open error log file: open() "/var/log/nginx/error.log" failed (2: No such file or directory)
nginx: [emerg] open() "/var/log/nginx/access.log" failed (2: No such file or directory)
```

The final cleanup also reported that the original unit name was no longer loaded.
The probe was not weakened; this is a faithful-twin runtime finding.

## 2026-08-05 STOPSIGNAL regeneration

Current `target/debug/cix` built `/nix/store/aqf3p4z5gyjbx5pqfsvjdclz5iyiayz1-cix-item-nginx`; `./check.sh cix`, faithful `--cold`, and dissolved `--cold` each exited 0. The pid lives at `/run/nginx/nginx.pid` under `RUNDIR`, and the service carries `STOPSIGNAL SIGQUIT`.

## 2026-08-06 widened-parser cold-replay verification

After the pinned context was restored, the faithful warm build and
`devenv shell -- ./target/debug/cix build --cold corpus/migrate/docker/nginx#nginx`
each exited 0 and produced `/nix/store/aqf3p4z5gyjbx5pqfsvjdclz5iyiayz1-cix-item-nginx`.
The cold verification dirtied only `Cixfile.lock`'s `sourceHash`, changing
`2289625103e7245081b02115293cc8910f4da9520cdb8104152ec153e26dfba0` to
`31aa13b1809fbe04ae8957eac7ca84368a76f92cf35dad307e5afb73302fdf93`; the
exact line was restored byte-for-byte and no regeneration was performed. This
is retained as a keying-neutrality exhibit in `GAPS.md`.
