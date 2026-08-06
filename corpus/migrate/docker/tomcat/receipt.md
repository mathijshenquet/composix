# tomcat migration receipt

Cix refresh: 2026-08-01. Language generation: D56–D64 (`STATEDIR`, `LOGDIR`,
explicit artifacts, setup, and claims).

Docker side: historical 2026-07-30 receipt, not rerun; no historical Docker digest was captured.

## `./check.sh cix` — pass

```text
cix item /nix/store/zb77wa6z6fka2yx3dmz9s3b0wnbb7d3w-cix-item-tomcat
PASS cix
```

The legacy root cause was an incomplete artifact: linking only `catalina.sh`
lost sibling scripts, Tomcat libraries/configuration, Java, and even shell
utilities, so its symlink resolution searched `//bin/setclasspath.sh`. The fixed
item carries the complete Tomcat 10.1.57 and JRE 21 trees, seeds a writable
`CATALINA_BASE`, and declares JIT. Tomcat also opens a private shutdown listener
on port 8005 by default; the setup disables it because the service declares only
8080. The natural probe regards any HTTP response at `/` as reachable, matching
the candidate wording and Tomcat's expected no-application 404.

## 2026-08-04 regeneration (cold, gpt-5.6-luna)

Worker evidence: both staged twins built warm and cold, the supplied HTTP probe
exited 0, and a direct dissolved probe passed. The primary item was
`/nix/store/s58jpph2qgzj18xwwam5is3jkzhqa9mf-cix-item-tomcat`; the dissolved
item was `/nix/store/cffy1xg2019nx265gd2k4rvhlpymd5fl-cix-item-tomcat`.

After `bash corpus/migrate/docker/fetch.sh tomcat` exited 0, the assembler ran the exact
ordinary and cold twin commands:

- `target/debug/cix build corpus/migrate/docker/tomcat` and
  `target/debug/cix build --file Cixfile.dissolved corpus/migrate/docker/tomcat`
  exited 0 with the two items above.
- `CIX=/home/mathijs/worktrees/composix/track-regen2/target/debug/cix
  ./check.sh cix` exited 0 synchronously from the case directory and printed
  `PASS cix`.
- The same two builds with `--cold` each exited 0 and returned the same items.

Docker mode was not rerun.

## 2026-08-06 widened-parser cold-replay sweep

The faithful warm and cold commands each exited 0, with cold returning
`/nix/store/5bqhzp9yc7plf621fr33560zs6hdz41v-cix-item-tomcat`. Verification
dirtied `Cixfile.lock`: `sourceHash` changed from
`4e8b397afdd22a4bc32bf5e1beffd2be13842037a8bbfdbac64df7f809a1ff14` to
`a98267fb02f1acf91908f1e3e8f8ae081bae22b9f65e37b7f186dd97a2c5a60a`, and
`storePath` changed from `/nix/store/s58jpph2qgzj18xwwam5is3jkzhqa9mf-cix-item-tomcat`
to `/nix/store/5bqhzp9yc7plf621fr33560zs6hdz41v-cix-item-tomcat`. The exact
two-line diff was restored byte-for-byte; no regeneration claim is made. This
is retained as a keying-neutrality exhibit in `GAPS.md`.
