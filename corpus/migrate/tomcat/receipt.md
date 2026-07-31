# tomcat migration receipt

Cix refresh: 2026-07-31. Language generation: D56–D64 (`STATEDIR`, `LOGSDIR`,
explicit artifacts, setup, and grants).

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
