# Chapter 6: Compose

> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.
> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.
> Version **0.1.0**, commit `unknown`.
> **Do not edit** — re-run the test to regenerate.

You will connect two independently built services with a Unix edge and shared state, validate and diff their compose generation, and exercise the socket-activation primitive beneath named listeners. Afterwards, you will understand compose's resolve/build/activate lifecycle, unary `cix run`, rollback boundary, pod option, and journal namespace without mistaking rootless dry-runs for system activation.

## Named listeners are systemd sockets

A `LISTENER` does not let the process call `socket()` for that port. This canonical Cixfile imports the probe's runtime, copies the checked-in Python script, and declares `LISTENER http`; systemd owns the socket and passes file descriptor 3 to the process.

#### `listener-fixture/Cixfile`

```dockerfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE listener-demo
IMPORT ${pkgs.coreutils} ${pkgs.python3}
COPY listenfds.py /bin/listenfds
START listenfds
LISTENER http
```

#### `listener-fixture/listenfds.py`

```python
#!/usr/bin/env python3
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
```

```sh
$ cix build listener-fixture
{
  "listener-demo": "/nix/store/…-cix-item-listener-demo"
}
```

#### `cix-manifest.json`

```json
{
  "cixManifest": 0,
  "env": {
    "PATH": {
      "default": "bin"
    }
  },
  "listeners": {
    "http": {
      "type": "stream"
    }
  },
  "mounts": [
    "/bin/[",
    "/bin/b2sum",
    "/bin/base32",
    "/bin/base64",
    "/bin/basename",
    "/bin/basenc",
    "/bin/cat",
    "/bin/chcon",
    "/bin/chgrp",
    "/bin/chmod",
    "/bin/chown",
    "/bin/chroot",
    "/bin/cksum",
    "/bin/comm",
    "/bin/coreutils",
    "/bin/cp",
    "/bin/csplit",
    "/bin/cut",
    "/bin/date",
    "/bin/dd",
    "/bin/df",
    "/bin/dir",
    "/bin/dircolors",
    "/bin/dirname",
    "/bin/du",
    "/bin/echo",
    "/bin/env",
    "/bin/expand",
    "/bin/expr",
    "/bin/factor",
    "/bin/false",
    "/bin/fmt",
    "/bin/fold",
    "/bin/groups",
    "/bin/head",
    "/bin/hostid",
    "/bin/id",
    "/bin/idle",
    "/bin/idle3",
    "/bin/idle3.14",
    "/bin/install",
    "/bin/join",
    "/bin/kill",
    "/bin/link",
    "/bin/listenfds",
    "/bin/ln",
    "/bin/logname",
    "/bin/ls",
    "/bin/md5sum",
    "/bin/mkdir",
    "/bin/mkfifo",
    "/bin/mknod",
    "/bin/mktemp",
    "/bin/mv",
    "/bin/nice",
    "/bin/nl",
    "/bin/nohup",
    "/bin/nproc",
    "/bin/numfmt",
    "/bin/od",
    "/bin/paste",
    "/bin/pathchk",
    "/bin/pinky",
    "/bin/pr",
    "/bin/printenv",
    "/bin/printf",
    "/bin/ptx",
    "/bin/pwd",
    "/bin/pydoc",
    "/bin/pydoc3",
    "/bin/pydoc3.14",
    "/bin/python",
    "/bin/python-config",
    "/bin/python3",
    "/bin/python3-config",
    "/bin/python3.14",
    "/bin/python3.14-config",
    "/bin/readlink",
    "/bin/realpath",
    "/bin/rm",
    "/bin/rmdir",
    "/bin/runcon",
    "/bin/seq",
    "/bin/sha1sum",
    "/bin/sha224sum",
    "/bin/sha256sum",
    "/bin/sha384sum",
    "/bin/sha512sum",
    "/bin/shred",
    "/bin/shuf",
    "/bin/sleep",
    "/bin/sort",
    "/bin/split",
    "/bin/stat",
    "/bin/stdbuf",
    "/bin/stty",
    "/bin/sum",
    "/bin/sync",
    "/bin/tac",
    "/bin/tail",
    "/bin/tee",
    "/bin/test",
    "/bin/timeout",
    "/bin/touch",
    "/bin/tr",
    "/bin/true",
    "/bin/truncate",
    "/bin/tsort",
    "/bin/tty",
    "/bin/uname",
    "/bin/unexpand",
    "/bin/uniq",
    "/bin/unlink",
    "/bin/uptime",
    "/bin/users",
    "/bin/vdir",
    "/bin/wc",
    "/bin/who",
    "/bin/whoami",
    "/bin/yes",
    "/share/gdb",
    "/share/man"
  ],
  "start": [
    "bin/listenfds"
  ]
}
```

```sh
$ cix run /nix/store/…-cix-item-listener-demo --user -p http=127.0.0.1:8420 --detach
cix-run-listener-demo-NONCE.service
warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path
```

```sh
$ curl -fsS http://127.0.0.1:8420
LISTEN_FDS=1; no socket() authority
```

```sh
$ systemctl --user stop cix-run-listener-demo-NONCE.service
```

## Two items, one operator document

#### `producer/Cixfile`

```dockerfile
FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE producer
IMPORT ${pkgs.coreutils}
START sleep 300
ENV VERSION = v1
STATEDIR /var/lib/shared
RUNDIR /run/producer
```

#### `consumer/Cixfile`

```dockerfile
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

#### `compose.json`

```json
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

#### `cix.lock`

```
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
