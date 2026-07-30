# The Cixfile

*Status: v1 implemented through D40/D41: RUN/FETCH, persistent build caches, multi-item
plucks, and one bare v4 manifest per item.*

A Cixfile turns a directory into a runnable composix item — Dockerfile-shaped, so your hands
already know it, without writing nix. It can assemble software that already exists in the
Nix store, or build in a deliberately narrow linear workdir chain. The `.nix` escape hatch
remains first-class for custom derivations.

## A Cixfile, next to the Dockerfile you'd write instead

```dockerfile
# Dockerfile (docker)
FROM nginx:1.27
COPY index.html /usr/share/nginx/html/
COPY nginx.conf /etc/nginx/nginx.conf
EXPOSE 8080
```

```dockerfile
# Cixfile (composix) — examples/pack/nginx/Cixfile in this repo, verbatim
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

COPY index.html /srv/www/index.html

COPY nginx.conf /etc/nginx/nginx.conf

ITEM nginx
TAKE ${build}/srv/www/index.html /srv/www/index.html
TAKE ${build}/etc/nginx/nginx.conf /etc/nginx/nginx.conf
LINK /etc/nginx/mime.types ${pkgs.nginx}/conf/mime.types
EXEC ${pkgs.nginx}/bin/nginx -c /etc/nginx/nginx.conf -e stderr
PORT http = 8080
CACHE /var/cache/nginx
RUNDIR /run/nginx
```

`nginx.conf` sits next to the Cixfile and is copied unchanged:

```nginx
# examples/pack/nginx/nginx.conf
daemon off;
pid /run/nginx/nginx.pid;
error_log stderr info;
events { }
http {
  include /etc/nginx/mime.types;
  access_log off;
  client_body_temp_path /var/cache/nginx/body;
  proxy_temp_path /var/cache/nginx/proxy;
  fastcgi_temp_path /var/cache/nginx/fastcgi;
  uwsgi_temp_path /var/cache/nginx/uwsgi;
  scgi_temp_path /var/cache/nginx/scgi;
  server {
    listen 8080;
    root /srv/www;
  }
}
```

`COPY` puts checked-in bytes into the build workdir. `TAKE` is the explicit boundary that
plucks only the two runtime files into this item. `LINK` brings in the package-owned
`mime.types` asset; the one-off nginx executable stays a direct store reference.

The Cixfile is longer — because it states things the Dockerfile leaves implicit in the base
image (the config, the writable paths, what the process actually is). Nothing here is
boilerplate; every line is contract.

## Directives

| directive | what it does | closest docker |
| --- | --- | --- |
| `FROM <flakeref> AS <name>` | bind a pinned package universe; this is **not** filesystem/layer inheritance | `FROM` (truthful meaning differs) |
| `PATH <dir>…` | ordered tool directories; prelude paths are absolute/package-based, while an ITEM may add paths relative to its own filesystem | `PATH` |
| `CACHE <dir>` before the first `ITEM` | persist a writable build directory across RUNs, outside memo keys, snapshots, and store items | `RUN --mount=type=cache` |
| `COPY <src> <dst>` | copy a regular sibling file into the current workdir snapshot, **verbatim, never substituted** | `COPY` (identical intent) |
| `FETCH <command…>` | run the only network-enabled build step; TOFU-pin its output NAR hash in `Cixfile.lock` | `RUN --network=default` (fixed-output here) |
| `RUN <command…>` | run a memoized command in a networkless, offer-only sandbox | `RUN` |
| `ITEM <name>` | begin one store item and its one service definition | one image/config pair |
| `TAKE <build-path> <item-path>` | pluck a file or directory from the final workdir into this item | `COPY --from` (without named stages) |
| `FILE <dst> <<EOF` | add an inline, `${…}`-interpolated file to the current item | `COPY <<EOF` (buildkit heredoc) |
| `SCRIPT <dst> <<EOF` | inline executable script (shebang added) | — |
| `LINK <dst> <target>` | symlink into another package | — (see "the LINK shift") |
| `EXEC <argv…>` | the process (`$VAR` = runtime env) | `ENTRYPOINT`/`CMD` |
| `SETUP <argv…>` | pre-start hook, every start, must be idempotent | — (the docker-entrypoint.sh convention, promoted to contract) |
| `ENV NAME [= default] [required] [secret]` | declare the config surface | `ENV` (compatible for `ENV FOO = bar`, but declares, not just sets) |
| `PORT name = $VAR` / `PORT name = 8080` | declare a listening port (env-bound or fixed) — this *grants* network | `EXPOSE` (which grants nothing) |
| `STATE` `CACHE` `LOGS` `CONFIG` `RUNDIR` inside an `ITEM` | runtime-writable dirs by role, at the path the app expects | `VOLUME` (roleless) |
| `JIT` | the service maps writable+executable memory | — (docker allows W+X silently) |
| `OUTBOUND` | declare that this service initiates outward network connections; enforcement comes with pod netns | — |

### Scripts and tools

Keep checked-in shell scripts as verbatim sibling files and `COPY` them to native projected
paths. Invoke a non-executable copied script through a direct shell path, such as
`EXEC ${pkgs.bash}/bin/sh /opt/app/start`. Declare the tools it calls with `PATH`, then scripts can
use ordinary `initdb`, `postgres`, `id`, or `mkdir`; the generated PATH default is still an
ordinary, operator-overridable `ENV` value. `LINK` is for non-executable assets a script needs,
such as a preload library or a package data tree. This removes item-local bin symlinks and
`$(dirname "$0")` plumbing while leaving checked-in scripts verbatim and reviewable.

### Package interpolation

Every Cixfile begins by naming its package universes: `FROM <flakeref> AS <name>`. `AS` is
required—there is no implicit `pkgs` binding—and a `FROM` must appear before that namespace is
used. `nixpkgs` is the documented registry spelling; otherwise use
`github:owner/repo[/ref]` or an HTTPS tarball URL. `${<name>.<attrpath>}` is **build time**: it
resolves an arbitrary attribute path from that named universe's revision locked in
`Cixfile.lock`, in directive arguments and inline `FILE`/`SCRIPT` bodies. For example,
`${pkgs.postgresql}/bin` and `${pkgs.python3Packages.black}/bin/black` are both valid. Bare
`${name}` is an error; an unknown namespace lists the declared ones. `$VAR` remains **runtime**
and is only valid in `EXEC`/`SETUP`; `COPY` content is always verbatim.

References define dependencies: there is no package declaration to keep in sync. Each namespaced
reference becomes part of the built item's Nix closure, which is the authoritative manifest—inspect
it with `nix path-info -r`. Destinations beginning with `/` are projected read-only at that native
path; bare-relative destinations stay inside the item for executable and script targets. Native
paths let copied configs remain plain files rather than templates.

### RUN, FETCH, and the linear workdir

`COPY`, `FETCH`, and `RUN` form one linear chain. Each step starts from the immutable workdir
snapshot produced by the preceding step. This is the cache boundary: copy dependency manifests,
fetch and cook dependencies, then copy frequently edited source so a source edit cannot invalidate
the earlier cook.

```dockerfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
PATH ${pkgs.bash}/bin ${pkgs.cargo}/bin ${pkgs.rustc}/bin ${pkgs.gcc}/bin

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
CACHE target
COPY src/main.rs src/main.rs
RUN cargo build --release --locked --offline && mkdir -p output && cp target/release/app output/app

ITEM app
TAKE output/app bin/app
EXEC bin/app
```

`TAKE` accepts either a clean workdir-relative source or `${build}/…`; `${build}` is valid
only in that source position. Relative `EXEC`, `LINK`, and item `PATH` values resolve against
the item filesystem, so build-only cargo files never leak into the runtime item.

Before a command, `cix build` hashes the command, effective fixed/declared environment, incoming
workdir NAR, and the complete offered store closure. A live matching entry in the lock's `memo`
section is reused. On a miss, bubblewrap exposes only the closure of package paths referenced by
the Cixfile, read-only at their real `/nix/store` paths, plus the writable workdir and minimal
`/proc`, `/dev`, `/tmp`, and `/etc`. PID, UTS, IPC, cgroup, and user namespaces are fresh.
`RUN` cannot create internet sockets. The preferred mechanism is a fresh network namespace with
only loopback. On hosts whose user-namespace policy lets bubblewrap build the sandbox but prevents
it from configuring that namespace, `cix build` instead applies a seccomp filter that returns
`EPERM` from `socket(2)` and `socketpair(2)` for `AF_INET`, `AF_INET6`, and `AF_PACKET`;
local-only `AF_UNIX` and `AF_NETLINK` remain available. The fallback also denies
`io_uring_setup(2)`, because io_uring can otherwise create a socket without calling `socket(2)`.
Non-native syscall ABIs are rejected rather than allowed around the filter. The isolation promise
is identical in both tiers, and a failed fallback RUN notes that localhost was unavailable.
`FETCH` deliberately shares host networking and receives only the host resolver files.

The environment is cleared and rebuilt from declared defaults plus `PATH`, `HOME=/work`,
`SOURCE_DATE_EPOCH=1`, `TZ=UTC`, `LC_ALL=C`, `TMPDIR=/tmp`, and umask 022. A successful step is
added to the Nix store as a NAR snapshot. FETCH's first output hash is trusted-on-first-use and
pinned; a later forced/refetched result must match or the build fails. Plain `--update-lock`
deliberately refreshes the pin.

A prelude `CACHE target` is mounted writable at `/work/target` for every RUN. Its contents
persist under a host-local cache identity, but are excluded from the memo key, workdir
snapshot, Nix store, and final items. The build must copy wanted results to a non-cache path
before the RUN ends, then TAKE them. `cix build --no-cache` bypasses RUN memo hits and ignores
all CACHE directories, providing the clean-rebuild soundness hook.

This sandbox requires bubblewrap and unprivileged user namespaces. If the host forbids them, v0
refuses the step with a loud error; it never silently drops isolation. There is no tracer in v0:
the complete offered closure is the sound input set. Observed-read pruning remains a possible
optimization, not part of correctness.

## Where this is honestly not a Dockerfile

The syntax is familiar; the model underneath is different, and pretending otherwise would
cost you more later. The differences, biggest first:

**`RUN` is narrower and `FETCH` is explicit.** Docker lets every RUN step see the network and
ambient image filesystem unless configured otherwise. Cixfile RUN sees only declared store
closures and the current workdir, with no network. FETCH is a separate, visibly impure-looking
keyword whose output is hash-pinned. v0 has one chain, persistent declared cache directories,
no named/multi-stage graph or secret mounts, and no read tracer.

**`FROM` is not layer inheritance.** Docker composes by stacking filesystems; a Cixfile's
`FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs` binds the package universe visible as `pkgs`.
`${pkgs.nginx}` doesn't copy nginx into your item—nginx stays in the nix store, your item points
at it. Consequences: no base image to patch or scan, no `ONBUILD`, no layer-cache ordering games,
no image-size golf (deduplication is store-wide and automatic). The lock pins each named universe.

**The LINK shift — assets, not executable shims.** A docker image is a whole root filesystem:
software you use lives at global paths (`/usr/bin/…`, `/etc/…`) because it was *installed*
there. A composix item is a small directory of your own files plus asset symlinks into other
packages (`LINK /etc/nginx/mime.types ${pkgs.nginx}/conf/mime.types`). Use `PATH` for tools called by
scripts, or a direct `${pkgs.<attrpath>}/bin/tool` for a trivial one-off executable; the compiler records
the real store path in the spec. Projected destinations appear at their native absolute paths at
runtime, while at authoring time you think in references, not installations. In exchange: your
item is kilobytes, its dependencies are exact and inspectable (`nix path-info -r`), and two
items sharing nginx share them fully.

**`ENV` declares a config surface, not just a value.** `ENV FOO = bar` behaves like docker's.
But `ENV DB_URL required secret` is something a Dockerfile cannot say: the runtime refuses to
start without it, and (compose era) delivers it as a credential rather than a plain env var.
The spec is a contract, not metadata.

**`ITEM` is first-class, and there can be several.** Every ITEM becomes its own content-addressed
store item with one bare v4 manifest. `cix build . -t v1` tags them as `<item-name>:v1`;
`-t name:tag` is accepted only when the Cixfile has one ITEM. Shared package closures still
deduplicate at the Nix-store level.

**Declaring is granting.** `PORT` is not documentation like `EXPOSE` — it is what opens the
network (and below 1024, what grants the capability). Everything undeclared is denied:
read-only filesystem, no network, no capabilities. A Dockerfile inherits docker's defaults;
a Cixfile *is* the security policy.

**Inputs are pinned and realizations are explicit.** `cix build` pins every `FROM` input in
`Cixfile.lock`, keyed by its `AS` name with URL, revision, and content hash. Use
`cix build --update-lock` to roll all inputs or `cix build --update-lock pkgs` to roll one.
FETCH outputs are pinned there too, and RUN memo entries name their output NAR hash and store
path. Repeating a deterministic chain yields the same final item; v0 does not pretend every
upstream tool is deterministic, and remote realization sharing/sampled rebuild policy remains
publish-era work. Docker has no equivalent input-and-realization record.

## When to drop to `.nix`

Custom build graphs, exotic packaging, anything the directive set can't say: write `default.nix`
instead — the escape hatch is first-class (D4), and the two coexist per-example in this very
repo. A Cixfile compiles to nix underneath, so graduating from one to the other changes
notation, not concepts.
