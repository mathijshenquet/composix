# The Cixfile

*Status: D47 blocks and binders are implemented: named builders, services, one-shot apps,
asset-only items, local and remote source binders, and pinned network fetches.*

A Cixfile turns a directory into one or more composix artifacts. It is Dockerfile-shaped, so
the common operations are recognizable, but its boundaries are explicit: builders create
snapshots; `SERVICE`, `APP`, and `ITEM` blocks assemble independent runtime artifacts. The
`.nix` escape hatch remains first-class for custom derivations.

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
FROM . AS src

BUILDER build
COPY ${src}/index.html srv/www/index.html
COPY ${src}/nginx.conf etc/nginx/nginx.conf

SERVICE nginx
COPY ${build}/srv/www/index.html srv/www/index.html
COPY ${build}/etc/nginx/nginx.conf etc/nginx/nginx.conf
LINK /etc/nginx/mime.types ${pkgs.nginx}/conf/mime.types
EXEC ${pkgs.nginx}/bin/nginx -c /etc/nginx/nginx.conf -e stderr
PORT http = 8080
CACHE /var/cache/nginx
RUNDIR /run/nginx
```

`FROM . AS src` gives the Cixfile directory a name. Bare relative sources remain legal:
`COPY index.html srv/www/index.html` means exactly the same thing as
`COPY ${src}/index.html srv/www/index.html`. Explicit binders become useful when a file has
more than one possible origin.

## Structure

Prelude declarations come first:

| declaration | meaning |
| --- | --- |
| `FROM <flakeref> AS <name>` | bind a locked package universe when the ref is nixpkgs, or a locked source tree for another remote flake |
| `FROM . AS <name>` | optionally name the Cixfile directory; it is local input and is not lock-pinned |
| `FETCH <name> <command…>` | run a networked command in an empty workdir and bind its pinned output snapshot |

Blocks then declare work and outputs:

| block | allowed directives | result |
| --- | --- | --- |
| `BUILDER <name>` | `COPY`, `FETCH`, `RUN`, `CACHE`, `PATH` | an immutable named workdir snapshot |
| `SERVICE <name>` | `COPY`, `FILE`, `SCRIPT`, `LINK`, `PATH`, `EXEC`, `SETUP`, `ENV`, `PORT`, `LISTENER`, `STATE`, `CACHE`, `LOGS`, `CONFIG`, `RUNDIR`, `JIT`, `OUTBOUND` | a long-running service artifact |
| `APP <name>` | `COPY`, `FILE`, `SCRIPT`, `LINK`, `EXEC`, `ENV`, `OUTBOUND`, `STATE`, `CACHE` | a run-to-completion app artifact |
| `ITEM <name>` | `COPY`, `FILE`, `LINK` | an asset-only artifact with no executable |

Names share one namespace and references point backward. A builder cannot copy from itself,
and a declaration cannot refer to a later declaration. Errors report both the bad line and,
where useful, the first declaration line.

### Unified `COPY`

`COPY <source> <destination>` is used in builders and artifact blocks:

```dockerfile
COPY README.md share/README.md
COPY ${src}/config.toml etc/my-app/config.toml
COPY ${download}/archive.tar share/archive.tar
COPY ${compile}/bin/server bin/server
COPY ${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt etc/ssl/cert.pem
```

A source is either a bare relative path in the implicit Cixfile-directory context or a path
under a declared binder. Only remote sources need an explicit `FROM` binder. Package
references use `${universe.attrpath}`; source, fetch, and builder references use
`${binder}/path`. Copying a binder without `/path` copies its whole root. Destinations are
clean paths relative to the builder or artifact root; `.` means the whole root.

There is no magic `${build}` namespace and `TAKE` is gone. A migrated file should name its
builder, normally `BUILDER build`, and use `COPY ${build}/path destination`. The parser emits
that migration directly instead of turning either spelling into a mysterious unknown name.

`FILE <destination> <<EOF` adds an inline interpolated file, `SCRIPT` adds an executable
script with a shell header, and `LINK <destination> <target>` adds a symlink. `SCRIPT` is
available only where execution is meaningful; asset-only `ITEM` deliberately excludes it.

### Package universes and source binders

`AS` is required. The documented nixpkgs spelling is:

```dockerfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
```

`${pkgs.postgresql}/bin/postgres` and
`${pkgs.python3Packages.black}/bin/black` resolve package attributes from that locked
revision. Other remote flakes are source trees, not package universes:

```dockerfile
FROM github:owner/project/v1.2.3 AS upstream
```

The lock records remote revisions and NAR hashes. `FROM .` and the implicit local context are
not pinned: their bytes enter the normal content-addressed snapshot. `$VAR` remains runtime
environment syntax and is valid in `EXEC` and `SETUP`; `${…}` is resolved while building.
Copied file content is always verbatim.

## Builders, `RUN`, `FETCH`, and caches

Each `BUILDER` starts empty. Its directives form a linear chain of immutable snapshots, and
later blocks may copy from the final snapshot by name:

```dockerfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

BUILDER compile
PATH ${pkgs.bash}/bin ${pkgs.cargo}/bin ${pkgs.rustc}/bin ${pkgs.gcc}/bin
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
CACHE target
COPY src src
RUN cargo build --release --locked --offline \
    && mkdir -p output \
    && cp target/release/app output/app

APP app
COPY ${compile}/output/app bin/app
EXEC bin/app
```

Before `RUN`, composix hashes the command, fixed environment, incoming snapshot, and offered
store closure. A matching live memo entry is reused. Bubblewrap exposes only that closure and
the workdir; internet sockets are denied. The environment is cleared and rebuilt with the
declared `PATH`, `HOME=/work`, `SOURCE_DATE_EPOCH=1`, `TZ=UTC`, `LC_ALL=C`, `TMPDIR=/tmp`,
and umask 022.

`CACHE target` belongs to its builder. Its host-local identity includes the builder name, so
two builders using `target` do not accidentally share state. Cache contents are writable
during `RUN` but excluded from memo keys, snapshots, and artifacts. Copy wanted results to a
non-cache path before the command ends. `cix build --no-cache` bypasses `RUN` memo hits and
uses empty caches.

There are two intentionally different fetch forms:

- Top-level `FETCH payload <command…>` runs in an empty workdir and creates a reusable binder.
  Its memo identity is the resolved command; unrelated builder state does not invalidate it.
- A `FETCH <command…>` inside a builder runs with that builder's incoming workdir and advances
  its snapshot.

Both are the only network-enabled build steps. Their output hash is trusted on first use and
written to `Cixfile.lock`; a later forced result must match unless that pin is explicitly
updated.

## Artifact kinds

Every block produces its own store item and bare v4 manifest.

`SERVICE` is the full long-running contract. `EXEC` is its main process; `SETUP` is an
idempotent pre-start hook. `PORT` and `LISTENER` grant inbound networking. Role directives
declare systemd-managed writable paths. `JIT` grants writable-and-executable memory, and
`OUTBOUND` declares outward network access.

`APP` is a one-shot command. `cix run` starts it as `Type=oneshot`, waits, streams its
output, and returns the command's exit status. Apps have no setup hooks, ports, listeners,
health checks, JIT grant, or log/config/run role directories.

`ITEM` contains files and links only. Its manifest says `kind: "item"` and has no `exec`.
`cix run` refuses it with an asset-only diagnostic. This makes static bundles usable as
inputs without pretending they are processes.

Relative copied destinations live at the artifact root and are projected at their native
runtime path. Package binaries remain direct store references; `PATH` is for tools invoked by
service scripts. `LINK` is best used for package-owned assets such as nginx's `mime.types`.

When a Cixfile has several artifacts, `cix build . -t v1` tags them as
`<artifact-name>:v1`. `-t name:tag` is accepted only for a single-artifact build.

## Where this is honestly not a Dockerfile

- `FROM` binds package or source inputs; it never inherits a root filesystem or image layer.
- `RUN` is networkless and closure-limited. Network access is a visible, hash-pinned `FETCH`.
- Builders are named data-flow nodes, not mutable images. Cross-builder transfer is an
  explicit `COPY`.
- Artifacts are sparse roots plus exact Nix store references. There is no `ADD`, URL/tar
  auto-extraction, `.dockerignore`, arbitrary `USER`, or secret mount syntax.
- `ENV` declares an operator-facing contract. `PORT` is an enforced grant, not documentation.
- `SERVICE`, `APP`, and `ITEM` distinguish daemons, one-shot commands, and inert assets in the
  manifest, so invalid combinations fail while parsing rather than at runtime.

That narrower language is deliberate: the generated manifest is both the runtime description
and the capability policy.
