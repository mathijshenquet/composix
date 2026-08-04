# Chapter 6: Compose

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

You will connect two independently built services with a Unix edge and shared state, validate and diff their compose generation, and exercise the socket-activation primitive beneath named listeners. Afterwards, you will understand compose's resolve/build/activate lifecycle, unary `cix run`, rollback boundary, pod option, and journal namespace without mistaking rootless dry-runs for system activation.

## Named listeners are systemd sockets

A `LISTENER` does not let the process call `socket()` for that port. Systemd owns the socket and passes file descriptor 3; this real fixture checks `LISTEN_FDS` and serves one HTTP response from the inherited descriptor.

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
  "cixManifest": 0,
  "start": [
    "bin/listenfds"
  ],
  "listeners": {
    "http": {
      "type": "stream"
    }
  }
}
```

```sh
$ cix run /nix/store/…-listener-fixture --user -p http=127.0.0.1:8420 --detach
cix-run-listener-fixture-NONCE.service
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path
```

```sh
$ curl -fsS http://127.0.0.1:8420
LISTEN_FDS=1; no socket() authority
```

```sh
$ systemctl --user stop cix-run-listener-fixture-NONCE.service
```

## Two items, one operator document

```sh
$ cat producer/Cixfile consumer/Cixfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE producer
IMPORT ${pkgs.coreutils}
START sleep 300
ENV VERSION = v1
STATEDIR /var/lib/shared
RUNDIR /run/producer
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE consumer
IMPORT ${pkgs.coreutils}
START sleep 300
ENV VERSION = v1
STATEDIR /var/lib/shared
```

```sh
$ cix build producer -t current
{
  "producer": "/nix/store/…-cix-item-producer"
}
```

```sh
$ cix build consumer -t v1
{
  "consumer": "/nix/store/…-cix-item-consumer"
}
```

The compose file owns host policy rather than rebuilding either item. Both members opt the same declared STATEDIR into compose-local shared backing, while the edge projects the producer's `/run/producer` Unix surface into the consumer and orders startup structurally.

```sh
$ cat compose.json
{
  "cixCompose": 1,
  "name": "tour-stack",
  "logNamespace": true,
  "children": {
    "producer": {
      "item": "producer:current",
      "update": "track",
      "dirs": {"/var/lib/shared": {"shared": "payload"}}
    },
    "consumer": {
      "item": "consumer:v1",
      "dirs": {"/var/lib/shared": {"shared": "payload"}}
    }
  },
  "edges": {
    "producer-api": {
      "producer": {"child": "producer", "path": "/run/producer"},
      "consumers": {"consumer": {}}
    }
  }
}
```

```sh
$ cix compose check compose.json
compose tour-stack: 2 services, 1 edges, valid
```

```sh
$ cat cix.lock
{
  "paths": {
    "consumer": {
      "narHash": "sha256-7iODRRqFnnKMdPDks8KJXgFJ4wAa7xazu8UBWnSBDvg=",
      "ref": "consumer:v1",
      "storePath": "/nix/store/…-cix-item-consumer"
    },
    "producer": {
      "narHash": "sha256-B5dRnQ8cJfzVmdHg2mLRdFwGgKJXw8EVgVxY/WlA+8Q=",
      "ref": "producer:current",
      "storePath": "/nix/store/…-cix-item-producer"
    }
  }
}
```

```sh
$ cix compose diff compose.json
unit added: cix-tour-stack-consumer.service
unit added: cix-tour-stack-edge-producer\x2dapi.service
unit added: cix-tour-stack-producer.service
unit added: cix-tour-stack-shared-payload.service
unit added: cix-tour-stack.slice
unit added: cix-tour-stack.target
service consumer: - -> /nix/store/…-cix-item-consumer
service producer: - -> /nix/store/…-cix-item-producer
```

`cix run` is the unary form of the same contract compiler. It gives one item a transient lifecycle; compose adds stable names, edges, shared backing, operator values, and retained generations.

```sh
$ cix run producer:current --user --detach
cix-run-producer-NONCE.service
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path
```

```sh
$ systemctl --user stop cix-run-producer-NONCE.service
```

Change only the tracked producer item. The dry diff resolves its moved tag and builds a candidate generation without touching the active system manager.

```sh
$ sed -i 's/ENV VERSION = v1/ENV VERSION = v2/' producer/Cixfile
```

```sh
$ cix build producer -t current
{
  "producer": "/nix/store/…-cix-item-producer"
}
```

```sh
$ cix compose diff compose.json
unit added: cix-tour-stack-consumer.service
unit added: cix-tour-stack-edge-producer\x2dapi.service
unit added: cix-tour-stack-producer.service
unit added: cix-tour-stack-shared-payload.service
unit added: cix-tour-stack.slice
unit added: cix-tour-stack.target
service consumer: - -> /nix/store/…-cix-item-consumer
service producer: - -> /nix/store/…-cix-item-producer
```

## Activation is the privileged receipt

This harness intentionally stops at `check` and `diff`: `cix up compose.json`, `cix rollback tour-stack`, and `cix down tour-stack` manage `/etc/systemd/system`, a root profile, shared backing ownership, and the system manager. The [stack VM scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/lib.nix) executes that exact up → selective change → diff → rollback → down lifecycle, and [the dirs scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/dirs2.nix) asserts both writers see the same setgid shared directory.

`network: "pod"` places a subtree in one private network namespace; named networks and service-DNS policy stay separate concerns. The [network-namespace scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/netns.nix) proves pod co-location, isolation, publication, and cleanup. `logNamespace: true` similarly asks systemd for one journal namespace for this compose tree; `cix logs tour-stack[/child]` selects its stamped fields, with the [observability scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/observability.nix) carrying the privileged receipt.


---

[← Previous](05-runtime-contract.html) · [Tour index](index.html) · [Next →](07-dev-loop-docker.html)
