# dstyle track log

- 2026-07-28 22:52 UTC — Started the D25 style-D networking track under the frozen-runtime
  rule. Read the track and current D20–D27 design. Confirmed the host runs systemd 257, sudo is
  non-interactive, and the repository devenv is available. Current cix v2 maps a single declared
  run path to `RuntimeDirectory=cix-run-<service>:<leaf>` with mode `0700`; services with no
  declared ports receive `RestrictAddressFamilies=AF_UNIX` and `PrivateNetwork=yes`. Plan:
  demonstrate each missing compose primitive with raw systemd properties or temporary units,
  keep all changes in this directory, and turn each wall into a concrete design proposal.
