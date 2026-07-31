# Receipt

Verdict: **capability-gap** (Cixfile class: build). Dozzle requires the Docker socket/API at runtime; cix has no declared host Unix-socket or Docker-API grant. Independently, this attempted build fails before the runtime boundary.

Verbatim transcript:
```
$ timeout 120 ../../../target/debug/cix build .
$ vite build
sh: /work/node_modules/.bin/vite: /bin/sh: bad interpreter: No such file or directory
[ELIFECYCLE] Command failed with exit code 126.
Error: line 11: RUN failed
status=1
```
Docker digest and cix store path: not produced.

## Corpus fetch verification (2026-07-31)

The raw pinned checkout additionally contains `.claude/`, which was not in the
historic build context and is not a Docker build input. `SOURCE` now records that
one exclusion; the selected checkout diffed byte-identically with the vendored
tree.
