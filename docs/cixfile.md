# The Cixfile

*Status: D47 blocks and binders through D65 are implemented: named persistent builders,
narrow consumed-path records, package IMPORT unions, declared or TOFU-pinned network fetches,
directive continuations, RUN heredocs, and full-line comments.*

A Cixfile turns a directory into one or more composix artifacts. It is Dockerfile-shaped, so
the common operations are recognizable, but its boundaries are explicit: builders do
networked or executable build work; `SERVICE` and `APP` blocks assemble independent runtime
artifacts; `ITEM` assembles a pure store tree. The `.nix` escape hatch remains first-class for
custom derivations.

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

SERVICE my-nginx
COPY ${src}/index.html /srv/www/index.html
COPY ${src}/nginx.conf /etc/nginx/nginx.conf
LINK ${pkgs.nginx}/conf/mime.types /etc/nginx/mime.types
EXEC ${pkgs.nginx}/bin/nginx -c /etc/nginx/nginx.conf -e stderr
PORT http = 8080
CACHEDIR /var/cache/nginx
RUNDIR /run/nginx
```

`FROM . AS src` gives the Cixfile directory a name. Bare relative sources remain legal:
`COPY index.html /srv/www/index.html` means exactly the same thing as
`COPY ${src}/index.html /srv/www/index.html`. Explicit binders become useful when a file has
more than one possible origin.

## Structure

Prelude declarations come first:

| declaration | meaning |
| --- | --- |
| `FROM <flakeref> AS <name>` | bind a locked package universe or source tree |
| `FROM <index-ref:tag> AS <name>` | bind a lock-pinned cix item as a source tree |
| `FROM . AS <name>` | optionally name the Cixfile directory; it is local input and is not lock-pinned |
| `FETCH <name> [EXPECT <sri-hash>] <command…>` | run a networked command in an empty workdir and bind its pinned output |

Blocks then declare work and outputs:

| block | allowed directives | result |
| --- | --- | --- |
| `BUILDER <name>` | `IMPORT`, `COPY`, `FETCH`, `RUN`, `ENV` | a persistent workspace whose consumed outputs are recorded individually |
| `SERVICE <name>` | `COPY`, `FILE`, `LINK`, `EXEC`, `SETUP`, `ENV`, `PORT`, `LISTENER`, `STATEDIR`, `CACHEDIR`, `LOGSDIR`, `CONFIGDIR`, `RUNDIR`, `GRANT` | a long-running service artifact |
| `APP <name>` | `COPY`, `FILE`, `LINK`, `EXEC`, `ENV`, `GRANT`, `STATEDIR`, `CACHEDIR` | a run-to-completion app artifact |
| `ITEM <name>` | `COPY`, `FILE`, `LINK` | a pure store tree, with no manifest |

Names share one namespace and references point backward. A builder cannot copy from itself,
and a declaration cannot refer to a later declaration. Errors report both the bad line and,
where useful, the first declaration line.

**A BUILDER exists only when there is RUN or FETCH work to do.** Pure assembly belongs
directly in the `SERVICE`, `APP`, or `ITEM` that consumes the sources; routing local files
through a COPY-only builder adds a name without adding a boundary.

### Unified `COPY`

`COPY <source> <destination>` is used in builders and artifact blocks. A BUILDER destination is
workdir-relative; a SERVICE, APP, or ITEM destination is absolute in that item's runtime world:

```dockerfile
COPY README.md .
COPY ${src}/config.toml /etc/my-app/config.toml
COPY ${download}/archive.tar /share/archive.tar
COPY ${compile}/bin/server /bin/server
COPY ${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt /etc/ssl/cert.pem
```

A source is either a bare relative path in the implicit Cixfile-directory context or a path
under a declared binder. Only remote sources need an explicit `FROM` binder. Package
references use `${universe.attrpath}`; source, cix-item, fetch, and builder references use
`${binder}/path`. Copying a binder without `/path` copies its whole root. A whole-builder read
is legal but expensive: the entire left-behind tree becomes one consumed object and any
changed byte invalidates that consumer. Prefer narrow paths for build artifacts. BUILDER
destinations are clean workdir-relative paths, and `.` means that whole workdir. SERVICE, APP,
and ITEM destinations must start with `/`: this is the item's runtime root, stored
item-relatively beneath the resulting artifact. `COPY source /` is the absolute spelling for the
whole item root.

Prefer one directory COPY when its contents move as a unit:

```dockerfile
COPY ${src}/rust/ .
```

Enumerate files only when the separation deliberately creates a memo boundary, such as
copying dependency manifests before source. Structural globs such as `**/Cargo.toml` are not
implemented; the known manifest-first cases are already expressible without a glob language.

There is no magic `${build}` namespace and `TAKE` is gone. A migrated file should name its
builder, normally `BUILDER build`, and use `COPY ${build}/path /destination`. The parser emits
that migration directly instead of turning either spelling into a mysterious unknown name.

`FILE <destination> <<EOF` adds an inline interpolated file. Use it when content must contain
a build-time store path; ordinary files and scripts should remain real source files copied
with `COPY`. Invoke a copied script through an explicit package shell:

```dockerfile
COPY start /bin/start
EXEC ${pkgs.bash}/bin/sh /bin/start
```

`SCRIPT` was dropped by D55; the parser reports this migration instead of accepting an alias.
`LINK <target> <linkpath>` adds a symlink. LINK follows both `ln -s TARGET LINKNAME` and
COPY's source-first reading: where from, then where it lands.

### Lines, comments, and heredocs

A backslash continues any directive onto the next physical line:

```dockerfile
IMPORT ${pkgs.bash} ${pkgs.cargo} \
    ${pkgs.rustc} ${pkgs.gcc}
```

Errors retain physical Cixfile line numbers. A line whose first non-whitespace character is
`#` is a comment; end-of-line comments are deliberately not special because RUN and FETCH
arguments are shell text.

RUN accepts either a one-line command or a builder-shell heredoc:

```dockerfile
RUN <<BUILD
mkdir -p output
cargo build --release --locked --offline
cp target/release/app output/app
BUILD
```

The complete body is the command and therefore part of the same memo key as a one-line RUN.
Shell comments inside the body belong to the shell. `${…}` remains build-time interpolation;
use `$${…}` when the shell itself must receive a braced expansion.

### Package universes, source binders, and cix-item binders

`AS` is required. The documented nixpkgs spelling is:

```dockerfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
```

`${pkgs.postgresql}/bin/postgres` and
`${pkgs.python3Packages.black}/bin/black` resolve package attributes from that locked
revision. A flakeref is a fetch address, not a promise that the tree contains a `flake.nix`.
Its accepted spellings start with `github:`, `git+`, `path:`, `tarball+`, `.`, or `./`.
Other remote trees are source binders:

```dockerfile
FROM github:owner/project/v1.2.3 AS upstream
```

The third FROM input is an explicit-tag index ref:

```dockerfile
FROM cix.my-org.com/acme/web-vault:v3 AS webvault

SERVICE web
COPY ${webvault}/share/web-vault /share/web-vault
EXEC /bin/true
```

It resolves through the cix index (qualified refs fetch when needed), verifies its NAR hash,
and records `artifacts.<ref> = { storePath, narHash }` in `Cixfile.lock`. The tag may move;
the lock keeps this build on the selected store path until `cix build --update-lock webvault`.
A missing local ref says to pull or tag it first. Item binders are trees: use only
`${webvault}/path` in `COPY` or `LINK`. `${webvault.attr}` is rejected because index refs never
create package namespaces (D65(c)), and `IMPORT ${webvault}` is deliberately deferred (D65(d)).

Disambiguation is deliberately mechanical: a known flakeref spelling is a flakeref; every
other FROM token must be a valid index ref with an explicit `:tag`. There is no default tag.
An untagged `family/web-vault` therefore gives the same `:latest is not a thing here` error as
the rest of the index surface.

The lock records remote revisions/NAR hashes and cix-item store paths/NAR hashes. `FROM .` and
the implicit local context are not pinned: the content of each declared COPY source enters its
builder's chain key. `$VAR` remains runtime environment syntax and is valid in `EXEC` and
`SETUP`; `${…}` is resolved while building. Copied file content is always verbatim.

## Builders, `IMPORT`, `RUN`, and `FETCH`

Each `BUILDER` has a persistent workspace. Its directives form a pure key chain, and later
blocks consume named paths from the tree it leaves behind:

```dockerfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

BUILDER compile
IMPORT ${pkgs.bash} ${pkgs.cargo} ${pkgs.rustc} ${pkgs.gcc} ${pkgs.cacert}
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
FETCH cargo fetch --locked
COPY src/ src
RUN <<BUILD
cargo build --release --locked --offline
BUILD

APP app
COPY ${compile}/target/release/app /bin/app
EXEC app
```

Builder `ENV NAME = value` is plain text, applies only to later builder steps, and is part of
the chain key as declared. It is exported through each step shell, so `ENV CACHE = $PWD/.cache`
expands `$PWD` in that step's `/work` directory. `EXEC` and `SETUP` accept single- and
double-quoted argv words; for example, `EXEC nginx -g 'daemon off;'` passes `daemon off;` as one
argument. Unterminated quotes are line-numbered errors.

`IMPORT` takes whole package references, is repeatable, and unions each package's `bin`,
`etc`, and `share` trees read-only at `/bin`, `/etc`, and `/share`. Earlier declarations win
collisions. Bare builder commands resolve through `/bin`; IMPORTed package closures and
explicit package references are the only store paths offered to the sandbox. Import
`${pkgs.cacert}` when a FETCH needs public TLS roots. RUN remains networkless.

The fixed skeleton adds exactly one alias: `/usr/bin/env` points to `/bin/env`. This lets
tool-generated `#!/usr/bin/env bash` (or similar) launchers work when an IMPORT supplies
`env`, typically `${pkgs.coreutils}`; it deliberately dangles otherwise. No other `/usr`
content is present.

Every step key hashes its directive and resolved arguments, the ordered imports and offered
closure, the predecessor key, the versioned fixed sandbox skeleton and environment, COPY source
hashes, and any FETCH pin.
It never hashes workspace bytes. The environment is cleared and rebuilt with `PATH=/bin`,
`SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt`, `HOME=/work`, `SOURCE_DATE_EPOCH=1`, `TZ=UTC`,
`LC_ALL=C`, `TMPDIR=/tmp`, and umask 022. The certificate path becomes readable only when an
IMPORT supplies it; composix does not import a CA package implicitly.

Declared COPY inputs are staged fresh on every execution, including deletions. Files written
by build commands persist in the workspace, so ordinary `target/`, `node_modules/`, and
similar incremental state are warm by default. Nothing is excluded from keys by a cache
declaration because workspace bytes never enter keys. `CACHE` was removed; delete old CACHE
lines. The workspace lives under the user cache directory and is disposable: removing it can
cost time but cannot change an artifact.

The lock memo maps a final chain key to each path an artifact-bound COPY consumes, including
that path's content hash and store object. A memo hit materializes only those paths. Adding a
new consumed path forces the builder to run so it can be recorded. `cix build --cold` runs
with an empty workspace and compares every consumed path with the warm result; a mismatch
names the exact COPY and Cixfile line. `--no-cache` remains a deprecated alias for `--cold`.

There are two intentionally different fetch forms:

- Top-level `FETCH payload <command…>` runs in an empty workdir and creates a reusable binder.
- A `FETCH <command…>` inside a builder runs with that builder's incoming workdir and advances
  its chain.

Both are the only network-enabled build steps. Their output hash is trusted on first use and
written to `Cixfile.lock`; a later forced result must match unless that pin is explicitly
updated. Add `EXPECT <sri-hash>` before the command to declare the hash instead: this removes
the first-use trust window, records the declaration, and reports declared versus actual on a
mismatch. `--update-lock` is intentionally rejected for EXPECT fetches; change the declared
hash.

## Artifact kinds

Each `SERVICE` or `APP` produces its own store item and bare v5 manifest. An `ITEM` produces a
pure store tree with no `cix-manifest.json`; it is suitable for `FROM` consumption and tagging,
not for `cix run` or `cix debug`.

`SERVICE` is the full long-running contract. `EXEC` is its main process; `SETUP` is an
idempotent pre-start hook. `PORT` and `LISTENER` grant inbound networking. `STATEDIR`,
`CACHEDIR`, `LOGSDIR`, `CONFIGDIR`, and `RUNDIR` map directly to systemd's managed
`*Directory=` roles. `GRANT jit` drops `MemoryDenyWriteExecute=`; `GRANT egress` declares
outward network access and retains compose's usage-override semantics. The grant vocabulary is
closed to `jit` and `egress`, one grant per line.

`APP` is a one-shot command. `cix run` starts it as `Type=oneshot`, waits, streams its
output, and returns the command's exit status. Apps have no setup hooks, ports, listeners,
health checks or log/config/run role directories.

An anonymous `cix run` holds an indirect Nix GC root for the item's unit lifetime, then its
visible `ExecStopPost=` removes that root when the unit stops. Tags remain the durable naming
and GC-root mechanism; an untagged item becomes collectable again after its run ends (D63).

Artifact destinations name their native runtime paths and are stored item-relatively. Every
SERVICE and APP gets `PATH=bin` unless it explicitly declares
`ENV PATH = …`, which replaces that default entirely. A one-word `EXEC app` or `SETUP app`
therefore resolves at build time to that item's `bin/app`; use it as the preferred spelling
after copying or linking a runtime binary there. `EXEC /bin/app` and package binaries as direct
store references remain valid. Add external runtime tools visibly with lines such as
`LINK ${pkgs.postgresql}/bin/postgres /bin/postgres`, rather than making them ambient PATH
entries. `cix exec` and `cix debug` inherit the same item-bin PATH. `LINK` is also useful for
package-owned assets such as nginx's `mime.types`.

`ITEM` is the content-only block. It accepts only `COPY`, `FILE`, and `LINK`: runtime directives
such as `EXEC`, `ENV`, ports, grants, or role directories cross the D68 seam and are rejected.
Items are build products; `SERVICE` and `APP` declare runnable contracts.

## Building and tagging a family

`SERVICE`, `APP`, and `ITEM` names are the declared member names. They are not bytes in the
generated manifest or tree: the same source can be forked, promoted, or tagged under another
family without a rebuild. `cix build .` prints only a stable JSON member map, even for one
member, and does not tag anything:

```sh
$ cix build .
{"api":"/nix/store/…-cix-item-api","worker":"/nix/store/…-cix-item-worker"}
```

`cix build .#api` builds only `api` and the backward FETCH/BUILDER slice it consumes, then
prints that bare store path. It cannot be combined with `-t`: a tag names the complete family.

`-t` takes a tag only, is repeatable, and applies to every member. A multi-member Cixfile must
supply its operational family with `--namespace`; this name is CLI-only and never enters output
bytes:

```sh
cix build . --namespace my-project -t v3 -t stable
# my-project/api:v3, my-project/worker:v3, …
```

A single-member Cixfile may omit `--namespace`, so `SERVICE my-nginx` plus `-t v1` creates
`my-nginx:v1`. A namespace may be host-qualified, such as
`--namespace cix.example.com/my-project`, but it never has a scheme. `-t name:tag` and
`-t family/tag` are migration errors: member names live in the Cixfile and the family belongs
in `--namespace`. There is no implicit `:latest`; every ref used by `run`, `pull`, or `inspect`
must spell its tag.

## Where this is honestly not a Dockerfile

- `FROM` binds package or source inputs; it never inherits a root filesystem or image layer.
- `RUN` is networkless and closure-limited. Network access is a visible, hash-pinned `FETCH`.
- Builders are named data-flow nodes, not mutable images. Cross-builder transfer is an
  explicit `COPY`.
- Artifacts are sparse roots plus exact Nix store references. There is no `ADD`, URL/tar
  auto-extraction, `.dockerignore`, arbitrary `USER`, or secret mount syntax.
- `ENV` declares an operator-facing contract. `PORT` is an enforced grant, not documentation.
- `SERVICE` and `APP` distinguish daemons from one-shot commands in the manifest, so invalid
  combinations fail while parsing rather than at runtime.

That narrower language is deliberate: the generated manifest is both the runtime description
and the capability policy.
