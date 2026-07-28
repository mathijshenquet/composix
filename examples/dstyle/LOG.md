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
