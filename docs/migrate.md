# Converting a Dockerfile to a Cixfile

You are converting a Dockerfile into a Cixfile: a declarative build+run file for
composix (`cix`), which produces immutable, content-addressed artifacts that run as
hardened systemd services. This document is all you need.

## The shape

A Cixfile is a list of named blocks. Every `${name}` you use is bound by a
declaration; there are no ambient names.

```dockerfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs   # binds the package universe
FROM . AS src                                      # (optional) binds the source dir

BUILDER build                    # the workshop: the only place RUN may appear
  PATH ${pkgs.cargo}/bin ${pkgs.gcc}/bin           # build-time tool search path
  CACHE target                   # dir that persists across builds (cargo target etc.)
  COPY ${src}/ .                 # bare relative sources also work (docker context)
  RUN cargo build --release --offline

SERVICE myapp                    # one artifact = one service
  COPY ${build}/target/release/myapp bin/myapp     # pluck ONLY what the item needs
  EXEC bin/myapp
  ENV PORT default=8080
  PORT http = env PORT
```

- `BUILDER <name>` binds its final workdir as `${name}`. `RUN` is sandboxed and has
  NO network. `FETCH <cmd>` (inside a builder) or `FETCH <name> <cmd>` (top-level,
  empty workdir, binds `${name}`) is the only network access; its output hash is
  pinned in `Cixfile.lock` automatically.
- `SERVICE` blocks assemble the runnable artifact: `COPY`/`LINK`/`FILE`, `EXEC`,
  `ENV name default=…|required`, `PORT name = <value|env NAME>`, `EGRESS` if the
  service initiates outbound connections, and role dirs:
  `DIR state /var/lib/<name>` for persistent data (docker `VOLUME` maps here),
  plus cache/logs/run variants.
- `APP <name>` is like SERVICE but run-to-completion (no ports).

## Mapping heuristics

- **`FROM debian/alpine` + `apt/apk install X`** → do not install anything: reference
  the package from nixpkgs (`${pkgs.X}/bin/…` in EXEC/PATH). Most official images
  (nginx, redis, postgres…) need NO builder at all — the package already exists.
- **Multi-stage builds** → one `BUILDER` per stage; a later builder stages an earlier
  one with `COPY ${earlier}/ .`; `COPY --from=X` → `COPY ${X}/path dest`.
- **`RUN --mount=type=cache,target=D`** → `CACHE D` in the builder.
- **`curl|wget + checksum` downloads** → a `FETCH` (the pin is enforced for you).
- **`VOLUME /data`** → `DIR state /var/lib/<name>` and point the app there (env/flag).
- **`EXPOSE N`** → `PORT http = N` (or via env). **`USER`/`gosu`/`su-exec`/`tini`**
  → delete: systemd runs the service as an unprivileged dynamic user and is the init.
- **ENTRYPOINT shell scripts** → read them; port the essential env/flag setup into
  ENV/EXEC lines. Scripts that chown/mkdir/sed files at startup are usually
  replaceable by role dirs + a config file assembled at build time (`FILE`/`COPY`).

## The check

Ship a `check.sh` next to your Cixfile with two modes, same probe body:
`./check.sh docker` (docker build+run the original, probe, teardown) and
`./check.sh cix` (`cix build .` + `cix run <item>`, same probe, teardown).
The probe proves the service does its one central thing (HTTP 200, redis PING,
`--version`…), bounded by timeouts, exit 0/1. Both modes must pass.
