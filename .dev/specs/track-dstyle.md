# Track: dstyle — Style-D networking examples (unix sockets + socket activation), co-design via TDD

Goal: build working D25-style examples (docs/design.md D25: unix sockets in runtime dirs,
socket activation, `PrivateNetwork` services that never touch an IP stack) and let every wall
you hit produce a CONCRETE design proposal. This track co-designs spec v3 / compose: the LOG
is as important as the demos.

HARD RULE: do NOT modify `crates/` — the runtime is frozen for this track. Where the current
spec/runtime cannot express something, demonstrate the wiring with raw `systemd-run`
properties or hand-written transient units in the demo script, and record in LOG exactly what
cix should grow to make it native (proposed schema/mechanism, one per wall).

## Ground rules

- Everything lives in `examples/dstyle/` (new): subdirs per example + `examples/dstyle/LOG.md`
  (timestamped, the design-input document). Territory: `examples/dstyle/` ONLY.
- Sudo available; clean up ALL units incl. any hand-made socket units, on all paths.
- COMMIT AS YOU GO; clean `git status --short`.

## Deliverables

1. **postgres-unix** (`examples/dstyle/postgres-unix/`, Cixfile or .nix, your call): postgres
   with `listen_addresses=''` — unix socket ONLY, in its run dir. No ports declared ⇒ verify
   the generated unit really gets `PrivateNetwork=yes` (show it in the demo via
   `systemctl show`). Connect via `psql -h /run/cix-run-postgres…` from the host. Then probe
   the permission reality: who can reach the socket (root? uid 1001? another DynamicUser
   unit?) — the 0700 dir and dynamic ownership are the point: document how socket access
   should become a granted capability (candidate mechanisms: shared runtime dir object form
   `{"path": …, "shared": "<edge-name>"}`, per-edge groups via `SupplementaryGroups=`,
   `JoinsNamespaceOf=`; argue for one).
2. **nginx-unix** (`examples/dstyle/nginx-unix/`): nginx listening on
   `unix:/run/nginx/http.sock` only, no ports ⇒ `PrivateNetwork=yes`. Demo:
   `curl --unix-socket`. Then PUBLISH it D-style: a systemd `.socket` unit on host
   `127.0.0.1:8080` + `systemd-socket-proxyd` forwarding to the unix socket (hand-written
   transient/temporary units in the demo — allowed and expected). Record the proposal: how
   should compose declare "publish this unix socket at host :8080 via socket activation"?
3. **stack** (`examples/dstyle/stack/`): nginx-unix `proxy_pass http://unix:…` to a tiny
   backend service (one item, two services: nginx + a minimal HTTP backend on a unix socket —
   a small python/script service is fine). This WILL hit the shared-socket-dir wall (each
   service gets its own RuntimeDirectory; there is no shared-dir mechanism yet): wire it in
   the demo with an extra raw `BindPaths=` property, prove the request flows browser→nginx→
   backend with BOTH services under `PrivateNetwork=yes`, and write the shared-dir design
   proposal this proves out.
4. **listen-fds probe** (`examples/dstyle/listenfds/`): a minimal genuinely socket-activated
   service (script reading `$LISTEN_FDS`, e.g. python `socket.fromfd`) behind a hand-made
   `.socket` unit — the pure-caps end state: service starts on first connection, zero network
   authority of its own. Record what `cix run`/spec must grow for native socket activation
   (transient socket units, `Requires=`/`Sockets=` wiring, spec surface: does a port entry
   grow `"activation": "socket"`?).
5. **LOG.md final section — "Design proposals, ranked"**: every wall → one concrete proposal
   (schema sketch + mechanism), ordered by how much of D25/D27 it unlocks. This section is
   the track's real deliverable and feeds the compose design round.

## Done gate

Every example has a `demo.sh` (build, run, prove the wiring with real requests, show
PrivateNetwork/activation state, stop, cleanup-trap); all demos green under sudo twice; no
leftover units/sockets in either manager; committed; LOG complete.
