# Track: specv3 — listeners, kernel-enforced ports, unit-generator API (D29)

Read `docs/design.md` D29 + the dstyle proposals it cites (`examples/dstyle/LOG.md`, sections
"Design proposals, ranked" #2 and the listenfds example — that live evidence is your
behavioral reference). Where ambiguous, choose boring and note it in LOG.

## Ground rules

- Work log: `crates/cix-run/LOG.md`. Territory: `crates/cix-run/`, `examples/`
  (a new `examples/listenfds/` promoted from the dstyle probe; do not modify
  `examples/dstyle/` itself), tour regeneration only via the generator.
- Sudo available; clean up all units AND sockets, always. COMMIT AS YOU GO; clean status.

## Deliverables

1. **Version gating**: `cixSpec: 3` accepted alongside 1/2; `listeners` under v1/v2 errors
   naming the field and required version. v1/v2 semantics unchanged.
2. **`listeners` field** per service: `{"<name>": {"type": "stream"}}` (v3 keeps it minimal:
   TCP stream only; datagram/extra types error "not yet supported"). Semantics: the service
   accepts inherited fds — compile `PrivateNetwork=` logic and `RestrictAddressFamilies=`
   exactly as if NO network were declared (fd-inherit grants nothing; AF_UNIX only), set
   `FileDescriptorName=<name>` expectations via the socket unit, and pass
   `LISTEN_FDNAMES`-compatible wiring. A service may have both `ports` and `listeners`;
   capability compilation treats them independently (ports grant IP sockets, listeners do
   not).
3. **`cix run` binding**: `-p <listener>=<addr:port>` (e.g. `-p http=127.0.0.1:8080`) creates
   a transient `.socket` unit (`ListenStream=`, `FileDescriptorName=`, explicit `Service=`)
   plus explicit `Sockets=`/`Requires=`/`After=` on the service — per the dstyle finding that
   basename-convention pairing is not reliable. Unbound listener at run time = clear error.
   Stop tears down socket + service; `cix ps` shows listening sockets (address, target
   service, active state).
4. **D24**: compile `SocketBindDeny=any` + one `SocketBindAllow=` per effective declared port
   (tcp/udp per its protocol) for every service with `ports`; services with only `listeners`
   get `SocketBindDeny=any` outright. Verify live: a service binding an undeclared port is
   refused by the kernel (EPERM) while its declared port works — make this a real
   root-gated integration test (self-skipping pattern already exists in
   `tests/system_projection.rs`).
5. **Unit-generator library API**: refactor so unit compilation is callable as a library:
   naming scheme (unit/slice/target names, directory-name prefix) and extra properties
   (e.g. future edge `SupplementaryGroups=`/`BindPaths=`) are injectable parameters with the
   current `cix-run-<svc>` behavior as the default. Public, documented (rustdoc), covered by
   a test that compiles a service under a foreign naming scheme (`cix-mycomp-web`). No
   behavior change for existing paths — golden fixtures prove it.
6. **`examples/listenfds/`**: the dstyle probe as a first-class example — small Cixfile (or
   .nix) with `cixSpec: 3`, one `listeners.http`, demo.sh: `cix run … -p http=127.0.0.1:PORT
   --detach`, curl proves serving, `systemctl show` proves `PrivateNetwork`-tier confinement
   + empty capabilities + SocketBindDeny, stop cleans both units.
7. Golden fixtures for all new mappings; validation tests (listener under v2, bad `-p`
   targets, listener+port coexistence).

## Done gate

fmt/clippy/`cargo test --workspace` green ×2; the new demo + both existing sudo demos green;
VM check green; no leftover units/sockets; committed; LOG summary.
