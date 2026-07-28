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
