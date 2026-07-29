# Track: compose — compose v0 (D9, D28, D30)

The biggest track so far. Contract, in order: `docs/design.md` D30 (exact IN/OUT scope — do
not build anything on the OUT list), D28 (canonical compose.json), D9 (mechanism), the dstyle
edge mechanism (`examples/dstyle/LOG.md` proposal #1 — follow it, it is live-proven), and the
unit-generator library API just landed in cix-run (specv3). Where ambiguous, choose boring
and note it in LOG.

## Ground rules

- Work log: `crates/cix-compose/LOG.md`. Territory: `crates/cix-compose/` (new),
  `crates/cix/src/main.rs` + Cargo.toml wiring, `examples/compose/` (new). Nothing else —
  needed cix-run/cix-index functionality is consumed via their library APIs; if an API is
  missing, the boring extension goes in a clearly-marked minimal commit.
- `cix up/down/rollback` are root operations (system manager, `/etc/systemd/system`, root
  profiles) — clear error otherwise. Sudo available; ALWAYS clean up: every demo/test ends
  with the composite down, links removed, both managers free of `cix-*` units/sockets.
- COMMIT AS YOU GO; clean status at the end.

## The data model

`compose.json` (composeVersion 1), strictly validated (unknown keys rejected, errors carry
JSON paths):

```json
{
  "composeVersion": 1,
  "name": "stack",
  "services": {
    "web":     { "item": "stack-web:v1", "bind": { "http": "127.0.0.1:8080" } },
    "backend": { "item": "stack-backend:v1", "update": "track",
                 "env": { "GREETING": "hello" } },
    "db":      { "item": "stack-db:v1" }
  },
  "edges": {
    "database": { "producer": { "service": "db", "path": "/run/postgresql" },
                  "consumers": { "backend": {} } },
    "http":     { "producer": { "service": "backend", "path": "/run/backend" },
                  "consumers": { "web": {} } }
  }
}
```

- `item`: bare ref (local tag), qualified ref (resolved via the index HTTP API — reuse
  cix-index), or `/nix/store/...` path. `service` key selects within multi-service items
  (optional when unambiguous). `update`: `"pin"` (default; resolved once into the lock,
  re-resolved only by `cix up --update [svc]`) or `"track"` (re-resolved every `up`).
- `env` overrides validate against the item's declared env (D21 rules, `-e` semantics).
  `bind` targets declared v3 `listeners`. Edge producer `path` must be one of the producer's
  declared `dirs.run`; consumer path defaults to the producer path.
- Semantic checks (`cix compose check`): unknown service refs in edges, env overrides on
  undeclared vars, missing required env without override, host-port collisions between
  services' effective `ports` values (v0 is host networking — two services claiming 8080 is
  an error), bind address collisions, items whose spec fails validation.

`cix.lock` next to compose.json: per service `{ref, storePath, narHash}` for the current
system; created/updated by resolution; committed by the user.

## Mechanism (per D9)

1. **Resolve** refs → lock (respecting pin/track and `--update`).
2. **Build the generation**: compile every service via the cix-run generator API with naming
   `cix-<name>-<svc>` in slice `cix-<name>.slice`, plus per-edge owner units
   (`cix-<name>-edge-<edge>.service`, RuntimeDirectory + per-edge group + mode 2770, held
   active under the target; group provisioning via a generated `sysusers.d` fragment applied
   with `systemd-sysusers` during activation — follow dstyle mechanics), consumer/producer
   `SupplementaryGroups=` + `BindPaths=` injection via the generator's extra-properties API,
   listener `.socket` units for every `bind`, and one `cix-<name>.target` wiring it all
   (`Wants=`/`After=` per edge dependencies). All units rendered as FILES into a generation
   dir (units/ + copies of compose.json + cix.lock + a manifest.json), then
   `nix store add-path` → the generation store item.
3. **Profile**: `nix-env -p /nix/var/nix/profiles/cix-compose-<name> --set <generation>` —
   generations give atomic upgrade + `cix rollback <name>` (`--rollback`, then re-activate).
4. **Activate**: link every unit file into `/etc/systemd/system/`, `daemon-reload`,
   `systemctl start cix-<name>.target`; restart exactly the services whose unit file content
   changed vs the previous generation; stop+unlink units that disappeared. `cix down`:
   stop target+units, unlink, daemon-reload (profile stays, `up` revives).
5. **`cix compose diff`**: dry-build the new generation, report per unit added/removed/
   changed (with the store-item change per service old→new). No activation.
6. `cix ps` gains composite grouping (slice-based) showing composite, service, unit state.

## Example + demo (`examples/compose/stack/`)

Three small items (own Cixfiles): `db` (postgres, unix-socket-only — adapt the dstyle
postgres-unix shape), `backend` (tiny HTTP-over-unix-socket service that queries the db over
the edge socket and serves the result), `web` (nginx with a v3 `listeners.http`, proxying to
the backend's unix socket via the `http` edge). Both a hand-written `compose.json` AND an
equivalent `generate.py` (~40 lines, stdlib-only, emitting byte-identical compose.json —
demonstrating D28 config-as-code; a test asserts the equivalence).

`demo.sh` (sudo): build+tag the three items, `cix compose check`, `cix up`, curl
`127.0.0.1:8080` proving web→backend→db over two edges with the web service accepting only
inherited fds, `cix ps` showing the composite, then: retag `stack-backend:v2` (changed
greeting), `cix compose diff` (shows exactly one service changing), `cix up` (only backend
restarts — prove via `systemctl show -p ActiveEnterTimestamp` on the other units), curl shows
v2, `cix rollback stack` (curl shows v1 again), `cix down`, cleanliness audit.

## Tests

Schema/validation unit tests (every semantic check above, both orders); lock lifecycle
(pin vs track, --update); generation determinism (same inputs ⇒ same store path); unit-file
golden fixtures for a small composite (service + edge owner + socket + target); root-gated
integration test running the real stack demo flow (self-skipping pattern).

## Done gate

fmt/clippy/`cargo test --workspace` green ×2; demo green ×2 under sudo incl. rollback; VM
check green; no leftover units/sockets/links in either manager; committed; LOG final summary
with deviations + every wall (walls here are compose-v1 design input).
