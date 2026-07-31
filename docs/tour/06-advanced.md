# Chapter 6: Advanced

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

The basic chapters use ordinary port grants and single services. This chapter shows two places where composix deliberately exposes the underlying systemd and Nix shapes instead of hiding them.

## Socket activation

The fixture is not opaque: it contains an executable that consumes systemd file descriptor 3 and a v3 manifest declaring the named `http` listener.

```sh
$ ls -R listener-fixture
listener-fixture:
bin
cix-manifest.json

listener-fixture/bin:
listenfds
```

```sh
$ cat listener-fixture/bin/listenfds listener-fixture/cix-manifest.json
#!/usr/bin/python3
import os
import socket

listen_fds = int(os.environ.get("LISTEN_FDS", "0"))
listen_pid = int(os.environ.get("LISTEN_PID", "0"))
if listen_fds != 1 or listen_pid != os.getpid():
    raise SystemExit("expected one named systemd listener")

listener = socket.fromfd(3, socket.AF_INET, socket.SOCK_STREAM)
while True:
    connection, _ = listener.accept()
    with connection:
        connection.recv(4096)
        body = b"LISTEN_FDS=1; no socket() authority\n"
        connection.sendall(
            b"HTTP/1.1 200 OK\r\n"
            + b"Content-Type: text/plain\r\n"
            + b"Content-Length: " + str(len(body)).encode() + b"\r\n"
            + b"Connection: close\r\n\r\n" + body
        )
{
  "cixManifest": 3,
  "services": {
    "listenfds": {
      "exec": ["bin/listenfds"],
      "listeners": {"http": {"type": "stream"}}
    }
  }
}
```

```sh
$ cix run /nix/store/…-listener-fixture --user -p http=127.0.0.1:8420 --detach
cix-run-listenfds-NONCE.service
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path
```

```sh
$ curl -fsS http://127.0.0.1:8420
LISTEN_FDS=1; no socket() authority
```

```sh
$ systemctl --user stop cix-run-listenfds-NONCE.service
```

Stopping the transient service also removes its companion `.socket` unit.

## Compose

Compose now starts from a real Cixfile-built service rather than a harness-created store path. Its complete build input is visible before use.

```sh
$ ls -1 compose-app
Cixfile
Cixfile.lock
web
```

```sh
$ cat compose-app/Cixfile compose-app/web
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

SERVICE web
COPY ${src}/web bin/web
EXEC ${pkgs.bash}/bin/sh ${src}/web
echo compose fixture v1
```

```sh
$ cat compose-app/Cixfile.lock
{
  "inputs": {
    "pkgs": {
      "url": "github:NixOS/nixpkgs/nixos-unstable",
      "rev": "624af665418d3c65d544145b4d34ad696439570e",
      "narHash": "sha256-m0pDuRJG7EDo9ri+4Ksu83VsI+PlxNC9lNBfydejce4="
    }
  }
}
```

```sh
$ cix build compose-app -t tour-compose:current
/nix/store/…-cix-item-web
```

```sh
$ cat compose.json
{
  "composeVersion": 1,
  "name": "tour-compose",
  "services": {
    "web": {
      "item": "tour-compose:current",
      "update": "track"
    }
  }
}
```

```sh
$ cix compose check compose.json
compose tour-compose: 1 services, 0 edges, valid
```

`check` resolves and validates without activation. Root `cix up` owns the persistent lock write, so this rootless chapter records the checked tag's actual values before showing the lock and dry diff.

```sh
$ cat cix.lock
{
  "services": {
    "web": {
      "ref": "tour-compose:current",
      "storePath": "/nix/store/…-cix-item-web",
      "narHash": "sha256-x89hj7V519BkYOMWTa1vgZJ36cGgG5jiTmW3/jBmQAo="
    }
  }
}
```

```sh
$ cix compose diff compose.json
unit added: cix-tour-compose-web.service
unit added: cix-tour-compose.slice
unit added: cix-tour-compose.target
service web: - -> /nix/store/…-cix-item-web
```

Changing the copied script makes a new immutable item; rebuilding with the same tracked tag moves only the name.

```sh
$ cat compose-app/Cixfile compose-app/web
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

SERVICE web
COPY ${src}/web bin/web
EXEC ${pkgs.bash}/bin/sh ${src}/web
echo compose fixture v2
```

```sh
$ cix build compose-app -t tour-compose:current
/nix/store/…-cix-item-web
```

```sh
$ cix compose diff compose.json
unit added: cix-tour-compose-web.service
unit added: cix-tour-compose.slice
unit added: cix-tour-compose.target
service web: - -> /nix/store/…-cix-item-web
```

`cix up`, `cix rollback`, and `cix down` use the system manager and therefore require root. The [stack example](../../examples/compose/stack/) VM check covers activation, selective update, rollback, and cleanup.


---

[← Previous](05-proj1.html) · [Tour index](index.html)
