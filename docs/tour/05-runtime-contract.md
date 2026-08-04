# Chapter 5: Running: the runtime contract

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

You will inspect a tagged HTTP service with health contracts at the honest rootless boundary, then debug and schedule its run-to-completion sibling. Afterwards, you will understand the runtime boundary—immutable world, declared writable state, credential files, health supervision, timers, and journald/accounting observability—including which guarantees require the system-manager VM gate.

## The item owns needs; the operator owns values

The web service declares a direct port, persistent application-native state, one credential-file need, and real HTTP readiness and liveness endpoints. The finite APP is eligible for timer scheduling, while the minimal observer service stays alive long enough for scoped observability receipts.

```sh
$ cat Cixfile server.py
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE web
IMPORT ${pkgs.python3}
COPY server.py /srv/app/server.py
START python3 /srv/app/server.py
PORT http = 18086
STATEDIR /var/lib/runtime-guide
SECRET db-password AS DB_PASSWORD_FILE
READINESS http :18086/healthz IN 10s
LIVENESS http :18086/livez EVERY 2s

APP cleanup
IMPORT ${pkgs.coreutils}
START true

SERVICE observer
IMPORT ${pkgs.coreutils}
START sleep 300
from http.server import BaseHTTPRequestHandler, HTTPServer

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        body = b"runtime healthy\n"
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):
        pass

print("runtime service started", flush=True)
HTTPServer(("127.0.0.1", 18086), Handler).serve_forever()
```

```sh
$ cix build . --namespace runtime -t v1
{
  "cleanup": "/nix/store/…-cix-item-cleanup",
  "observer": "/nix/store/…-cix-item-observer",
  "web": "/nix/store/…-cix-item-web"
}
```

## Inspect the item, then cross the system-manager boundary

`cix run` resolves the tag and compiles the manifest into a transient unit. Production projects `/srv/app/server.py` from the item before readiness and liveness supervision begins. Because D13 permits a user manager to reject that mount namespace, the rootless receipt parses the copied program through its physical item path instead of claiming a live HTTP service.

```sh
$ /nix/store/…-cix-item-web/bin/python3 -c 'compile(open("/nix/store/…-cix-item-web/srv/app/server.py").read(), "server.py", "exec"); print("copied server parses")'
copied server parses
```

`cix debug` still resolves an item by tag and replaces its entrypoint inside the service sandbox. The finite cleanup sibling has no mount or health dependency, so it is the honest rootless target for that receipt.

```sh
$ cix debug runtime/cleanup:v1 --user -- true
warning: cix debug --user is degraded development mode; it does not provide the full system-manager sandbox or DynamicUser identity
=== cix debug: degraded service sandbox; service=cleanup; identity=caller (--user) ===
```

The observer sibling is deliberately small and long-running, so the observability receipts can assert one tour-owned unit. `ps --json` selects that exact unit instead of formatting an ambient table whose widths depend on unrelated units; the `stats` projection keeps the live counters live while asserting their stable manager, composite, and unit identity.

```sh
$ cix run runtime/observer:v1 --user --detach
cix-run-observer-NONCE.service
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path
```

```sh
$ cix ps --json | jq --arg unit 'cix-run-observer-NONCE.service' '.[] | select(.unit == $unit) | {manager, service, unit, state}'
{
  "manager": "user",
  "service": "observer",
  "unit": "cix-run-observer-NONCE.service",
  "state": "active/running"
}
```

```sh
$ cix stats 2>/dev/null | awk -v unit='cix-run-observer-NONCE.service' 'NR == 1 || $3 == unit'
MANAGER  COMPOSITE  SERVICE  MEMORY  CPU  TASKS  IO  IP
user  run  cix-run-observer-NONCE.service  <live>  <live>  <live>  <live>  <live>
```

```sh
$ cix logs run/observer --explain
journalctl CIX_COMPOSITE=run CIX_SERVICE=observer
```

## The system-manager guarantees

The ordinary production path runs in a read-only world: in `--closed-root` audit mode even undeclared host paths are absent, while the whole Nix store and the item's projections remain read-only. Only declared role directories are writable. The rootless contract does not guarantee that mount namespace, so the [closed-root audit scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/closedroot-audit.nix) executes the failed undeclared access and sealed-root inventory under the system manager.

`STATEDIR /var/lib/runtime-guide` survives service restarts and belongs to cix until an explicit purge; the item never chooses a host backing path. `SECRET db-password` similarly names no value: compose supplies a root-owned file, systemd projects it below `$CREDENTIALS_DIRECTORY`, and `DB_PASSWORD_FILE` receives only that path. The [directory lifecycle scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/dirs2.nix), [secrets scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/secrets.nix), and [health scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/health.nix) execute persistence, credential rotation, readiness blocking, and liveness restart without faking host privileges here.

## Schedule the APP

An APP runs to completion instead of staying active. `--schedule` writes a transient service/timer pair using systemd's `OnCalendar` syntax and prints the timer name; no polling daemon is involved.

```sh
$ cix run runtime/cleanup:v1 --user --schedule '*-*-* 00:00:00'
cix-run-cleanup-NONCE.timer
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path
```

```sh
$ systemctl --user is-active cix-run-cleanup-NONCE.timer
active
```

You now have the complete ownership split: artifacts declare their process needs, compose supplies host policy and secrets, and systemd owns lifecycle, health, logs, timers, and accounting.


---

[← Previous](04-naming-distribution.html) · [Tour index](index.html) · [Next →](06-compose.html)
