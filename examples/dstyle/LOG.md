# dstyle track log

- 2026-07-28 22:52 UTC — Started the D25 style-D networking track under the frozen-runtime
  rule. Read the track and current D20–D27 design. Confirmed the host runs systemd 257, sudo is
  non-interactive, and the repository devenv is available. Current cix v2 maps a single declared
  run path to `RuntimeDirectory=cix-run-<service>:<leaf>` with mode `0700`; services with no
  declared ports receive `RestrictAddressFamilies=AF_UNIX` and `PrivateNetwork=yes`. Plan:
  demonstrate each missing compose primitive with raw systemd properties or temporary units,
  keep all changes in this directory, and turn each wall into a concrete design proposal.
- 2026-07-28 22:52 UTC — Added the first executable probe, `postgres-unix`: a cixSpec 2 item
  starts PostgreSQL with an empty `listen_addresses`, a socket under its declared run directory,
  and no ports. Its demo inspects raw unit properties, connects from the host through the backing
  runtime path, and compares root access with uid 1001 and an unrelated DynamicUser service.
  Added a small shared demo helper for exact property assertions and collected-unit cleanup.
- 2026-07-28 22:55 UTC — First PostgreSQL run reached ready state and the journal proved its sole
  listener was `/run/postgresql/.s.PGSQL.5432`, but initial cluster creation took about 13 seconds,
  longer than the demo's 10-second socket deadline. Cleanup stopped the service and removed the
  runtime directory; systemd retained only the empty `cix-run.slice`. Extended only the readiness
  deadline, fixed collected-unit detection to inspect `LoadState`, and explicitly collect the empty
  cix run slice so the done gate can assert no residual units.
- 2026-07-28 22:57 UTC — Second PostgreSQL harness attempt revealed the permission boundary even
  in readiness detection: unprivileged bash reports the socket path as nonexistent because it
  cannot traverse the DynamicUser-owned `0700` directory. A root inspection while the service was
  live showed `/run/postgresql -> cix-run-postgres`, the socket in the backing directory, and the
  runtime directory owned by the allocated dynamic identity. Changed readiness to test the path as
  root; this is test plumbing and preserves the access probe's expected denial for ordinary users.
- 2026-07-28 22:57 UTC — `postgres-unix/demo.sh` passed end to end. Raw properties were
  `PrivateNetwork=yes`, `RestrictAddressFamilies=AF_UNIX`,
  `RuntimeDirectory=cix-run-postgres`, `RuntimeDirectoryMode=0700`, and `DynamicUser=yes`;
  the allocated uid was 61220 in this run. Root connected from the host and returned `SELECT 1`.
  Host uid 1001 and a second DynamicUser unit both failed to connect, including when the latter
  received a read-only bind of the socket directory. Stop removed the socket and runtime directory,
  and the transient service and empty cix slice were collected. Design wall: a socket pathname is
  not a usable grant while the producer exclusively owns its parent runtime directory.
- 2026-07-28 22:58 UTC — Added `nginx-unix`: nginx declares no ports and listens only at
  `/run/nginx/http.sock`. The demo verifies its cix-generated isolation and direct Unix-socket
  HTTP, then asks `systemd-run` to create a transient `.socket` on `127.0.0.1:8080` paired with
  `systemd-socket-proxyd`. The proxy service is constrained to `PrivateNetwork=yes` and
  `RestrictAddressFamilies=AF_UNIX`; its TCP authority is solely the inherited listener. The demo
  asserts the service is absent before first connection, inspects activation properties after the
  request, and owns cleanup of both generated units, the host listener, and the nginx runtime path.
- 2026-07-28 22:59 UTC — `nginx-unix/demo.sh` passed on its first live run. cix emitted
  `PrivateNetwork=yes`, `RestrictAddressFamilies=AF_UNIX`, `DynamicUser=yes`, and a `0700`
  `cix-run-nginx` runtime directory. Root's direct `curl --unix-socket` returned the expected page.
  `systemd-run` then reported separate `cix-dstyle-nginx-publish.socket` and `.service` units; the
  socket showed `Listen=127.0.0.1:8080 (Stream)` while the service was inactive. The first ordinary
  host curl activated proxyd and returned the same page. The active proxy retained
  `PrivateNetwork=yes` and `AF_UNIX` only, proving an inherited TCP listener does not require
  ambient IP-stack authority. Both units, the listener, cix service, slice, and runtime socket were
  absent after cleanup. Design wall: cix/compose cannot yet describe or own this socket/proxy pair.
- 2026-07-28 23:01 UTC — Added the two-service `stack` item. Its cix spec contains a tiny Python
  HTTP backend bound to `/run/backend/backend.sock` and nginx bound to its own Unix socket, with
  nginx configured to proxy through `/run/stack-shared/backend.sock`. The backend starts through
  frozen cix and demonstrates the exclusive-directory wall. The consumer is therefore a raw
  transient service carrying the missing `BindPaths=` projection and a temporary per-edge
  `SupplementaryGroups=` grant; the demo changes the producer directory/socket group and modes to
  match that grant. This deliberately exposes that bind visibility and filesystem authorization
  are two distinct compose mechanisms. Both units retain cix-equivalent hardening and
  `PrivateNetwork=yes`/`AF_UNIX`-only address-family policy.
- 2026-07-28 23:02 UTC — Stack execution found two harness-level issues. The first backend start
  was canceled by a transient systemd transaction, then the identical item/service started
  normally in isolation and on immediate full retry. The raw nginx service then failed because
  its managed cache directory was not aliased to nginx's app path under `/var/cache`; this is
  existing cix v2 behavior that the raw definition must reproduce, not a new style-D wall. Changed
  the raw property to `CacheDirectory=cix-run-stack-nginx:nginx`. The cleanup trap removed both
  services, socket paths, slice, and temporary group after each attempt.
- 2026-07-28 23:03 UTC — The cache alias alone then failed namespace assembly with `File exists`:
  this host already has `/var/cache/nginx`, so the alias cannot replace it in the unmasked view.
  A minimal raw bind probe reproduced the failure independently. Added cix's corresponding
  `TemporaryFileSystem=/var/cache:ro`, which supplies a private collision-free role root before
  systemd creates the managed-directory alias. This keeps the stack's raw unit faithful to current
  cix generation; it is unrelated to the shared socket projection under test. All probe state was
  removed.
- 2026-07-28 23:03 UTC — `stack/demo.sh` passed end to end. The cix backend showed
  `PrivateNetwork=yes`, `AF_UNIX`, `DynamicUser=yes`, and its exclusive `0700` runtime directory;
  uid 1001 could not even see the socket. After applying the operator-side edge group and modes,
  raw nginx showed `BindPaths=/run/cix-run-backend:/run/stack-shared:rbind`,
  `SupplementaryGroups=cix-dstyle-stack-edge`, and the same private-network/AF_UNIX isolation.
  A request through nginx's Unix listener returned `hello from the dstyle backend`, proving both
  hops use filesystem capabilities with no IP stack. Cleanup removed both services, both sockets,
  runtime directories, the cix slice, and the temporary group. Design wall: native shared runtime
  edges must jointly control directory lifetime, namespace projection, and consumer membership;
  `BindPaths=` alone makes the object visible but does not authorize traversal.
- 2026-07-28 23:05 UTC — Added `listenfds`, a no-port cixSpec 2 item whose Python process requires
  exactly one systemd listener (`LISTEN_PID` must match and `LISTEN_FDS` must equal 1) and accepts
  HTTP directly from fd 3. The demo supplies the missing transient `.socket`/`.service` pair via
  `systemd-run`. The socket owns `127.0.0.1:18081`; the service has `PrivateNetwork=yes`,
  `RestrictAddressFamilies=AF_UNIX`, an empty capability bounding set, and the standard cix
  hardening profile. It proves the service is inactive before the first request, inspects
  `Triggers`/`TriggeredBy`/`Sockets`, and makes two real requests before cleaning both units and
  the listener.
- 2026-07-28 23:05 UTC — `listenfds/demo.sh` passed on its first live run. The socket was active at
  `127.0.0.1:18081` with `Triggers=cix-dstyle-listenfds.service`, while the service remained
  inactive. The first curl activated it and received `LISTEN_FDS=1; no socket() authority`; a
  second request reused the process. Raw service state showed `TriggeredBy` the socket,
  `DynamicUser=yes`, `PrivateNetwork=yes`, `AF_UNIX`, and an empty capability bounding set.
  `Sockets` was not populated by systemd-run's same-name implicit activation pair, which is useful
  design evidence: generated compose units should emit explicit dependency/fd-source wiring rather
  than rely on basename convention. Stop removed the service, socket unit, and TCP listener.
  Design wall: neither cixSpec nor `cix run` can request, create, or report an activation socket.

## Design proposals, ranked

- 2026-07-28 23:08 UTC — Ranked the mechanisms by D25/D27 leverage using the live evidence above.

The sketches below use JSON only to make the data model precise; they do not decide compose's
surface language.

### 1. Named Unix-socket edges with compose-owned runtime directories

This unlocks the largest part of D25 and the strongest D27 enforcement: PostgreSQL clients and
the nginx/backend stack both need the same primitive.

```json
{
  "unixEdges": {
    "database": {
      "producer": {"service": "postgres", "path": "/run/postgresql"},
      "consumers": {
        "api": {"path": "/run/postgresql", "access": "connect"}
      }
    }
  }
}
```

The corresponding item-level declaration remains app-semantic: PostgreSQL still declares its
run path, and compose says that this particular instance's path participates in a named edge.
If embedding the association beside a service proves more readable, the equivalent object form
is `{"path": "/run/postgresql", "shared": "database"}`; `shared` is a compose overlay, not a raw
systemd escape hatch.

Mechanism:

1. Generate a `cix-<composite>-edge-<edge>.service`, held active by the composite target, with
   `RuntimeDirectory=cix-<composite>-edge-<edge>`, a root-owned per-edge group, mode `2770`, and
   stop-time cleanup. This gives the directory composite/edge lifetime instead of producer-process
   lifetime.
2. Project that backing directory to each declared app path with `BindPaths=` for the producer and
   `BindReadOnlyPaths=` where a consumer only connects. Add `Requires=`/`After=` on the edge owner.
3. Generate a collision-resistant system group for the edge (for example
   `cix-e-<stable-hash>`) and put only producer and declared consumers in it with
   `SupplementaryGroups=`. Set `UMask=0007`; applications that expose an explicit socket mode
   should use `0660`.
4. Report the effective grant in `cix ps`/inspection as `unix-edge(group)`, including every member
   and projected path.

Choose the per-edge group. A shared directory object without membership merely repeats the stack's
raw `BindPaths=` failure: visibility is not authorization. `JoinsNamespaceOf=` is the wrong tool:
systemd does not use it to share mount namespaces, it does not align DynamicUser identities, and it
would widen a namespace boundary instead of granting one object. A per-edge group maps directly to
Unix traversal/connect checks, can include multiple consumers, and is observable with ordinary
systemd properties. The generated group is host-persistent metadata, but authority is not: only
running units explicitly carrying `SupplementaryGroups=` can exercise it.

### 2. A first-class activated-listener contract, distinct from `ports`

This is the pure-capability endpoint: `listenfds` showed that a service can accept an inherited
TCP listener while retaining `PrivateNetwork=yes`, `RestrictAddressFamilies=AF_UNIX`, and no
capabilities.

```json
{
  "listeners": {
    "http": {
      "type": "stream",
      "family": "tcp",
      "activation": "fd"
    }
  }
}
```

Do **not** add `"activation": "socket"` to today's `ports` entry. A port currently means “this
process may create an IP socket,” which intentionally grants AF_INET/AF_INET6. An activated
listener means the opposite: “this process accepts an already-open fd and must not gain IP socket
creation.” A separate `listeners` field keeps capability compilation unambiguous. For named
multi-fd services, the application consumes `LISTEN_FDNAMES`; the listener name becomes
`FileDescriptorName=`.

`cix run` supplies the operator binding for an item listener, for example
`cix run item -p http=127.0.0.1:8080`; compose stores the same decision:

```json
{
  "services": {
    "api": {
      "bind": {"http": "127.0.0.1:8080"}
    }
  }
}
```

Mechanism: create a transient `cix-run-…-http.socket` with `ListenStream=` and
`FileDescriptorName=http`, point `Service=` at the service, and put explicit
`Requires=`/`After=` plus `Sockets=` on the service. Explicit wiring is preferable to
systemd-run's same-basename convention: the live probe populated `TriggeredBy` but not `Sockets`,
and systemd permits a conventionally paired service to start without its socket. Put all generated
units behind one run/composite target so stop and failed-start cleanup are atomic. Extend `cix ps`
to show inactive/listening sockets, their bound addresses, target service, and activation count.

### 3. Publish a Unix edge through a socket-activated proxy

This unlocks D25 for daemons such as nginx that can bind Unix sockets but cannot consume
`LISTEN_FDS`.

```json
{
  "publish": {
    "web": {
      "listen": "127.0.0.1:8080",
      "to": {"service": "nginx", "unix": "/run/nginx/http.sock"},
      "mode": "socket-proxy"
    }
  }
}
```

`listen` is an operator/compose decision; the item continues to declare no ports. `to` must resolve
to a declared run path or named Unix edge, never an arbitrary host path.

Mechanism: generate `cix-<composite>-publish-web.socket` with
`ListenStream=127.0.0.1:8080` and a paired `systemd-socket-proxyd` service targeting the Unix
socket. The proxy gets `PrivateNetwork=yes`, `RestrictAddressFamilies=AF_UNIX`, no capabilities,
and consumer membership/projection from proposal 1. It requires the edge and producer service;
the composite target owns both units. This is exactly the nginx proof: the `.socket` alone owns the
host IP listener, proxyd starts on demand, and the application never receives IP-stack authority.
Expose proxy mode honestly because proxyd does not forward `SO_PEERCRED`/SCM side-channel data; a
native fd-activated service should use proposal 2 when end-to-end fd capability semantics matter.

- 2026-07-28 23:10 UTC — The first post-gate residue audit found no system units, socket units,
  TCP listeners, managed runtime directories, socket files, or temporary edge group after two
  complete green passes. It did find that systemd had created the raw `BindPaths=` destination
  `/run/stack-shared` in the host mount namespace and left that empty directory after unit
  collection. Updated stack cleanup to own and `rmdir` that exact empty mountpoint on normal and
  trapped exits, and added it to the assertions. Design amendment to proposal 1: generated compose
  must own destination mountpoint lifecycle too (prefer paths beneath an edge-owned managed
  directory; otherwise explicitly remove empty destinations). The audit also found an empty
  user-manager `cix-run.slice` left by earlier repository work; these demos never use the user
  manager, but the final track gate will stop it and verify both managers are empty as required.
- 2026-07-28 23:11 UTC — Final done gate passed. All four demos completed successfully twice in
  sequence; after the bind-destination cleanup change, the stack completed successfully twice
  again. Final assertions found no `cix-*` loaded units or sockets in either the system or user
  manager, no listeners on 8080 or 18081, none of the managed/runtime/alias/bind paths, and no
  temporary edge group. `bash -n` passed for the shared helper and every demo, every `default.nix`
  parsed with `nix-instantiate`, and `git diff --check` passed. The branch diff from the track-spec
  commit contains only `examples/dstyle/`; no frozen runtime crate was modified.
