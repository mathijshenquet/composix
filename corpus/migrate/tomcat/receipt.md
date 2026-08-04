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

After `bash corpus/migrate/fetch.sh tomcat` exited 0, the assembler ran the exact
ordinary and cold twin commands:

- `target/debug/cix build corpus/migrate/tomcat` and
  `target/debug/cix build --file Cixfile.dissolved corpus/migrate/tomcat`
  exited 0 with the two items above.
- `CIX=/home/mathijs/worktrees/composix/track-regen2/target/debug/cix
  ./check.sh cix` exited 0 synchronously from the case directory and printed
  `PASS cix`.
- The same two builds with `--cold` each exited 0 and returned the same items.

Docker mode was not rerun.
