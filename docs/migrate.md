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
  IMPORT ${pkgs.bash} ${pkgs.cargo} ${pkgs.gcc}    # /bin + offered closures
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
  pinned in `Cixfile.lock` automatically. If a re-fetch legitimately changes the
  output, accept it with `cix build --update-lock <binder-name> .` (the FETCH's
  name).
- A builder has no ambient toolchain. `IMPORT ${pkgs.git} ${pkgs.cacert}` makes
  `git` available as a bare command and provides the conventional CA tree for
  git-over-HTTPS; no `SSL_CERT_FILE` ceremony is needed. IMPORT is repeatable,
  earlier declarations win command or file collisions, and only `bin`, `etc`, and
  `share` are unioned into the sandbox root.
- Do NOT chain network steps into one long shell line. Top-level `FETCH <name>` runs
  in an EMPTY workdir — use it only for truly independent ingredients (a dist
  tarball, a prebuilt UI). Steps that build on each other (clone, then download
  deps) are MULTIPLE small `FETCH` lines inside the BUILDER — they chain on the
  workdir and each gets its own pin and memo entry.
- When the Dockerfile builds from a repo context, FETCH the repo (git clone) and
  record the resolved revision in your SOURCE notes. Language dependency caches
  fetched over the network must live INSIDE the fetched tree so the offline RUN can
  use them (e.g. `GOMODCACHE=$PWD/.gomodcache go mod download` in the FETCH, then
  the same variable in RUN).
- `SERVICE` blocks assemble the runnable artifact: `COPY`/`LINK`/`FILE`, `EXEC`,
  `ENV name default=…|required`, `PORT name = <value|env NAME>`, `GRANT egress` if the
  service initiates outbound connections, and role dirs:
  `DIR state /var/lib/<name>` for persistent data (docker `VOLUME` maps here),
  plus cache/logs/run variants.
- `APP <name>` is like SERVICE but run-to-completion (no ports).

## Mapping heuristics

- **`FROM debian/alpine` + `apt/apk install X`** → do not install anything: reference
  the package from nixpkgs (`${pkgs.X}/bin/…` in EXEC, or `ENV PATH = ${pkgs.X}/bin`
  for service scripts). Most official images
  (nginx, redis, postgres…) need NO builder at all — the package already exists.
- **Multi-stage builds** → one `BUILDER` per stage; a later builder stages an earlier
  one with `COPY ${earlier}/ .`; `COPY --from=X` → `COPY ${X}/path dest`.
- **`RUN --mount=type=cache,target=D`** → delete the declaration. Builder
  workspaces persist by default, while no workspace byte enters a chain key. Use
  narrow `COPY ${build}/path` consumers and sample with `cix build --cold`.
- **`curl|wget + checksum` downloads** → a `FETCH` (the pin is enforced for you).
- **`VOLUME /data`** → `STATEDIR /var/lib/<name>` and point the app there (env/flag).
  Writable role dirs in SERVICE/APP blocks: `STATEDIR /var/lib/…` (persistent),
  `CACHEDIR /var/cache/…`, `LOGS /var/log/…`, `CONFIG /etc/…`, `RUNDIR /run/…`.
  Apps that want writable XDG/home dirs get a STATEDIR/CACHEDIR dir plus env vars
  (`XDG_DATA_HOME=…`) pointing into it.
- **`EXPOSE N`** → `PORT http = N` (or via env). **`USER`/`gosu`/`su-exec`/`tini`**
  → delete: systemd runs the service as an unprivileged dynamic user and is the init.
- **A Dockerfile's `COPY` of sibling files** (entrypoint.sh, config files) refers to
  its build context: fetch those files from the same repository directory as the
  Dockerfile itself before converting — you need to read them.
- **`LABEL` lines** → drop them. Provenance labels (source/version/revision) are
  superseded by the lock and the closure (content-addressed provenance); display
  labels (title/description) have no cix home yet — a manifest annotations story is
  recorded but deliberately unbuilt.
- **ENTRYPOINT shell scripts** → read them; port the essential env/flag setup into
  ENV/EXEC lines. Scripts that chown/mkdir/sed files at startup are usually
  replaceable by role dirs + a config file assembled at build time (`FILE`/`COPY`).

## The check

Ship a `check.sh` next to your Cixfile with two modes, same probe body:
`./check.sh docker` (docker build+run the original, probe, teardown) and
`./check.sh cix` (`cix build .` + `cix run <item>`, same probe, teardown).
The probe proves the service does its one central thing (HTTP 200, redis PING,
`--version`…), bounded by timeouts, exit 0/1. Both modes must pass.

Practicalities for the cix mode: `cix run` uses the system service manager and needs
root — run it via passwordless `sudo`; the command prints the transient unit name;
tear down with `sudo systemctl stop <unit>` (and stop the `cix-run.slice` if you
started several).
