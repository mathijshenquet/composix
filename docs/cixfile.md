# The Cixfile

*Status: v1 designed, implementation in progress. This page describes the design; the
directive set is small on purpose and will grow consciously.*

A Cixfile turns a directory into a runnable composix item — Dockerfile-shaped, so your hands
already know it, without writing nix. v1 is deliberately an **assembly** format: it wraps
software that already exists (nixpkgs packages, your files, a service contract) into an item.
It does not build your code — that's the ecosystem-builder story (later) or the `.nix` escape
hatch (always).

## A Cixfile, next to the Dockerfile you'd write instead

```dockerfile
# Dockerfile (docker)
FROM nginx:1.27
COPY index.html /usr/share/nginx/html/
COPY nginx.conf /etc/nginx/nginx.conf
EXPOSE 8080
```

```dockerfile
# Cixfile (composix) — examples/nginx/Cixfile in this repo, verbatim
PKG nginx

FILE www/index.html <<EOF
<h1>hello from composix</h1>
EOF

LINK bin/nginx ${nginx}/bin/nginx
LINK etc/mime.types ${nginx}/conf/mime.types

FILE etc/nginx.conf <<EOF
daemon off;
pid /run/nginx/nginx.pid;
error_log stderr info;
events { }
http {
  include /item/etc/mime.types;
  access_log off;
  client_body_temp_path /var/cache/nginx/body;
  proxy_temp_path /var/cache/nginx/proxy;
  fastcgi_temp_path /var/cache/nginx/fastcgi;
  uwsgi_temp_path /var/cache/nginx/uwsgi;
  scgi_temp_path /var/cache/nginx/scgi;
  server {
    listen 8080;
    root /item/www;
  }
}
EOF

SERVICE nginx
EXEC bin/nginx -c /item/etc/nginx.conf -e stderr
PORT http = 8080
CACHE /var/cache/nginx
RUNDIR /run/nginx
```

(A sibling file via `COPY index.html www/index.html` would work identically to the inline
`FILE`; this example inlines everything so the repo carries one self-contained file. Note the
executable is `LINK`ed into the item and exec'd as `bin/nginx` — cross-package references are
always pulled in via `LINK`.)

The Cixfile is longer — because it states things the Dockerfile leaves implicit in the base
image (the config, the writable paths, what the process actually is). Nothing here is
boilerplate; every line is contract.

## Directives

| directive | what it does | closest docker |
| --- | --- | --- |
| `PKG <attr>` | bring a nixpkgs package into scope; enables `${attr}` in directive arguments | `FROM` (spiritually — see below) |
| `COPY <src> <dst>` | copy a sibling file into the item, **verbatim, never substituted** | `COPY` (identical intent) |
| `FILE <dst> <<EOF` | inline file, `${…}`-interpolated at build time | `COPY <<EOF` (buildkit heredoc) |
| `SCRIPT <dst> <<EOF` | inline executable script (shebang added) | — |
| `LINK <dst> <target>` | symlink into another package | — (see "the LINK shift") |
| `SERVICE <name>` | begin a service block (an item can hold several) | — (docker splits this over image + compose) |
| `EXEC <argv…>` | the process (`$VAR` = runtime env) | `ENTRYPOINT`/`CMD` |
| `SETUP <argv…>` | pre-start hook, every start, must be idempotent | — (the docker-entrypoint.sh convention, promoted to contract) |
| `ENV NAME [= default] [required] [secret]` | declare the config surface | `ENV` (compatible for `ENV FOO = bar`, but declares, not just sets) |
| `PORT name = $VAR` / `PORT name = 8080` | declare a listening port (env-bound or fixed) — this *grants* network | `EXPOSE` (which grants nothing) |
| `STATE` `CACHE` `LOGS` `CONFIG` `RUNDIR` | writable dirs by role, at the path the app expects | `VOLUME` (roleless) |
| `JIT` | the service maps writable+executable memory | — (docker allows W+X silently) |

Two interpolation worlds, one rule: `${name}` is **build time** (only in directive arguments,
never in file contents); `$VAR` is **runtime** (only in `EXEC`/`SETUP`). File contents are
verbatim — they reference `/item/…`, which is where the runtime mounts the item, read-only, at
a stable path. That stable path is what lets configs be plain files instead of templates.

## Where this is honestly not a Dockerfile

The syntax is familiar; the model underneath is different, and pretending otherwise would
cost you more later. The differences, biggest first:

**There is no `RUN`.** Docker's core primitive — run a command in a container, snapshot the
filesystem — does not exist here and is not planned. You cannot `apt-get install`, `curl`, or
compile mid-file. That primitive is exactly where Dockerfile reproducibility dies (the same
Dockerfile builds differently tomorrow); refusing it is where composix reproducibility comes
from. Building *your own* code is the ecosystem-builder story (cargo/pnpm/uv lockfiles,
planned) or a `.nix` file (today).

**There is no `FROM`, no layers, no inheritance.** Docker composes by stacking filesystems;
a Cixfile composes by *referencing packages*. `PKG nginx` doesn't copy nginx into your item —
nginx stays in the nix store, your item points at it. Consequences: no base image to patch or
scan, no `ONBUILD`, no layer-cache ordering games, no image-size golf (deduplication is
store-wide and automatic). The role of "which base am I on" is played by the nixpkgs pin in
`Cixfile.lock`.

**The LINK shift — you assemble a view, not a filesystem.** This is the real mental-model
change. A docker image is a whole root filesystem: software you use lives at global paths
(`/usr/bin/…`, `/etc/…`) because it was *installed* there. A composix item is a small
directory of your own files plus symlinks into other packages (`LINK etc/mime.types
${nginx}/conf/mime.types`). At runtime `/item` makes this feel docker-like again — stable
absolute paths — but at authoring time you think in references, not installations. In
exchange: your item is kilobytes, its dependencies are exact and inspectable
(`nix path-info -r`), and two items sharing nginx share it fully.

**`ENV` declares a config surface, not just a value.** `ENV FOO = bar` behaves like docker's.
But `ENV DB_URL required secret` is something a Dockerfile cannot say: the runtime refuses to
start without it, and (compose era) delivers it as a credential rather than a plain env var.
The spec is a contract, not metadata.

**`SERVICE` is first-class, and there can be several.** Docker needs an image *and* a compose
file to express "this artifact runs as these services, configured so". The Cixfile says it in
one place, and `cix run item#service` picks one.

**Declaring is granting.** `PORT` is not documentation like `EXPOSE` — it is what opens the
network (and below 1024, what grants the capability). Everything undeclared is denied:
read-only filesystem, no network, no capabilities. A Dockerfile inherits docker's defaults;
a Cixfile *is* the security policy.

**Determinism is enforced, not hoped for.** `cix build` pins nixpkgs in `Cixfile.lock`
(revision + content hash, `--update-lock` to roll). Same Cixfile + same lock = same item,
bit-for-bit, on any machine. There is no docker equivalent — `docker build` today and
tomorrow are different images.

## When to drop to `.nix`

Custom builds, exotic packaging, anything the directive set can't say: write `default.nix`
instead — the escape hatch is first-class (D4), and the two coexist per-example in this very
repo. A Cixfile compiles to nix underneath, so graduating from one to the other changes
notation, not concepts.
