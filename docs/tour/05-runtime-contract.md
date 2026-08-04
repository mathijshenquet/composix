# Chapter 5: Running: the runtime contract

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

You will run and inspect a tagged HTTP service with health contracts, then schedule a run-to-completion app. Afterwards, you will understand the runtime boundary—immutable world, declared writable state, credential files, health supervision, timers, and journald/accounting observability—including which guarantees require the system-manager VM gate.

## The item owns needs; the operator owns values

The service declares a direct port, persistent application-native state, one credential-file need, and real HTTP readiness and liveness endpoints. The APP beside it has a finite entrypoint and is therefore eligible for timer scheduling.

```sh
$ cat Cixfile server.py
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

SERVICE web
IMPORT ${pkgs.python3}
START python3 ${src}/server.py
PORT http = 18086
STATEDIR /var/lib/runtime-guide
SECRET db-password AS DB_PASSWORD_FILE
READINESS http :18086/healthz IN 10s
LIVENESS http :18086/livez EVERY 2s

APP cleanup
IMPORT ${pkgs.coreutils}
START true
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
{"cleanup":"/nix/store/…-cix-item-cleanup","web":"/nix/store/…-cix-item-web"}
```

## Run by tag, then debug the same contract

`cix run` resolves the tag and compiles the manifest into a transient unit. This host's user manager takes the loud D13 fallback, but the readiness adapter still holds startup until the real HTTP endpoint answers and the liveness adapter continues probing it.

```sh
$ cix run runtime/web:v1 --user --detach
cix-run-web-NONCE.service
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path
```

```sh
$ curl -fsS http://127.0.0.1:8420/healthz
runtime healthy
```

```sh
$ cix debug runtime/web:v1 --user -- python3 -c 'print("debug uses the item package union")'
debug uses the item package union
warning: cix debug --user is degraded development mode; it does not provide the full system-manager sandbox or DynamicUser identity
=== cix debug: degraded service sandbox; service=web; identity=caller (--user) ===
```

```sh
$ cix ps
MANAGER  COMPOSITE  SERVICE  UNIT                                          STATE       RESULT   DESCRIPTION
user     -          web      cix-run-web-NONCE.service  active/running  success  /nix/store/…-cix-item-web/bin/python3 /nix/store/…-cix-source/server.py
```

```sh
$ cix stats 2>/dev/null | head -n 1
MANAGER  COMPOSITE  SERVICE  MEMORY  CPU  TASKS  IO  IP
```

```sh
$ cix logs run/web --explain
journalctl CIX_COMPOSITE=run CIX_SERVICE=web
```

## The system-manager guarantees

The ordinary production path runs in a read-only world: in `--closed-root` audit mode even undeclared host paths are absent, while the whole Nix store and the item's projections remain read-only. Only declared role directories are writable. This host cannot demonstrate that honestly because its user manager rejects the required mount namespace; the [closed-root audit scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/closedroot-audit.nix) executes the failed undeclared access and sealed-root inventory under the system manager.

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

```sh
$ systemctl --user stop cix-run-web-NONCE.service
```

You now have the complete ownership split: artifacts declare their process needs, compose supplies host policy and secrets, and systemd owns lifecycle, health, logs, timers, and accounting.


---

[← Previous](04-naming-distribution.html) · [Tour index](index.html) · [Next →](06-compose.html)
