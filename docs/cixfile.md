# The Cixfile

*Status: D47 blocks and binders through D65, CIP-79 health, CIP-87 read-set keying, CIP-91
artifact import, and CIP-92 protocol-aware ports are implemented: named persistent builders,
constructive step traces, narrow consumed-path records, universal package IMPORT unions,
declared or TOFU-pinned network fetches, directive continuations, RUN heredocs, and full-line
comments.*

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
IMPORT ${pkgs.nginx}
COPY ${src}/index.html /srv/www/index.html
COPY ${src}/nginx.conf /etc/nginx/nginx.conf
COPY ${pkgs.nginx}/conf/mime.types /etc/nginx/mime.types
START nginx -c /etc/nginx/nginx.conf -e stderr
PORT http = 8080
CACHEDIR /var/cache/nginx
RUNDIR /run/nginx
```

`FROM . AS src` gives the Cixfile directory a name. Bare relative sources remain legal:
`COPY index.html /srv/www/index.html` means exactly the same thing as
`COPY ${src}/index.html /srv/www/index.html`. Explicit binders become useful when a file has
more than one possible origin.

<a id="blocks-and-directives"></a>

## Structure

Prelude declarations come first:

| declaration | meaning |
| --- | --- |
| `FROM <flakeref> [OVERLAY <./file.nix>…] AS <name>` | bind a locked package universe or source tree |
| `FROM <index-ref:tag> AS <name>` | bind a lock-pinned cix item as a source tree |
| `FROM . AS <name>` | optionally name the Cixfile directory; it is local input and is not lock-pinned |
| `FETCH <name> <command…> [EXPECT <sri-hash>]` | run a networked command in an empty workdir and bind its pinned output |

Blocks then declare work and outputs:

| block | allowed directives | result |
| --- | --- | --- |
| `BUILDER <name>` | `IMPORT`, `COPY`, `FETCH`, `RUN`, `ENV` | a persistent workspace whose consumed outputs are recorded individually |
| `SERVICE <name>` | `IMPORT`, `COPY`, `FILE`, `START`, `START_PRE`, `ENV`, `SECRET`, `PORT`, `LISTENER`, `STATEDIR`, `CACHEDIR`, `LOGDIR`, `CONFIGDIR`, `RUNDIR`, `DIR`, `CLAIM`, `READINESS`, `LIVENESS`, `STOPSIGNAL` | a long-running service artifact |
| `APP <name>` | `IMPORT`, `COPY`, `FILE`, `START`, `ENV`, `SECRET`, `CLAIM`, `STATEDIR`, `CACHEDIR`, `READINESS`, `LIVENESS`, `STOPSIGNAL` | a run-to-completion app artifact |
| `ITEM <name>` | `IMPORT`, `COPY`, `FILE` | a pure store tree, with no manifest |

Names share one namespace and references point backward. A builder cannot copy from itself,
and a declaration cannot refer to a later declaration. Errors report both the bad line and,
where useful, the first declaration line.

**A BUILDER exists only when there is RUN or FETCH work to do.** Pure assembly belongs
directly in the `SERVICE`, `APP`, or `ITEM` that consumes the sources; routing local files
through a COPY-only builder adds a name without adding a boundary.

<a id="build-args"></a>

### Closed build arguments

`ARG NAME from value1 value2 …` declares a closed build matrix. The first value is the default;
select another cell with `cix build --arg NAME=value`, or build the Cartesian product of all
declared arguments with `cix build --all-args`. Each selected cell has distinct output receipts
and resolved-statement lock identities, and runnable artifacts record the selection in the
manifest's `buildArgs` object. An undeclared name or value is an error that prints the declared
matrix. `--all-args` cannot be combined with `--arg`.

`LET NAME = value…` is author-local substitution, not a build dimension. A scalar LET can appear
inside a template. A multi-value LET expands to separate arguments only when `${NAME}` occupies a
whole argv position.

<a id="copy"></a>

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

Copy is store-aware by a rule visible in the Cixfile: local and remote source-context paths are
materialized, while package, builder/FETCH, and cix-item sources link as immutable closure
members. A role-directory or `DIR` mount at or below a linked destination, or a later assembly
write below it, materializes that destination tree so mount ancestors and ordered writes remain
real directories.

Copy the whole source tree by default. FETCH and RUN memo entries record the paths they actually
read, so a source-only edit does not invalidate a dependency-fetch step that read only manifests.
Manifest-first enumeration remains an optimization for pathological read sets or especially
large staging trees; structural globs such as `**/Cargo.toml` are not implemented.

There is no magic `${build}` namespace and `TAKE` is gone. A migrated file should name its
builder, normally `BUILDER build`, and use `COPY ${build}/path /destination`. The parser emits
that migration directly instead of turning either spelling into a mysterious unknown name.

`FILE <destination> <<EOF` adds an inline interpolated file. Keep ordinary configuration and
scripts as real files next to the Cixfile and copy them with `COPY`; use a FILE heredoc only
when the file content itself must interpolate `${…}` — and treat wanting that as a smell:
after IMPORT/COPY assembly, configuration can reference stable runtime paths instead of store
paths, so the file stays a real checked-in file. (An interpolating `FILE … FROM` form was
[considered and rejected](../cips/rejected/file-from.md) as unneeded.) Invoke a copied script
through an imported shell:

```dockerfile
IMPORT ${pkgs.bash}
COPY start /bin/start
START sh /bin/start
```

`SCRIPT` was removed; the parser reports the `COPY` plus explicit-shell migration instead of
accepting an alias.

<a id="link"></a>`LINK` was removed. Use `COPY <source> <destination>`; the parser rejects the old directive and points here.

<a id="syntax"></a>

### Lines, comments, and heredocs

A backslash continues any directive onto the next physical line:

```dockerfile
IMPORT ${pkgs.bash} ${pkgs.cargo} \
    ${pkgs.rustc} ${pkgs.gcc}
```

Errors retain physical Cixfile line numbers. A line whose first non-whitespace character is
`#` is a comment; end-of-line comments are deliberately not special because RUN and FETCH
arguments are shell text.

New RUN and FETCH lines are argv-first: each whitespace-delimited value is passed directly to the
declared program, with no implicit shell. Shell punctuation such as `;`, `>`, and `&&` is therefore
literal argv data. Invoke a shell explicitly when shell syntax is intended. The pre-epoch
unbraced grammar remains accepted during migration.

A heredoc always declares its interpreter. Cix writes the body to a file outside `/work` and
invokes the interpreter with that filename:

```dockerfile
RUN bash <<BUILD
mkdir -p output
cargo build --release --locked --offline
cp target/release/app output/app
BUILD
```

The interpreter and complete body are part of the node identity. Shell comments inside the body
belong to that interpreter. `${…}` remains build-time interpolation; use `$${…}` when the shell
itself must receive a braced expansion.

Indented `WITH NAME=value` edges add an environment value to that node only. `WITH NAME` bridges
a preceding scalar `LET NAME = value`; the value never leaks to another node.

<a id="unsafe-ignore"></a>

`WITH UNSAFE IGNORE path` is an explicit evidence waiver for a workdir-relative subtree. Cix
excludes it from trace reads, changes, memo replay, volatility, pins, and final consumed-output
seals, while leaving the warm workspace subtree available to the command. Every use prints the
waived path. Cix may suggest a cache-shaped subtree observed as both read and written, but never
adds or classifies an ignore automatically. Paths must remain within `/work`; ignored outputs
cannot be consumed by a later artifact.

<a id="formatting"></a>

### Runtime credentials

`SECRET <name> [AS <VAR_FILE>]` declares a runtime credential need on a SERVICE or APP.
It names no value and is delivered only when compose supplies that name. The process reads
`$CREDENTIALS_DIRECTORY/<name>`; `AS` sets the given `_FILE` variable to that same path, for
images that already support the conventional `PASSWORD_FILE` shape. Raw secret environment
variables are deliberately refused.

FETCH credentials are host-local too: `~/.config/cix/credentials` (or
`$CREDENTIALS_DIRECTORY/credentials` for a cix unit) maps a token name to a narrow URL pattern
and credential file. On a matching concrete FETCH URL cix asks for per-project, per-token,
per-prefix consent; `cix build --allow-secret` is the non-interactive CI form and
`cix credentials revoke <token>` removes remembered consent. Neither Cixfiles nor locks name
tokens, and credential files never enter the store.

### Compose credentials

The compose document owns values, while item manifests own credential needs. Supply each
declared name exactly once at top level, using either an absolute plaintext file or an absolute
systemd-encrypted credential file:

```json
{
  "secrets": { "db-password": { "file": "/etc/cix/db-password" } },
  "children": { "database": { "item": "example/db:v1" } }
}
```

Only services that declare `SECRET db-password` receive `LoadCredential=db-password:…`; an
`encrypted` source uses `LoadCredentialEncrypted=`. `cix compose check` rejects a missing
declared source and warns loudly about a supplied source that no item consumes. On `cix up`, a
salted HMAC fingerprint detects a changed source and restarts only its consumers. Use
`cix run --compose FILE` (or `-` for stdin) for the same complete compose path; direct run
options cannot inject credentials.

## Formatting

`cix fmt [PATH…]` formats Cixfiles in place. With no path it searches the current directory
recursively for files named `Cixfile`, respecting `.gitignore`; an explicit file is formatted even
when it has another name. The formatter preserves comments, heredoc bodies, and author-chosen
line breaks while normalizing the surrounding Cixfile syntax.

Use `cix fmt --check [PATH…]` in automation to make no writes and print a unified diff for every
file that needs formatting. `cix fmt -` reads one Cixfile from standard input and writes its
formatted form to standard output.

<a id="inputs"></a>

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

For a project-local package customization, attach one or more ordered overlays to a package
universe:

```dockerfile
FROM github:NixOS/nixpkgs/nixos-unstable OVERLAY ./php.nix AS pkgs
```

Each overlay is a checked-in `final: prev: { ... }` Nix function. cix imports the locked base
with nixpkgs' native `overlays` argument, so every `${pkgs.*}` reference observes the same
fixpoint. The base must accept that argument; an error suggests wrapping it or using a full
universe tree. Overlay files cannot reference Cixfile binders. Their ordered content hashes join
the base pin in builder keys and vendored development-environment snapshots; editing an overlay
is therefore an ordinary source edit, while `--update-lock pkgs` moves only the base pin.

Multiple universes remain legal, but putting packages from differently overlaid worlds into one
item deliberately reopens world skew. Keep one project world where practical. `OVERLAY` is the
local convenience form; an organisation-owned full universe tree remains the general D65 form,
and composed items remain the distribution form.

The third FROM input is an explicit-tag index ref:

```dockerfile
FROM cix.my-org.com/acme/web-vault:v3 AS webvault

SERVICE web
COPY ${webvault}/share/web-vault /share/web-vault
START /bin/true
```

It resolves through the cix index (qualified refs fetch when needed), verifies its NAR hash,
and records `artifacts.<ref> = { storePath, narHash }` in `Cixfile.lock`. The tag may move;
the lock keeps this build on the selected store path until `cix build --update-lock webvault`.
A missing local ref says to pull or tag it first. Item binders are trees: use only
`${webvault}/path` in `COPY`. `${webvault.attr}` is rejected because index refs never
create package namespaces, and `IMPORT ${webvault}` is deliberately unavailable.

Disambiguation is deliberately mechanical: a known flakeref spelling is a flakeref; every
other FROM token must be a valid index ref with an explicit `:tag`. There is no default tag.
An untagged `family/web-vault` therefore gives the same `:latest is not a thing here` error as
the rest of the index surface.

The lock records remote revisions/NAR hashes and cix-item store paths/NAR hashes. A locked FROM
binder also exposes `${name.rev}`, `${name.shortRev}`, `${name.revCount}`, `${name.narHash}`,
`${name.lastModified}`, and `${name.lastModifiedDate}` where the pin supplies them. These are
resolved from `Cixfile.lock` before execution, so they enter ordinary step keys. Local tree
binders may expose dirty facts when derivable; a missing fact is a line-numbered error that lists
the attributes available on that binding. `FROM .` and
the implicit local context are not pinned: the content of each declared COPY source enters its
builder's chain key. `$VAR` remains runtime environment syntax and is valid in `START` and
`START_PRE`; `${…}` is resolved while building. Copied file content is always verbatim.

<a id="builders"></a>

## Builders, `IMPORT`, `RUN`, and `FETCH`

Each `BUILDER` has a persistent workspace. Later blocks consume named paths from the tree it
leaves behind:

```dockerfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

BUILDER compile
IMPORT ${pkgs.bash} ${pkgs.cargo} ${pkgs.rustc} ${pkgs.gcc} ${pkgs.cacert}
COPY ${src}/ .
FETCH cargo fetch --locked
RUN <<BUILD
cargo build --release --locked --offline
BUILD

APP app
COPY ${compile}/target/release/app /bin/app
START app
```

Builder `ENV NAME=value` is plain text, applies only to later builder steps, and is part of
the chain key as declared. It is exported through each step shell, so `ENV CACHE=$PWD/.cache`
expands `$PWD` in that step's `/work` directory. `START` and `START_PRE` accept single- and
double-quoted argv words; for example, `START nginx -g 'daemon off;'` passes `daemon off;` as one
argument. Unterminated quotes are line-numbered errors.

Runtime `ENV NAME=value` supplies a default, `ENV NAME required` requires an operator value, and
bare `ENV NAME` declares an optional value that remains unset unless supplied. Quotes keep spaces
inside a value (`ENV GREETING="hello world"`). There are no spaces around `=`; `ENV NAME = value`
is a parse error. A required declaration cannot also carry a default.

`IMPORT` takes whole package references, is repeatable in every block kind, and unions each
package's `bin`, `etc`, and `share` trees read-only at `/bin`, `/etc`, and `/share`. Earlier
declarations win collisions. Bare commands resolve through `/bin`; IMPORTed package closures
and explicit package references are the only store paths offered to the sandbox. Import
`${pkgs.cacert}` when a FETCH needs public TLS roots. RUN remains networkless.

On the first build for an ordered IMPORT set, cix asks the pinned nixpkgs for its development
environment and snapshots the deterministic store-path variables in `Cixfile.lock`. The fixed
sandbox values (`PATH`, `HOME`, `TMPDIR`, locale, timestamp, and timezone) still win; stdenv
control variables are dropped. Later builds reuse the snapshot without invoking Nix. This means
native package conventions such as `PKG_CONFIG_PATH` are supplied automatically: import
`${pkgs.pkg-config}` and the relevant library packages (including their `.dev` outputs), then run
the build command directly rather than spelling a manual pkg-config path.

The fixed skeleton includes `/usr/bin/env -> /bin/env`. This lets tool-generated
`#!/usr/bin/env bash` (or similar) launchers work when an IMPORT supplies `env`, typically
`${pkgs.coreutils}`; it deliberately dangles otherwise. No other `/usr` content is present.

<a id="fetch-tls-trust"></a>

### FETCH TLS trust

Network authority does not imply ambient trust material. A public-TLS FETCH normally imports
`${pkgs.cacert}` explicitly. Without it, clients that retry certificate failures can look like a
network hang; a timeout accompanied by repeated failed OpenSSL hashed-certificate probes is a
TLS-trust masquerade, not evidence that the package registry or IPv6 is broken.

<a id="pnpm-frozen-store"></a>

### Read-only pnpm stores

pnpm 11.7 and newer can consume a complete store read-only when Node is at least 22.15. Fetch the
whole store, including `files/` and `index.db`, in a networked builder; then point a separate
offline builder at that immutable binder with `frozen-store=true`, `--offline`, and
`--frozen-lockfile`. Do not extract the CAS, rewrite SQLite, or regenerate metadata:

```dockerfile
BUILDER pnpm-store
  IMPORT ${pkgs.pnpm} ${pkgs.nodejs_24} ${pkgs.bash} ${pkgs.cacert}
  COPY ${src}/pnpm-lock.yaml .
  FETCH pnpm fetch --frozen-lockfile --ignore-scripts --store-dir pnpm-store

BUILDER build
  IMPORT ${pkgs.pnpm} ${pkgs.nodejs_24} ${pkgs.bash}
  COPY ${src}/ .
  RUN <<INSTALL
pnpm config set frozen-store true --location project
pnpm config set store-dir ${pnpm-store}/pnpm-store --location project
pnpm install --offline --frozen-lockfile --frozen-store --ignore-scripts
INSTALL
```

The store is a TOFU instance-pin: independent fetches may produce different `index.db` bytes,
but one pinned instance must remain unchanged while every offline consumer uses it. Package
manager versions are part of the translation contract; if an upstream `packageManager` pin is
older than 11.7, an override must be deliberate and recorded with the migration's gaps.

### FHS loader aliases

On x86-64 the builder skeleton also includes fixed GNU
`/lib64/ld-linux-x86-64.so.2` and musl `/lib/ld-musl-x86_64.so.1` aliases. They dangle until
the ordered IMPORT set supplies the corresponding loader, normally through `${pkgs.glibc}` or
`${pkgs.musl}`. This lets an unmodified downloaded ELF enter its imported libc loader; changing
the versioned skeleton invalidates affected builder memo keys.

The aliases do not add packages' `lib` trees to `/lib` and do not change the loader's search
path. GNU compatibility therefore covers only `DT_NEEDED` libraries supplied by the imported
glibc. On a failed RUN, cix diagnoses a missing loader with the libc IMPORT to add. If the binary
needs libraries beyond that libc, it instead names the missing SONAMEs and points to the explicit
patchelf escape in [the migration guide](migrate.md#fhs-linked-native-binaries).

FETCH and RUN use constructive traces. Their static memo identity hashes the directive text,
resolved arguments, declared environment, ordered imports and offered closure, and the versioned
fixed sandbox skeleton—not the predecessor key or workspace bytes. The dynamic part records each
workspace path the command read: file content, file or directory existence for metadata-only
probes, directory entry lists, and absent-path probes. Lookup rehashes exactly that recorded read
set, so unrelated workspace changes are early-cut off. A miss executes the command under `strace`, records a fresh
single latest trace, and stores its filesystem delta for exact replay. COPY source hashes and the
old predecessor chain remains only the persistent-workspace lineage and final-receipt address.

The environment is cleared and rebuilt with `PATH=/bin`,
`SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt`, `HOME=/work`, `SOURCE_DATE_EPOCH=1`, `TZ=UTC`,
`LC_ALL=C`, `TMPDIR=/tmp`, and umask 022. The certificate path becomes readable only when an
IMPORT supplies it; composix does not import a CA package implicitly.

Declared COPY inputs are staged fresh on every execution, including deletions. A non-cold
re-run starts from that builder's own last end-state, then validates each affected command's
recorded reads; hits replay only that command's writes and misses execute. Files written by build
commands therefore persist in the workspace, so
ordinary `target/`, `node_modules/`, and similar incremental state are warm by default. Nothing
is excluded from keys by a cache declaration because workspace bytes never enter keys. `CACHE`
was removed; delete old CACHE lines. Workspaces are scoped to one project and builder name, with
no cross-builder or cross-project reuse.

Warm results are path-dependent: `build(A→B)` can differ from `build(B)` when deleted sources or
dependencies survive as ghost files in the underlay. Removing a workspace with `rm -rf` is always
safe: it drops that underlay and makes the next run clean. `cix build --cold` is the clean truth;
it starts with an empty workspace, audits each recorded read set against the clean replay, and
audits the consumed outputs against the warm result. A divergence names the directive and line.

The lock memo maps a final chain key to each path an artifact-bound COPY consumes, including
that path's content hash and store object. A memo hit materializes only those paths. Adding a
new consumed path forces the builder to run so it can be recorded. `cix build --cold` compares
every consumed path with the warm result; a
mismatch names the exact COPY and Cixfile line. It replays already pinned FETCH deltas and never
contacts the network: cold proves builder reproducibility, while trust in fetched bytes is the
FETCH pin.

There are two intentionally different fetch forms:

- Top-level `FETCH payload <command…>` runs in an empty workdir and creates a reusable binder.
- A `FETCH <command…>` inside a builder runs with that builder's incoming workdir and advances
  its chain.

Both are the only network-enabled build steps. An automatic lock pin records only a map of the
paths downstream consumers actually use; incidental cache files outside that set do not make a
build flap. A local Nix-store replay cache, keyed by that stable pin rather than serialized into
the lock, lets `--cold` replay FETCH outputs without
making volatile cache bytes part of `Cixfile.lock`. First use of an additional top-level consumed
path is reported and recorded as a fresh pin entry. `cix build --update-lock <fetch-or-builder>` deliberately fetches twice, reports
differing file names and sizes, and records those volatile-file facts in the lock; it never
silently removes them. Add trailing `EXPECT <sri-hash>` after the command to keep a whole-workdir author
integrity assertion instead: this removes the first-use trust window and reports declared versus
actual on a mismatch. `--update-lock` is intentionally rejected for EXPECT fetches; change the
declared hash. The former leading form is rejected with this migration.

When a FETCH leaves at least 16 MiB outside the downstream-consumed paths, cix prints an
informational size note. It never changes the build result or turns into a failure; it is a prompt
to keep a large workspace intentional.

### Build statistics

`cix build --stats` returns the normal item result together with a stable JSON `stats` object.
Each entry has a `name`, `kind`, and `status` (`executed` or `memo-hit`), while
`nixSubprocesses` totals Nix child processes for that invocation. A full memo hit reports every
step as `memo-hit` and `nixSubprocesses: 0`; this is the assertion channel for the no-op floor.

<a id="artifact-kinds"></a>

## Artifact kinds

Each `SERVICE` or `APP` produces its own store item and bare v0 manifest. An `ITEM` produces a
pure store tree with no `cix-manifest.json`; it is suitable for `FROM` consumption and tagging,
not for `cix run` or `cix debug`.

`SERVICE` is the full long-running contract. `START` is its main process; `START_PRE` is an
idempotent pre-start hook. `PORT` and `LISTENER` declare inbound networking. `STATEDIR`,
`CACHEDIR`, `LOGDIR`, `CONFIGDIR`, and `RUNDIR` map directly to systemd's managed
`*Directory=` roles.

`PORT <name> = <port>` declares TCP, while `PORT <name> = udp:<port>` declares UDP:

```dockerfile
PORT https = 443
PORT http3 = udp:443
```

Both are enforced through the corresponding `SocketBindAllow=` and deny rules. The Docker-form
`443/udp` is refused with a hint to write `udp:443`; the systemd spelling is the one Cixfile
form. `LISTENER` remains a TCP stream contract, and compose publication remains TCP-only until
a datagram-activation case establishes its required behavior.

<a id="role-dirs"></a>

Role dirs are claims cix fulfills itself. `STATEDIR`, `CACHEDIR`, `LOGDIR`, and `RUNDIR` each
accept any clean absolute path the application uses, including paths such as `LOGDIR /app/logs`
or `STATEDIR /var/www/db` beneath an artifact projection.
They are private to the service. cix mirrors the full in-namespace path beneath its unit-scoped
systemd backing root, binds that backing path into the service, and sets the corresponding
`$STATE_DIRECTORY`, `$CACHE_DIRECTORY`, `$LOGS_DIRECTORY`, or `$RUNTIME_DIRECTORY` value to the
declared in-namespace path. Declare a role according to its lifecycle, not its spelling.

`DIR /path[:ro|:rw]` is different: it claims operator-supplied data. A bare `DIR /path` is rw;
`DIR /media:ro` is read-only. cix never creates, owns, or deletes it. `DIR` needs compose
materialization, so `cix run` currently stops with a teaching error rather than inventing an
empty private directory.

| declaration | supplied by | stop/crash | future purge |
| --- | --- | --- | --- |
| `RUNDIR` | cix/systemd | removed | removed |
| `CACHEDIR` | cix/systemd | kept | removable |
| `LOGDIR` | cix/systemd | kept | removable with opt-in |
| `STATEDIR` | cix/systemd | kept | kept unless explicit purge |
| `DIR` | operator/compose | untouched | never |

<a id="claims"></a>

`CLAIM jit` drops `MemoryDenyWriteExecute=`; `CLAIM egress` declares
outward network access and retains compose's usage-override semantics. `CLAIM gpu` replaces the
ordinary `PrivateDevices=` sandbox with `DevicePolicy=closed`, allows the `/dev/dri` class, and
adds `video` and `render`. `CLAIM device /dev/<node>` adds exactly that node and resolves its
owning group while generating the unit. If the node is absent at generation time cix warns and
leaves activation to fail when the hardware is still absent. Device claims never broaden into
`--privileged` access. The claim vocabulary is closed to `jit`, `egress`, `gpu`, and
`device /dev/<node>`, one claim per line.

`SHM <size>` mounts a private `/dev/shm` with systemd size syntax, for example `SHM 64M` or
`SHM 1G`. In compose, a service `shm: "<size>"` is an operator override and wins over the item;
`cix compose diff` reports an effective SHM change. `grants:` is reserved for the future explicit
compose-side loosening field; it is deliberately not accepted yet, so compose cannot silently
widen device access.

`STOPSIGNAL <signal>` selects the process signal systemd sends while stopping the item, using a
known signal name such as `SIGQUIT`; it compiles directly to `KillSignal=`. The compose-side
`stopTimeout: "30s"` sets `TimeoutStopSec=` for that member.

<a id="closed-root"></a>

### Closed-root audit mode

`cix run --closed-root` and `cix up --closed-root` put every artifact process in a sealed
filesystem root. This is CIP-84's phase-1 audit gate; the flag is deliberately off by default
until the complete example/corpus tier is green, after which the sealed root becomes the only
runtime rather than a permanent compatibility dial.

The root starts empty. cix mounts the API filesystems, the whole Nix store read-only, the item's
declared runtime projections, role directories, and explicit compose/claim materializations.
Nothing else on the host is visible. `ProtectSystem=strict` remains defense in depth, but the
important boundary is visibility: an undeclared `/etc`, `/opt`, host data path, or executable
does not exist in the service's world.

Four host edges have narrow channels:

- cix generates `/etc/passwd` and `/etc/group` containing exactly root, the unit identity, and
  nobody, and combines that database with `PrivateUsers=`. This covers DynamicUser and declared
  static identities without importing the host account database.
- `CLAIM egress` brings in the host's `/etc/resolv.conf` verbatim. CA trust does not: `COPY`
  `${pkgs.cacert}` (or another declared trust bundle) into the artifact and point the application
  at it.
- timezone selection is an ordinary `TZ` environment value. `/etc/localtime` is never injected.
- systemd mounts notify and journald sockets into the root. Native readiness/liveness adapters
  therefore keep working without adding a shell or logging sidecar.

The sole conventional executable alias is `/usr/bin/env -> /bin/env`; it dangles unless the
artifact imports a package that provides `env`, usually `IMPORT ${pkgs.coreutils}`. cix reports
that dependency when a declared executable uses an env shebang. `/bin/sh` is never created.
Import a shell and invoke it by bare name, for example `IMPORT ${pkgs.bash}` followed by
`START sh /bin/start`; shell availability is part of the artifact contract, not NixOS host luck.

Closed-root services cannot bind ports below 1024 directly. `PrivateUsers=` puts their
capabilities in a user namespace, where `CAP_NET_BIND_SERVICE` cannot authorize a bind in the
host network namespace. Use an unprivileged port or declare a named `LISTENER` so systemd owns
the privileged socket; cix rejects the ineffective direct-capability combination at compile time.

There is no raw-host opt-out. Host data, shared surfaces, devices, and network egress enter only
through their declared directory/materialization/claim channels. `--user --closed-root` attempts
the same filesystem world for dev/prod parity and uses the existing loud D13 degradation when a
user manager or kernel cannot realize mount namespaces.

<a id="health"></a>

### Readiness and liveness

`READINESS` gates successful startup; `LIVENESS` opts the unit into watchdog restart. Both are
valid in `SERVICE` and `APP` blocks and accept only `http`, `tcp`, or `notify` probes:

```dockerfile
READINESS http://127.0.0.1:8080/healthz IN 90s
READINESS tcp://db.internal:5432 IN 60s
READINESS notify IN 90s

LIVENESS http://127.0.0.1:8080/livez EVERY 10s
LIVENESS tcp://db.internal:5432 EVERY 10s
LIVENESS notify EVERY 10s
```

HTTP and TCP targets are absolute URLs: the scheme is the probe kind. HTTP uses standard URL port
defaults, so `http://127.0.0.1/healthz` probes port 80 and may include a query; TCP requires an
explicit port and no path.
When the service declares exactly one `PORT`, a path-only target such as
`READINESS /healthz IN 90s` resolves to that port's
`http://127.0.0.1:<port>/healthz` URL. With zero or multiple ports, cix lists the explicit URLs to
choose from. The old `http <host:port/path>` and `tcp <host:port>` spellings are rejected with a
URL rewrite. Durations are positive integers followed by `ms`, `s`, `m`/`min`, `h`, or `d`.
`notify` has no target: the process sends native systemd readiness/watchdog messages.

For adapter probes, cix emits its own native HTTP/TCP prober—no `curl` or shell enters the item
closure. Readiness blocks the systemd start job until the first success and `IN` is its
`TimeoutStartSec=` budget, so `cix up` waits and fails when readiness times out. Liveness feeds
systemd's watchdog on success; the watchdog window is fixed at three times `EVERY`, and declaring
it emits a bounded `Restart=on-failure` policy. Structural compose edges wait for the producer's
start job and therefore its readiness. A separate `condition: service_healthy` graph is rejected.

The generated manifest fields are typed objects such as
`"readiness":{"type":"http","target":"http://127.0.0.1:8080/healthz","timeout":"90s"}` and
`"liveness":{"type":"notify","interval":"10s"}`. The former v0
`health {exec, interval}` manifest field is refused with a migration message; there is no `exec`
probe in this schema.

`APP` is a one-shot command. `cix run` starts it as `Type=oneshot`, waits, streams its
output, and returns the command's exit status. Apps have no setup hooks, ports, listeners,
or log/config/run role directories.

An anonymous `cix run` holds an indirect Nix GC root for the item's unit lifetime, then its
visible `ExecStopPost=` removes that root when the unit stops. Tags remain the durable naming
and GC-root mechanism; an untagged item becomes collectable again after its run ends (D63).

<a id="runtime-path"></a>

Artifact destinations name their native runtime paths and are stored item-relatively. Every
SERVICE and APP gets `PATH=bin` unless it explicitly declares
`ENV PATH=…`, which replaces that default entirely. A one-word `START app` or `START_PRE app`
therefore resolves at build time to that item's `bin/app`. Use `IMPORT` for package-provided
runtime tools and bare argv words; `COPY` can assemble an app-specific executable at `/bin/app`.
`START /bin/app` and package binaries as direct store references remain legal, but are not the
taught spelling. `cix exec` and `cix debug` inherit the same item-bin PATH. Use `COPY` for
package-owned assets such as nginx's `mime.types`.

<a id="item"></a>

`ITEM` is the content-only block. It accepts only `IMPORT`, `COPY`, and `FILE`: runtime
directives such as `START`, `ENV`, ports, claims, or role directories are rejected.
Items are build products; `SERVICE` and `APP` declare runnable contracts.

## Building and tagging a family

## Configuration

CLI flags take precedence over their matching `CIX_*` environment variables,
which in turn take precedence over the documented defaults. In particular,
`--workspace-directory`/`CIX_BUILD_WORKSPACE_DIR` selects persistent builder
workspaces and `--state-directory`/`CIX_STATE_DIR` selects local index state.
Configuration is resolved at the CLI boundary before build and runtime code run.

`SERVICE`, `APP`, and `ITEM` names are the declared member names. They are not bytes in the
generated manifest or tree: the same source can be forked, promoted, or tagged under another
family without a rebuild. `cix build .` prints only a stable, indented JSON member map, even
for one member, and does not tag anything. Every cix JSON surface is deterministically indented:
stdout remains pipe-safe for `jq` or another JSON consumer, and generated manifests, locks, and
compose artifacts are readable on disk too:

```sh
$ cix build .
{
  "api": "/nix/store/…-cix-item-api",
  "worker": "/nix/store/…-cix-item-worker"
}
```

`cix build .#api` builds only `api` and the backward FETCH/BUILDER slice it consumes, then
prints that bare store path. It cannot be combined with `-t`: a tag names the complete family.

`cix build --file Cixfile.dissolved .` selects a named Cixfile and writes its independent sibling
lock, `Cixfile.dissolved.lock`. The default `Cixfile` similarly uses `Cixfile.lock`; a translation
twin can therefore live beside its source file without sharing pins.

## Workshop dev loop

Use `cix watch [PATH]` for the artifact loop: it watches a Cixfile context, coalesces short
edit bursts, warm-rebuilds, and prints each resulting item path. It deliberately leaves build
output noisy. The watcher derives its own safety exclusions — `.git`, `target/`, Cixfile locks,
builder workspaces, and `.gitignore`d paths — so its own warm workspace and lock writes never
cause another build.

At a directory containing `compose.json`, `cix watch` rebuilds the edited local Cixfile member,
updates that member's local item tag, and runs the same selective activation path as `cix up`.
Only services whose item changed restart. This is the honest outer loop: the running service is
always a newly built artifact, never a source tree copied into a live process.

Framework hot reload is a different, inner-inner loop. Run Flask debug mode, Vite HMR, and
similar tooling in `nix develop`; they operate on development processes, not deployable cix
artifacts. `cix watch` intentionally does not offer Docker-style file sync.

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
- `ENV` declares an operator-facing contract. `PORT` is an enforced capability declaration, not documentation.
- `SERVICE` and `APP` distinguish daemons from one-shot commands in the manifest, so invalid
  combinations fail while parsing rather than at runtime.

That narrower language is deliberate: the generated manifest is both the runtime description
and the capability policy.
